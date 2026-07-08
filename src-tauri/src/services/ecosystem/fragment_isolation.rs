use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem_framework;
use super::fragment::{parse_json, write_json, extract_string_array};

/// Eco 隔离信息（目录 + 文件）
pub struct EcoIsolation {
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}

/// 从 eco.json 收集隔离配置
pub fn collect_eco_isolation(eco_dir: &Path) -> EcoIsolation {
    let eco_json_path = eco_dir.join("eco.json");
    if !eco_json_path.exists() {
        return EcoIsolation {
            dirs: Vec::new(),
            files: Vec::new(),
        };
    }

    let content = fs::read_to_string(&eco_json_path).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();

    let dirs = extract_string_array(&json, "isolatedDirs");
    let files = extract_string_array(&json, "isolatedFiles");

    EcoIsolation { dirs, files }
}

/// 收集所有框架的隔离配置
pub fn collect_framework_isolation(framework_ids: &[String]) -> EcoIsolation {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for fw_id in framework_ids {
        if let Some(fw) = ecosystem_framework::find_framework(fw_id) {
            for dir in &fw.isolated_dirs {
                if !dirs.contains(dir) {
                    dirs.push(dir.clone());
                }
            }
            for file in &fw.isolated_files {
                if !files.contains(file) {
                    files.push(file.clone());
                }
            }
        }
    }

    EcoIsolation { dirs, files }
}

/// 更新 eco.json 的隔离配置
pub fn update_eco_json_isolation(eco_dir: &Path, isolation: &EcoIsolation) -> Result<(), AppError> {
    let eco_json_path = eco_dir.join("eco.json");
    if !eco_json_path.exists() {
        return Ok(());
    }

    let content =
        fs::read_to_string(&eco_json_path).map_err(|e| AppError::io(&eco_json_path, e))?;
    let mut json: serde_json::Value = parse_json(&content, "解析 eco.json 失败")?;

    if let Some(obj) = json.as_object_mut() {
        obj.insert(
            "isolatedDirs".to_string(),
            serde_json::Value::Array(
                isolation
                    .dirs
                    .iter()
                    .map(|d| serde_json::Value::String(d.clone()))
                    .collect(),
            ),
        );
        obj.insert(
            "isolatedFiles".to_string(),
            serde_json::Value::Array(
                isolation
                    .files
                    .iter()
                    .map(|f| serde_json::Value::String(f.clone()))
                    .collect(),
            ),
        );
    }

    let content = write_json(&json)?;
    fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

    Ok(())
}

/// 清理 global hooks 字段，过滤掉包含 ${CLAUDE_PLUGIN_ROOT} 的命令，
/// 这种命令只应由 plugin 独立 hooks 执行，不能放入全局 settings.json。
pub fn sanitize_hooks_for_global_settings(hooks: &mut serde_json::Map<String, serde_json::Value>) {
    for (_hook_event, event_hooks_val) in hooks.iter_mut() {
        if let Some(groups_arr) = event_hooks_val.as_array_mut() {
            for group in groups_arr.iter_mut() {
                if let Some(group_obj) = group.as_object_mut() {
                    if let Some(hooks_list) = group_obj.get_mut("hooks") {
                        if let Some(hooks_list_arr) = hooks_list.as_array_mut() {
                            hooks_list_arr.retain(|hook| {
                                if let Some(hook_obj) = hook.as_object() {
                                    if let Some(cmd) = hook_obj.get("command").and_then(|c| c.as_str()) {
                                        if cmd.contains("${CLAUDE_PLUGIN_ROOT}") || cmd.contains("$CLAUDE_PLUGIN_ROOT") {
                                            return false;
                                        }
                                    }
                                }
                                true
                            });
                        }
                    }
                }
            }
            groups_arr.retain(|group| {
                if let Some(group_obj) = group.as_object() {
                    if let Some(hooks_list) = group_obj.get("hooks").and_then(|h| h.as_array()) {
                        return !hooks_list.is_empty();
                    }
                }
                true
            });
        }
    }
    hooks.retain(|_hook_event, event_hooks_val| {
        if let Some(groups_arr) = event_hooks_val.as_array() {
            return !groups_arr.is_empty();
        }
        true
    });
}
