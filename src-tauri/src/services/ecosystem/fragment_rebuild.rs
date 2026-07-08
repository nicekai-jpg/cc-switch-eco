use std::fs;
use std::path::Path;
use serde_json::Value;

use crate::error::AppError;
use crate::services::ecosystem_framework;
use super::fragment::{
    fragment_path, write_json, parse_json, extract_string_array,
    preserve_claude_provider_fields, json_deep_merge_with_array_dedup,
    CCS_PROVIDER_FRAGMENT_PREFIX,
};
use super::fragment_isolation::sanitize_hooks_for_global_settings;

/// 从所有 fragment 重建一个 JSON 根文件
///
/// 合并顺序：框架 fragment（按 eco.json 中 frameworks 安装顺序）→ ccs-fragment（供应商 env/model）→ user-fragment（始终最后）
/// 用户偏好优先：user-fragment 中的标量值会覆盖框架的值，且不记录为冲突。
pub fn rebuild_root_file(
    rootfiles_dir: &Path,
    file_name: &str,
    framework_order: &[String],
) -> Result<(), AppError> {
    let mut merged = serde_json::json!({});
    let mut has_any_fragment = false;
    let mut all_conflicts: Vec<String> = Vec::new();

    // 按 framework 安装顺序读取 fragment 并合并
    for fw_id in framework_order {
        let fw = ecosystem_framework::find_framework(fw_id);
        let prefix: &str = fw
            .as_ref()
            .map_or(fw_id.as_str(), |f| f.file_prefix.as_str());
        let frag_path = fragment_path(rootfiles_dir, file_name, prefix);

        if !frag_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&frag_path).map_err(|e| AppError::io(&frag_path, e))?;
        let frag_json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("Fragment {} JSON 解析失败，跳过: {e}", frag_path.display());
                continue;
            }
        };

        json_deep_merge_with_array_dedup(&mut merged, &frag_json, "", prefix, &mut all_conflicts);
        has_any_fragment = true;
    }

    // 合并 CC Switch 供应商 fragment（env/model），在框架之后、用户偏好之前
    let ccs_frag_path = fragment_path(rootfiles_dir, file_name, CCS_PROVIDER_FRAGMENT_PREFIX);
    if ccs_frag_path.exists() {
        let content =
            fs::read_to_string(&ccs_frag_path).map_err(|e| AppError::io(&ccs_frag_path, e))?;
        if let Ok(ccs_json) = serde_json::from_str::<Value>(&content) {
            json_deep_merge_with_array_dedup(
                &mut merged,
                &ccs_json,
                "",
                CCS_PROVIDER_FRAGMENT_PREFIX,
                &mut Vec::new(),
            );
            has_any_fragment = true;
        }
    }

    // 最后合并 user-fragment（用户偏好始终优先，不记录为冲突）
    let user_frag_path = fragment_path(rootfiles_dir, file_name, "user-");
    if user_frag_path.exists() {
        let content =
            fs::read_to_string(&user_frag_path).map_err(|e| AppError::io(&user_frag_path, e))?;
        if let Ok(user_json) = serde_json::from_str::<serde_json::Value>(&content) {
            json_deep_merge_with_array_dedup(
                &mut merged,
                &user_json,
                "",
                "user-",
                &mut Vec::new(), // 用户覆盖框架不视为冲突
            );
            has_any_fragment = true;
        }
    }

    // 写入合并后的根文件
    let dst_path = rootfiles_dir.join(file_name);

    // 如果是 settings.json，清理其中包含 ${CLAUDE_PLUGIN_ROOT} 的无效全局 hooks 字段
    let mut merged_clean = merged.clone();
    if file_name == "settings.json" {
        if let Some(obj) = merged_clean.as_object_mut() {
            if let Some(hooks_val) = obj.get_mut("hooks") {
                if let Some(hooks_obj) = hooks_val.as_object_mut() {
                    sanitize_hooks_for_global_settings(hooks_obj);
                }
                if hooks_val.is_object() && hooks_val.as_object().is_some_and(|o| o.is_empty()) {
                    obj.remove("hooks");
                }
            }
        }
    }

    let content = write_json(&merged_clean)?;
    fs::write(&dst_path, &content).map_err(|e| AppError::io(&dst_path, e))?;

    // 如果是当前激活的生态，也同步写入 live 文件，保证 live 配置文件与当前生态实时一致
    let eco_dir = rootfiles_dir.parent().unwrap_or(rootfiles_dir);
    if is_current_ecosystem(eco_dir) {
        let claude_dir = crate::config::get_claude_config_dir();
        let live_path = claude_dir.join(file_name);
        
        // 如果 live 路径是 symlink，先删掉它
        if is_symlink(&live_path) {
            let _ = fs::remove_file(&live_path);
        }
        
        // 对于 settings.json，如果是空文件，则不写入 (避免覆盖有用的 live 设置)
        if file_name != "settings.json" || !content.trim().is_empty() {
            let mut live_content = merged_clean.clone();
            if file_name == "settings.json" && live_path.exists() {
                if let Ok(existing_content) = fs::read_to_string(&live_path) {
                    if let Ok(existing_json) = serde_json::from_str::<Value>(&existing_content) {
                        preserve_claude_provider_fields(&mut live_content, &existing_json);
                    }
                }
            }

            let live_payload = write_json(&live_content)?;
            if let Err(e) = fs::write(&live_path, &live_payload) {
                log::warn!("同步写入 live 文件失败 {}: {e}", live_path.display());
            } else {
                log::info!("已同步写入 live 文件: {}", live_path.display());
            }
        }
    }

    // 将框架间冲突信息写入 eco.json 的 mergeConflicts 字段
    if !all_conflicts.is_empty() {
        save_merge_conflicts(
            rootfiles_dir.parent().unwrap_or(rootfiles_dir),
            file_name,
            &all_conflicts,
        )?;
        for conflict in &all_conflicts {
            log::warn!("根文件 {} 标量冲突: {conflict}", file_name);
        }
    } else {
        save_merge_conflicts(
            rootfiles_dir.parent().unwrap_or(rootfiles_dir),
            file_name,
            &[],
        )?;
    }

    if has_any_fragment {
        log::info!("根文件 {} 已从 fragment 重建", file_name);
    }

    Ok(())
}

/// 重建 Eco 的所有 JSON 根文件
pub fn rebuild_all_root_files(eco_dir: &Path) -> Result<(), AppError> {
    let eco_json_path = eco_dir.join("eco.json");
    if !eco_json_path.exists() {
        return Ok(());
    }

    let content =
        fs::read_to_string(&eco_json_path).map_err(|e| AppError::io(&eco_json_path, e))?;
    let json: serde_json::Value = parse_json(&content, "解析 eco.json 失败")?;

    let framework_order = extract_string_array(&json, "frameworks");
    let isolated_files = extract_string_array(&json, "isolatedFiles");

    let rootfiles_dir = eco_dir.join("rootfiles");
    if !rootfiles_dir.exists() {
        return Ok(());
    }

    for file_name in &isolated_files {
        if file_name.ends_with(".json") {
            rebuild_root_file(&rootfiles_dir, file_name, &framework_order)?;
        }
    }

    Ok(())
}

/// 将冲突信息写入 eco.json 的 mergeConflicts 字段
pub fn save_merge_conflicts(
    eco_dir: &Path,
    file_name: &str,
    conflicts: &[String],
) -> Result<(), AppError> {
    let eco_json_path = eco_dir.join("eco.json");
    if !eco_json_path.exists() {
        return Ok(());
    }

    let content =
        fs::read_to_string(&eco_json_path).map_err(|e| AppError::io(&eco_json_path, e))?;
    let mut json: serde_json::Value = parse_json(&content, "解析 eco.json 失败")?;

    // 更新 mergeConflicts 字段
    if let Some(obj) = json.as_object_mut() {
        if conflicts.is_empty() {
            if let Some(mc) = obj
                .get_mut("mergeConflicts")
                .and_then(|v| v.as_object_mut())
            {
                mc.remove(file_name);
            }
        } else {
            if !obj.contains_key("mergeConflicts") {
                obj.insert("mergeConflicts".to_string(), serde_json::json!({}));
            }
            if let Some(mc) = obj
                .get_mut("mergeConflicts")
                .and_then(|v| v.as_object_mut())
            {
                mc.insert(
                    file_name.to_string(),
                    serde_json::Value::Array(
                        conflicts
                            .iter()
                            .map(|c| serde_json::Value::String(c.clone()))
                            .collect(),
                    ),
                );
            }
        }
    }

    let content = write_json(&json)?;
    fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

    Ok(())
}

/// 检查指定的 eco_dir 是否是当前正在激活的生态。
/// 优先解析 plugins 符号链接，回退到查询数据库。
fn is_current_ecosystem(eco_dir: &Path) -> bool {
    // 1. 解析 plugins 符号链接（通常指向激活生态的 plugins 文件夹）
    let claude_dir = crate::config::get_claude_config_dir();
    let plugins_symlink = claude_dir.join("plugins");
    if let Ok(target) = fs::read_link(&plugins_symlink) {
        if let Some(target_parent) = target.parent() {
            if let (Ok(p1), Ok(p2)) = (fs::canonicalize(eco_dir), fs::canonicalize(target_parent)) {
                if p1 == p2 {
                    return true;
                }
            }
        }
    }

    // 2. 数据库回退查询
    let db_path = crate::config::get_app_config_dir().join("cc-switch-eco.db");
    if db_path.exists() {
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            let current_id: Result<String, _> = conn.query_row(
                "SELECT id FROM ecosystems WHERE is_current = 1 LIMIT 1",
                [],
                |row| row.get(0),
            );
            if let Ok(id) = current_id {
                let eco_id = eco_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
                return id == eco_id;
            }
        }
    }
    
    false
}

/// 检查路径是否是符号链接
fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.is_symlink())
        .unwrap_or(false)
}
