use std::fs;

use crate::error::AppError;
use super::fragment::{fragment_path, write_json, parse_json};
use super::fragment_rebuild::rebuild_all_root_files;
use super::fragment_isolation::EcoIsolation;

/// 保存用户偏好到 user-fragment
///
/// 将当前合并后的 JSON 根文件内容保存到 user-fragment，确保用户手动修改
/// 的配置在下次重建时不会丢失。用户偏好始终优先于框架配置。
pub fn save_user_preferences(eco_id: &str, file_name: &str) -> Result<(), AppError> {
    let claude_dir = crate::config::get_claude_config_dir();
    let eco_dir = super::ecosystem_dir(eco_id);
    let rootfiles_dir = eco_dir.join("rootfiles");
    let live_file = claude_dir.join(file_name);

    // 优先读取 live 路径文件（保证最新的用户偏好），如果不存在再 fallback 到生态 rootfiles 下的备份
    let content = if live_file.exists() {
        fs::read_to_string(&live_file).map_err(|e| AppError::io(&live_file, e))?
    } else {
        let root_file = rootfiles_dir.join(file_name);
        if !root_file.exists() {
            return Err(AppError::Message(format!("根文件 {file_name} 不存在")));
        }
        fs::read_to_string(&root_file).map_err(|e| AppError::io(&root_file, e))?
    };

    // 验证 JSON 格式
    let _: serde_json::Value = parse_json(&content, "JSON 解析失败")?;

    // 同步写回 root_file 以保持一致性
    let root_file = rootfiles_dir.join(file_name);
    let _ = fs::write(&root_file, &content);

    // 保存到 user-fragment
    let user_fragment = fragment_path(&rootfiles_dir, file_name, "user-");
    fs::write(&user_fragment, &content).map_err(|e| AppError::io(&user_fragment, e))?;

    log::info!("用户偏好已保存到 {}", user_fragment.display());
    Ok(())
}

/// 从 user-fragment 移除指定 key（恢复为框架默认值）
///
/// 从 user-fragment 中删除指定的配置项，下次重建时该 key 将使用框架的值。
pub fn remove_user_preference(
    eco_id: &str,
    file_name: &str,
    key_path: &str,
) -> Result<(), AppError> {
    let eco_dir = super::ecosystem_dir(eco_id);
    let rootfiles_dir = eco_dir.join("rootfiles");
    let user_fragment = fragment_path(&rootfiles_dir, file_name, "user-");

    if !user_fragment.exists() {
        return Ok(());
    }

    let content =
        fs::read_to_string(&user_fragment).map_err(|e| AppError::io(&user_fragment, e))?;
    let mut json: serde_json::Value = parse_json(&content, "JSON 解析失败")?;

    if let Some(obj) = json.as_object_mut() {
        remove_key_by_path(obj, key_path);
    }

    // 如果 user-fragment 变为空对象，删除文件
    if json.is_object() && json.as_object().is_some_and(|o| o.is_empty()) {
        fs::remove_file(&user_fragment).map_err(|e| AppError::io(&user_fragment, e))?;
        log::info!("user-fragment 已清空并删除: {}", user_fragment.display());
    } else {
        let content = write_json(&json)?;
        fs::write(&user_fragment, content).map_err(|e| AppError::io(&user_fragment, e))?;
        log::info!("已从 user-fragment 移除 key: {key_path}");
    }

    // 重建根文件
    rebuild_all_root_files(&eco_dir)?;

    Ok(())
}

/// 按 path 从 JSON object 中移除 key
fn remove_key_by_path(obj: &mut serde_json::Map<String, serde_json::Value>, path: &str) {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return;
    }

    if parts.len() > 1 {
        if let Some(child) = obj.get_mut(parts[0]) {
            if let Some(child_obj) = child.as_object_mut() {
                remove_key_by_path(child_obj, &parts[1..].join("."));
                if child_obj.is_empty() {
                    obj.remove(parts[0]);
                }
            }
        }
    } else {
        obj.remove(parts[0]);
    }
}

/// 将当前 Eco 的用户偏好
///
/// 从 live 路径读取内容并快照到 user-fragment 及 rootfiles，
/// 确保用户手动修改的配置在下次切换回来时不会丢失。
pub fn snapshot_user_preferences(eco_id: &str, isolation: &EcoIsolation) -> Result<(), AppError> {
    let claude_dir = crate::config::get_claude_config_dir();
    let eco_dir = super::ecosystem_dir(eco_id);
    let rootfiles_dir = eco_dir.join("rootfiles");
    if !rootfiles_dir.exists() {
        return Ok(());
    }

    for file_name in &isolation.files {
        let live_file = claude_dir.join(file_name);
        if !live_file.exists() {
            continue;
        }

        if file_name.ends_with(".json") {
            let content = match fs::read_to_string(&live_file) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // 只保存非空 JSON
            if content.trim().is_empty() || content.trim() == "{}" {
                continue;
            }

            // 验证一下是有效的 JSON
            if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
                log::warn!("Live 根文件 {} JSON 损坏，跳过快照: {e}", live_file.display());
                continue;
            }

            // 保存到 eco root_file 以及 user-fragment
            let root_file = rootfiles_dir.join(file_name);
            if let Err(e) = fs::write(&root_file, &content) {
                log::warn!("写入生态根文件失败 {}: {e}", root_file.display());
            }

            let user_fragment = fragment_path(&rootfiles_dir, file_name, "user-");
            if let Err(e) = fs::write(&user_fragment, &content) {
                log::warn!("写入用户 fragment 失败 {}: {e}", user_fragment.display());
            }
            
            log::info!(
                "切换前保存用户偏好: {} → {} (并同步到 rootfiles)",
                live_file.display(),
                user_fragment.display()
            );
        } else {
            // 非 JSON 文件（例如 CLAUDE.md）直接复制 to rootfiles
            let root_file = rootfiles_dir.join(file_name);
            if let Err(e) = fs::copy(&live_file, &root_file) {
                log::warn!("备份非 JSON 根文件失败: {} → {}: {e}", live_file.display(), root_file.display());
            }
        }
    }

    Ok(())
}
