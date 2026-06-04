use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem_framework;

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

/// 从所有 fragment 重建一个 JSON 根文件
///
/// 合并顺序：框架 fragment（按 eco.json 中 frameworks 安装顺序）→ user-fragment（始终最后）
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
    let content = write_json(&merged)?;
    fs::write(&dst_path, content).map_err(|e| AppError::io(&dst_path, e))?;

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

/// 保存用户偏好到 user-fragment
///
/// 将当前合并后的 JSON 根文件内容保存到 user-fragment，确保用户手动修改
/// 的配置在下次重建时不会丢失。用户偏好始终优先于框架配置。
pub fn save_user_preferences(eco_id: &str, file_name: &str) -> Result<(), AppError> {
    let eco_dir = super::ecosystem_dir(eco_id);
    let rootfiles_dir = eco_dir.join("rootfiles");
    let root_file = rootfiles_dir.join(file_name);

    if !root_file.exists() {
        return Err(AppError::Message(format!("根文件 {file_name} 不存在")));
    }

    let content = fs::read_to_string(&root_file).map_err(|e| AppError::io(&root_file, e))?;

    // 验证 JSON 格式
    let _: serde_json::Value = parse_json(&content, "JSON 解析失败")?;

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

/// 保存当前 Eco 的用户偏好
///
/// 将当前合并后的 JSON 根文件内容快照到 user-fragment，
/// 确保用户手动修改的配置在下次切换回来时不会丢失。
pub fn snapshot_user_preferences(eco_id: &str, isolation: &EcoIsolation) -> Result<(), AppError> {
    let eco_dir = super::ecosystem_dir(eco_id);
    let rootfiles_dir = eco_dir.join("rootfiles");
    if !rootfiles_dir.exists() {
        return Ok(());
    }

    for file_name in &isolation.files {
        if !file_name.ends_with(".json") {
            continue;
        }
        let root_file = rootfiles_dir.join(file_name);
        if !root_file.exists() {
            continue;
        }

        let content = match fs::read_to_string(&root_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // 只保存非空 JSON
        if content.trim().is_empty() || content.trim() == "{}" {
            continue;
        }

        let user_fragment = fragment_path(&rootfiles_dir, file_name, "user-");
        fs::write(&user_fragment, &content).map_err(|e| AppError::io(&user_fragment, e))?;
        log::info!(
            "切换前保存用户偏好: {} → {}",
            root_file.display(),
            user_fragment.display()
        );
    }

    Ok(())
}

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

/// Eco 隔离信息（目录 + 文件）
pub struct EcoIsolation {
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}

/// 从 JSON 值中提取字符串数组
fn extract_string_array(json: &serde_json::Value, key: &str) -> Vec<String> {
    json.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
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
}
