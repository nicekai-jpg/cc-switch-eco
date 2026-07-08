use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::AppError;

/// `settings.json` 中由 CC Switch 供应商切换写入、不应被 Eco fragment 重建覆盖的顶层字段。
pub const CLAUDE_PROVIDER_MANAGED_SETTINGS_KEYS: &[&str] = &["env", "model"];

pub const CCS_PROVIDER_FRAGMENT_PREFIX: &str = "ccs-";

/// 解析 JSON 字符串，失败时返回带上下文的错误
pub fn parse_json(content: &str, context: &str) -> Result<serde_json::Value, AppError> {
    serde_json::from_str(content)
        .map_err(|e| AppError::Message(format!("{context}: {e}")))
}

/// 序列化 JSON 值为格式化字符串
pub fn write_json(value: &serde_json::Value) -> Result<String, AppError> {
    serde_json::to_string_pretty(value)
        .map_err(|e| AppError::JsonSerialize { source: e })
}

/// 计算 fragment 文件路径
///
/// 命名规则：`<basename>.<prefix>fragment.json`
/// 例如 `settings.omc-fragment.json`、`settings.user-fragment.json`
pub fn fragment_path(rootfiles_dir: &Path, file_name: &str, prefix: &str) -> std::path::PathBuf {
    let stem = file_name.strip_suffix(".json").unwrap_or(file_name);
    rootfiles_dir.join(format!("{stem}.{prefix}fragment.json"))
}

/// 列出某个根文件的所有 fragment 文件（不含 user-fragment）
pub fn list_fragments(rootfiles_dir: &Path, file_name: &str) -> Vec<std::path::PathBuf> {
    let stem = file_name.strip_suffix(".json").unwrap_or(file_name);
    let suffix = "fragment.json";

    let Ok(entries) = fs::read_dir(rootfiles_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with(&format!("{stem}."))
                && name.ends_with(suffix)
                && !name.contains(".user-")
        })
        .map(|e| e.path())
        .collect()
}

/// 深合并 JSON：对象递归合并，数组去重拼接，标量后写优先
///
/// 标量冲突会记录到 `conflicts` 向量，格式：`key_path: old → new (被 prefix 覆盖)`
pub fn json_deep_merge_with_array_dedup(
    target: &mut serde_json::Value,
    source: &serde_json::Value,
    path: &str,
    prefix: &str,
    conflicts: &mut Vec<String>,
) {
    match (target, source) {
        (serde_json::Value::Object(t), serde_json::Value::Object(s)) => {
            for (key, sv) in s {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match t.get_mut(key) {
                    Some(tv) => {
                        json_deep_merge_with_array_dedup(tv, sv, &child_path, prefix, conflicts)
                    }
                    None => {
                        t.insert(key.clone(), sv.clone());
                    }
                };
            }
        }
        (serde_json::Value::Array(t), serde_json::Value::Array(s)) => {
            for item in s {
                if !t.contains(item) {
                    t.push(item.clone());
                }
            }
        }
        (tv, sv) => {
            if tv != sv {
                conflicts.push(format!("{}: {} → {} (被 {} 覆盖)", path, tv, sv, prefix));
            }
            *tv = sv.clone();
        }
    }
}

/// 将 live `settings.json` 中由供应商切换写入的字段合并到重建结果，避免被 Eco fragment 覆盖。
pub fn preserve_claude_provider_fields(target: &mut Value, source: &Value) {
    let Some(source_obj) = source.as_object() else {
        return;
    };
    let Some(target_obj) = target.as_object_mut() else {
        return;
    };

    for key in CLAUDE_PROVIDER_MANAGED_SETTINGS_KEYS {
        if let Some(value) = source_obj.get(*key) {
            target_obj.insert(key.to_string(), value.clone());
        }
    }
}

/// 从所有 fragment 重建一个 JSON 根文件
///
/// 合并顺序：框架 fragment（按 eco.json 中 frameworks 安装顺序）→ ccs-fragment（供应商 env/model）→ user-fragment（始终最后）
/// 用户偏好优先：user-fragment 中的标量值会覆盖框架的值，且不记录为冲突。
pub use super::fragment_rebuild::{rebuild_all_root_files, rebuild_root_file};
pub use super::fragment_pref::{save_user_preferences, remove_user_preference, snapshot_user_preferences};

/// 合并根文件（安装框架时调用）
///
/// - CLAUDE.md：追加合并，用 `<!-- prefix -->` 标记分隔
/// - JSON 文件：保存为 fragment，由调用方统一重建
/// - 其他文件：直接覆盖
pub fn merge_root_file(src: &Path, dst: &Path, prefix: &str) -> Result<(), AppError> {
    let file_name = dst
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if file_name == "CLAUDE.md" {
        // CLAUDE.md：追加合并
        let existing = if dst.exists() {
            fs::read_to_string(dst).map_err(|e| AppError::io(dst, e))?
        } else {
            String::new()
        };

        let new_content = fs::read_to_string(src).map_err(|e| AppError::io(src, e))?;

        let merged = if existing.is_empty() {
            format!("<!-- {prefix} -->\n{new_content}")
        } else {
            format!("{existing}\n\n<!-- {prefix} -->\n{new_content}")
        };

        fs::write(dst, merged).map_err(|e| AppError::io(dst, e))?;
    } else if file_name.ends_with(".json") {
        // JSON 文件：保存为 fragment
        let rootfiles_dir = dst
            .parent()
            .ok_or_else(|| AppError::Message(format!("路径无父目录: {}", dst.display())))?;
        let frag_path = fragment_path(rootfiles_dir, &file_name, prefix);
        fs::copy(src, &frag_path).map_err(|e| AppError::io(&frag_path, e))?;
    } else {
        // 其他文件：直接覆盖
        fs::copy(src, dst).map_err(|e| AppError::io(dst, e))?;
    }

    Ok(())
}

/// 从根文件中移除框架贡献的内容（卸载框架时调用）
///
/// - CLAUDE.md：移除 `<!-- prefix -->` 标记段
/// - JSON 文件：删除 fragment，由调用方统一重建
/// - 其他文件：删除
pub fn remove_framework_from_rootfile(file_path: &Path, prefix: &str) -> Result<(), AppError> {
    let file_name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if file_name == "CLAUDE.md" {
        // CLAUDE.md：移除标记段
        let content = fs::read_to_string(file_path).map_err(|e| AppError::io(file_path, e))?;
        let marker = format!("<!-- {prefix} -->");
        let mut result = String::new();
        let mut skip = false;

        for line in content.lines() {
            if line.trim() == marker {
                skip = true;
                continue;
            }
            if skip && line.trim().is_empty() {
                skip = false;
                continue;
            }
            if !skip {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(line);
            }
        }

        fs::write(file_path, result).map_err(|e| AppError::io(file_path, e))?;
    } else if file_name.ends_with(".json") {
        // JSON 文件：删除 fragment
        let rootfiles_dir = file_path
            .parent()
            .ok_or_else(|| AppError::Message(format!("路径无父目录: {}", file_path.display())))?;
        let frag_path = fragment_path(rootfiles_dir, &file_name, prefix);
        if frag_path.exists() {
            fs::remove_file(&frag_path).map_err(|e| AppError::io(&frag_path, e))?;
        }
    } else {
        // 其他文件：删除
        if file_path.exists() {
            fs::remove_file(file_path).map_err(|e| AppError::io(file_path, e))?;
        }
    }

    Ok(())
}

pub use super::fragment_isolation::{collect_eco_isolation, collect_framework_isolation, update_eco_json_isolation, sanitize_hooks_for_global_settings, EcoIsolation};

/// 从 JSON 值中提取字符串数组
pub fn extract_string_array(json: &serde_json::Value, key: &str) -> Vec<String> {
    json.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_deep_merge_objects() {
        let mut target = json!({"a": 1, "b": 2});
        let source = json!({"b": 3, "c": 4});
        let mut conflicts = Vec::new();
        json_deep_merge_with_array_dedup(&mut target, &source, "", "test", &mut conflicts);
        assert_eq!(target, json!({"a": 1, "b": 3, "c": 4}));
        assert_eq!(conflicts, vec!["b: 2 → 3 (被 test 覆盖)"]);
    }

    #[test]
    fn test_json_deep_merge_arrays_dedup() {
        let mut target = json!({"items": [1, 2, 3]});
        let source = json!({"items": [3, 4, 5]});
        let mut conflicts = Vec::new();
        json_deep_merge_with_array_dedup(&mut target, &source, "", "test", &mut conflicts);
        assert_eq!(target, json!({"items": [1, 2, 3, 4, 5]}));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_json_deep_merge_nested() {
        let mut target = json!({
            "permissions": {"allow": ["Bash", "Read"], "deny": []}
        });
        let source = json!({
            "permissions": {"allow": ["Bash", "Write", "Edit"], "deny": ["WebFetch"]}
        });
        let mut conflicts = Vec::new();
        json_deep_merge_with_array_dedup(&mut target, &source, "", "fw2", &mut conflicts);
        // permissions 是对象 → 递归合并
        // allow 是数组 → 去重拼接
        // deny 是数组 → 去重拼接
        let allow = target["permissions"]["allow"].as_array().unwrap();
        assert!(allow.iter().any(|v| v.as_str() == Some("Bash")));
        assert!(allow.iter().any(|v| v.as_str() == Some("Read")));
        assert!(allow.iter().any(|v| v.as_str() == Some("Write")));
        assert!(allow.iter().any(|v| v.as_str() == Some("Edit")));
        assert_eq!(allow.len(), 4);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_user_fragment_overrides_but_conflicts_still_recorded() {
        // json_deep_merge_with_array_dedup 本身会记录所有标量冲突
        // rebuild_root_file 通过传入 &mut Vec::new() 来丢弃用户覆盖的冲突
        let mut merged = json!({
            "defaultMode": "plan",
            "effort": "max",
            "language": "English"
        });
        let user = json!({
            "defaultMode": "bypassPermissions",
            "language": "中文"
        });
        let mut user_conflicts = Vec::new();
        json_deep_merge_with_array_dedup(&mut merged, &user, "", "user-", &mut user_conflicts);
        assert_eq!(merged["defaultMode"], "bypassPermissions");
        assert_eq!(merged["effort"], "max");
        assert_eq!(merged["language"], "中文");
        // 函数本身会记录冲突，但 rebuild_root_file 传入空 Vec 丢弃
        assert_eq!(user_conflicts.len(), 2);
    }

    #[test]
    fn test_framework_scalar_conflicts_recorded() {
        let mut merged = json!({"defaultMode": "bypassPermissions", "effort": "high"});
        let fw2 = json!({"defaultMode": "plan", "effort": "max"});
        let mut conflicts = Vec::new();
        json_deep_merge_with_array_dedup(&mut merged, &fw2, "", "fw2", &mut conflicts);
        assert_eq!(merged["defaultMode"], "plan");
        assert_eq!(merged["effort"], "max");
        assert_eq!(conflicts.len(), 2);
    }

    #[test]
    fn preserve_claude_provider_fields_keeps_env_and_model() {
        let mut target = json!({
            "hooks": {},
            "enabledPlugins": {"claude-hud@claude-hud": true}
        });
        let source = json!({
            "env": {"ANTHROPIC_MODEL": "kimi-k2"},
            "model": "kimi-k2",
            "hooks": {"PreCompact": []}
        });

        super::preserve_claude_provider_fields(&mut target, &source);

        assert_eq!(target["env"]["ANTHROPIC_MODEL"], "kimi-k2");
        assert_eq!(target["model"], "kimi-k2");
        assert_eq!(target["enabledPlugins"]["claude-hud@claude-hud"], true);
    }

    #[test]
    fn test_fragment_path_naming() {
        let dir = Path::new("/tmp/test");
        assert_eq!(
            fragment_path(dir, "settings.json", "omc-"),
            dir.join("settings.omc-fragment.json")
        );
        assert_eq!(
            fragment_path(dir, "settings.json", "user-"),
            dir.join("settings.user-fragment.json")
        );
    }

    #[test]
    fn test_rebuild_root_file_with_user_priority() {
        let tmp = tempfile::tempdir().unwrap();
        let rootfiles_dir = tmp.path().join("rootfiles");
        fs::create_dir_all(&rootfiles_dir).unwrap();

        // rebuild_root_file 通过 find_framework 获取 file_prefix
        // find_framework("ohmyclaudecode") → prefix "omc-"
        // find_framework("ruflo") → prefix "ruflo-"
        // 所以 fragment 文件名需要用这些 prefix

        // omc fragment (prefix = "omc-")
        let omc_frag = json!({
            "defaultMode": "bypassPermissions",
            "effort": "high",
            "language": "English",
            "permissions": {"allow": ["Bash", "Read"], "deny": []}
        });
        fs::write(
            fragment_path(&rootfiles_dir, "settings.json", "omc-"),
            serde_json::to_string_pretty(&omc_frag).unwrap(),
        )
        .unwrap();

        // ruflo fragment (prefix = "ruflo-")
        let ruflo_frag = json!({
            "defaultMode": "plan",
            "effort": "max",
            "language": "中文",
            "permissions": {"allow": ["Bash", "Write", "Edit"], "deny": ["WebFetch"]}
        });
        fs::write(
            fragment_path(&rootfiles_dir, "settings.json", "ruflo-"),
            serde_json::to_string_pretty(&ruflo_frag).unwrap(),
        )
        .unwrap();

        // user-fragment
        let user_frag = json!({
            "defaultMode": "bypassPermissions",
            "language": "中文"
        });
        fs::write(
            fragment_path(&rootfiles_dir, "settings.json", "user-"),
            serde_json::to_string_pretty(&user_frag).unwrap(),
        )
        .unwrap();

        // 创建 eco.json（需要 parent 目录）
        let eco_dir = tmp.path();
        let eco_json =
            json!({"frameworks": ["ohmyclaudecode", "ruflo"], "isolatedFiles": ["settings.json"]});
        fs::write(
            eco_dir.join("eco.json"),
            serde_json::to_string_pretty(&eco_json).unwrap(),
        )
        .unwrap();

        // 重建 — 使用正确的框架 ID
        rebuild_root_file(
            &rootfiles_dir,
            "settings.json",
            &["ohmyclaudecode".to_string(), "ruflo".to_string()],
        )
        .unwrap();

        // 验证结果
        let result: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(rootfiles_dir.join("settings.json")).unwrap())
                .unwrap();

        println!(
            "合并结果: {}",
            serde_json::to_string_pretty(&result).unwrap()
        );

        // 用户偏好优先
        assert_eq!(result["defaultMode"], "bypassPermissions");
        assert_eq!(result["language"], "中文");
        // ruflo 最后覆盖框架值
        assert_eq!(result["effort"], "max");
        // 数组去重合并
        let allow = result["permissions"]["allow"].as_array().unwrap();
        let allow_strs: Vec<&str> = allow.iter().filter_map(|v| v.as_str()).collect();
        assert!(allow_strs.contains(&"Bash"), "Bash missing from allow");
        assert!(allow_strs.contains(&"Read"), "Read missing from allow");
        assert!(allow_strs.contains(&"Write"), "Write missing from allow");
        assert!(allow_strs.contains(&"Edit"), "Edit missing from allow");
        // deny
        assert_eq!(result["permissions"]["deny"], json!(["WebFetch"]));
    }

    #[test]
    fn test_list_fragments_excludes_user() {
        let tmp = tempfile::tempdir().unwrap();
        let rootfiles_dir = tmp.path();
        fs::write(rootfiles_dir.join("settings.omc-fragment.json"), "{}").unwrap();
        fs::write(rootfiles_dir.join("settings.ruflo-fragment.json"), "{}").unwrap();
        fs::write(rootfiles_dir.join("settings.user-fragment.json"), "{}").unwrap();
        fs::write(rootfiles_dir.join("other.json"), "{}").unwrap();

        let frags = list_fragments(rootfiles_dir, "settings.json");
        let names: Vec<String> = frags
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"settings.omc-fragment.json".to_string()));
        assert!(names.contains(&"settings.ruflo-fragment.json".to_string()));
        assert!(!names.contains(&"settings.user-fragment.json".to_string()));
        assert!(!names.contains(&"other.json".to_string()));
    }

    #[test]
    fn test_pua_plugin_root_hooks_stripped_from_global_settings() {
        // PUA v3 hooks 引用 ${CLAUDE_PLUGIN_ROOT}，写入全局 settings 后会被剥离。
        // 因此 PUA 必须走 plugin 安装，不能仅用 skills CLI + settings fragment。
        let mut hooks = json!({
            "SessionStart": [{"hooks": [{"type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/hooks/session-restore.sh\""}]}],
            "PostToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/hooks/failure-detector.sh\""}]}],
            "PreCompact": [{"matcher": "*", "hooks": [{"type": "prompt", "prompt": "[PUA PreCompact]"}]}]
        })
        .as_object()
        .unwrap()
        .clone();

        sanitize_hooks_for_global_settings(&mut hooks);

        assert!(
            hooks.get("SessionStart").is_none(),
            "SessionStart command hooks 应被剥离"
        );
        assert!(
            hooks.get("PostToolUse").is_none(),
            "PostToolUse command hooks 应被剥离"
        );
        assert!(
            hooks.get("PreCompact").is_some(),
            "不含 ${{CLAUDE_PLUGIN_ROOT}} 的 prompt hook 应保留"
        );
    }
}
