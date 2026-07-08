use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem_framework;
use super::plugin_sync::find_latest_version_dir;

pub fn merge_hooks_objects(base: &serde_json::Value, overlay: &serde_json::Value) -> serde_json::Value {
    let mut result = base.clone();
    if !result.is_object() {
        result = serde_json::json!({});
    }
    if let Some(overlay_obj) = overlay.as_object() {
        for (key, value) in overlay_obj {
            if let Some(existing) = result.get(key) {
                let mut merged_arr = Vec::new();
                if let Some(arr) = existing.as_array() {
                    merged_arr.extend(arr.iter().cloned());
                }
                if let Some(arr) = value.as_array() {
                    merged_arr.extend(arr.iter().cloned());
                }
                result
                    .as_object_mut()
                    .unwrap()
                    .insert(key.clone(), serde_json::Value::Array(merged_arr));
            } else {
                result
                    .as_object_mut()
                    .unwrap()
                    .insert(key.clone(), value.clone());
            }
        }
    }
    result
}

/// 从 hooks JSON 中移除引用了指定插件旧版本路径的 hook 条目。
pub fn remove_stale_plugin_hooks(
    hooks: &mut serde_json::Value,
    marketplace_name: &str,
    plugin_name: &str,
    current_version: &str,
) {
    let claude_dir = crate::config::get_claude_config_dir();
    let version_parent = claude_dir
        .join("plugins")
        .join("cache")
        .join(marketplace_name)
        .join(plugin_name);
    let version_parent_str = version_parent.to_str().unwrap_or("");

    if version_parent_str.is_empty() {
        return;
    }

    let current_version_marker = format!("{}/{}/", version_parent_str, current_version);
    let current_version_marker_no_slash = format!("{}/{}", version_parent_str, current_version);

    remove_stale_hooks_recursive(hooks, version_parent_str, &current_version_marker, &current_version_marker_no_slash);
}

/// 递归遍历 hooks JSON，移除引用旧版本路径的 command hook
pub fn remove_stale_hooks_recursive(
    value: &mut serde_json::Value,
    version_parent: &str,
    current_version_marker: &str,
    current_version_marker_no_slash: &str,
) {
    match value {
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                remove_stale_hooks_recursive(item, version_parent, current_version_marker, current_version_marker_no_slash);
            }
            arr.retain(|item| {
                if let Some(obj) = item.as_object() {
                    if obj.contains_key("matcher") && !obj.contains_key("hooks") {
                        return false;
                    }
                    if let Some(cmd) = obj.get("command").and_then(|c| c.as_str()) {
                        if cmd.contains(version_parent)
                            && !cmd.contains(current_version_marker)
                            && !cmd.contains(current_version_marker_no_slash)
                        {
                            log::info!("移除旧版本 hook: {}", cmd);
                            return false;
                        }
                    }
                }
                true
            });
        }
        serde_json::Value::Object(map) => {
            for (_, child) in map.iter_mut() {
                remove_stale_hooks_recursive(child, version_parent, current_version_marker, current_version_marker_no_slash);
            }
            map.retain(|_, v| {
                if let Some(arr) = v.as_array() {
                    return !arr.is_empty();
                }
                true
            });
        }
        _ => {}
    }
}

/// 扫描所有插件的 cache 目录，清理 hooks 中引用旧版本路径的残留条目。
pub fn remove_all_stale_plugin_hooks(hooks: &mut serde_json::Value, eco_plugins_dir: &Path) {
    let cache_dir = eco_plugins_dir.join("cache");
    if !cache_dir.is_dir() {
        return;
    }

    let mut version_triples: Vec<(String, String, String)> = Vec::new();

    if let Ok(marketplace_entries) = fs::read_dir(&cache_dir) {
        for marketplace_entry in marketplace_entries.flatten() {
            if !marketplace_entry.path().is_dir() {
                continue;
            }
            if let Ok(plugin_entries) = fs::read_dir(marketplace_entry.path()) {
                for plugin_entry in plugin_entries.flatten() {
                    if !plugin_entry.path().is_dir() {
                        continue;
                    }
                    let version_parent = plugin_entry.path();
                    if let Some(latest) = find_latest_version_dir(&version_parent) {
                        if let Some(vp_str) = version_parent.to_str() {
                            let cv = latest.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                            let marker_with_slash = format!("{}/{}/", vp_str, cv);
                            let marker_no_slash = format!("{}/{}", vp_str, cv);
                            version_triples.push((vp_str.to_string(), marker_with_slash, marker_no_slash));
                        }
                    }
                }
            }
        }
    }

    for (version_parent, marker_with_slash, marker_no_slash) in &version_triples {
        remove_stale_hooks_recursive(hooks, version_parent, marker_with_slash, marker_no_slash);
    }
}

/// 将 plugin 的 hooks/hooks.json 注入到 eco 的 settings fragment
pub fn inject_plugin_hooks_to_settings(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    let marketplace_name = match framework.marketplace_name.as_ref() {
        Some(m) => m,
        None => return Ok(()),
    };
    let plugin_name = ecosystem_framework::framework_plugin_name(framework);

    let plugins_dir = eco_dir.join("plugins");
    let cache_base = plugins_dir.join("cache").join(marketplace_name).join(plugin_name);
    if !cache_base.is_dir() {
        return Ok(());
    }

    let version_dir = match find_latest_version_dir(&cache_base) {
        Some(v) => v,
        None => return Ok(()),
    };

    let current_version = version_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let hooks_json_path = version_dir.join("hooks").join("hooks.json");
    if !hooks_json_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&hooks_json_path)
        .map_err(|e| AppError::io(&hooks_json_path, e))?;
    let plugin_hooks: serde_json::Value =
        fragment::parse_json(&content, "解析 plugin hooks.json 失败")?;

    let hooks_value = match plugin_hooks.get("hooks") {
        Some(v) if v.is_object() => v.clone(),
        _ => return Ok(()),
    };

    let claude_dir = crate::config::get_claude_config_dir();
    let install_path = claude_dir
        .join("plugins")
        .join("cache")
        .join(marketplace_name)
        .join(plugin_name)
        .join(version_dir.file_name().unwrap_or_default());
    let install_path_str = install_path.to_str().unwrap_or("");

    let hooks_str = serde_json::to_string(&hooks_value).unwrap_or_default();
    let mut resolved_hooks_str = hooks_str.replace("${CLAUDE_PLUGIN_ROOT}", install_path_str);
    resolved_hooks_str = resolved_hooks_str.replace("$CLAUDE_PLUGIN_ROOT", install_path_str);
    let resolved_hooks: serde_json::Value =
        fragment::parse_json(&resolved_hooks_str, "解析替换后的 hooks 失败")?;

    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

    let prefix = &framework.file_prefix;
    let frag_path = fragment::fragment_path(&rootfiles_dir, "settings.json", prefix);

    let mut frag: serde_json::Value = if frag_path.exists() {
        let c = fs::read_to_string(&frag_path).map_err(|e| AppError::io(&frag_path, e))?;
        fragment::parse_json(&c, "解析 settings fragment 失败")?
    } else {
        serde_json::json!({})
    };

    if !frag.is_object() {
        frag = serde_json::json!({});
    }

    if let Some(existing_hooks) = frag.get_mut("hooks") {
        remove_stale_plugin_hooks(existing_hooks, marketplace_name, plugin_name, &current_version);
    }

    let existing_hooks = frag.get("hooks").cloned().unwrap_or(serde_json::json!({}));
    let merged = merge_hooks_objects(&existing_hooks, &resolved_hooks);
    frag.as_object_mut()
        .unwrap()
        .insert("hooks".to_string(), merged);

    let output = fragment::write_json(&frag)?;
    fs::write(&frag_path, output).map_err(|e| AppError::io(&frag_path, e))?;

    log::info!(
        "已将 plugin '{}' 的 hooks 注入到 settings fragment (version: {})",
        framework.id,
        current_version
    );
    Ok(())
}

/// plugin hooks 由 Claude Code 插件系统执行，不能留在框架 settings fragment
pub fn remove_plugin_hooks_from_settings_fragment(eco_dir: &Path, prefix: &str) {
    let frag_path =
        fragment::fragment_path(&eco_dir.join("rootfiles"), "settings.json", prefix);
    if !frag_path.exists() {
        return;
    }
    let Ok(content) = fs::read_to_string(&frag_path) else {
        return;
    };
    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return;
    };
    if let Some(obj) = json.as_object_mut() {
        obj.remove("hooks");
        if obj.is_empty() {
            if let Err(e) = fs::remove_file(&frag_path) {
                log::warn!("删除空 settings fragment 失败 {}: {e}", frag_path.display());
            }
            return;
        }
    }
    if let Ok(serialized) = fragment::write_json(&json) {
        if let Err(e) = fs::write(&frag_path, serialized) {
            log::warn!("更新 settings fragment 失败 {}: {e}", frag_path.display());
        }
    }
}
