use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::fs_utils;
use crate::services::ecosystem_framework;
use crate::services::ecosystem::cmd_utils::get_git_commit_hash;
use crate::services::ecosystem::plugin_sync::resolve_plugin_source_dir;
use crate::services::ecosystem::hud_ops::auto_setup_hud;

// Re-export hook functions so they are available under `plugin_ops` if needed
pub use super::hook_ops::{
    merge_hooks_objects, remove_stale_plugin_hooks, remove_stale_hooks_recursive,
    remove_all_stale_plugin_hooks
};

/// 将 plugin 类型框架注册到 Claude Code 的插件发现系统
pub fn register_plugin_to_installed_plugins(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    let plugin_key = ecosystem_framework::framework_plugin_key(framework).ok_or_else(|| {
        AppError::Message(format!(
            "框架 '{}' 是 plugin 类型但未配置 marketplace_name",
            framework.id
        ))
    })?;

    let marketplace_name = framework.marketplace_name.as_ref().unwrap();
    let plugin_name = ecosystem_framework::framework_plugin_name(framework);

    let plugins_dir = eco_dir.join("plugins");

    let plugin_staging_dir = &framework.id;
    let plugin_src = resolve_plugin_source_dir(eco_dir, plugin_staging_dir);

    // 读取 plugin.json 获取版本信息
    let plugin_json_path = plugin_src.join(".claude-plugin").join("plugin.json");
    let version = if plugin_json_path.exists() {
        let content = fs::read_to_string(&plugin_json_path)
            .map_err(|e| AppError::io(&plugin_json_path, e))?;
        let json: serde_json::Value = fragment::parse_json(&content, "解析 plugin.json 失败")?;
        json.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string()
    } else {
        "0.0.0".to_string()
    };

    let marketplace_json_path = plugin_src.join(".claude-plugin").join("marketplace.json");
    let git_commit_sha = get_git_commit_hash(&eco_dir.join("frameworks").join(plugin_staging_dir));

    let now = chrono::Utc::now().to_rfc3339();

    // 1. 创建 cache/{marketplaceName}/{pluginName}/{version}/ 目录并复制插件内容
    let cache_install_path = plugins_dir
        .join("cache")
        .join(marketplace_name)
        .join(plugin_name)
        .join(&version);
    if !cache_install_path.exists() {
        fs_utils::copy_dir_recursive(&plugin_src, &cache_install_path)?;
    }

    let cp_plugin_dir = cache_install_path.join(".claude-plugin");
    if cp_plugin_dir.exists() {
        if let Ok(entries) = fs::read_dir(&cp_plugin_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let src = entry.path();
                if src.is_dir() && !name.starts_with('.') && name != "marketplace.json" {
                    let dst = cache_install_path.join(&name);
                    if !dst.exists() {
                        fs_utils::copy_dir_recursive(&src, &dst)?;
                    }
                }
            }
        }
    }

    // 2. 创建 data/{pluginName}-{marketplaceName}/ 目录
    let data_dir = plugins_dir
        .join("data")
        .join(format!("{plugin_name}-{marketplace_name}"));
    fs::create_dir_all(&data_dir).map_err(|e| AppError::io(&data_dir, e))?;

    // 3. 创建 marketplaces/{marketplaceName}/ 目录
    let marketplace_dir = plugins_dir.join("marketplaces").join(marketplace_name);
    if !marketplace_dir.exists() {
        let fw_dir = eco_dir.join("frameworks").join(plugin_staging_dir);
        if fw_dir.exists() {
            fs_utils::copy_dir_recursive(&fw_dir, &marketplace_dir)?;
            let git_dir = marketplace_dir.join(".git");
            if git_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&git_dir) {
                    log::warn!("清理 marketplace .git 目录失败: {e}");
                }
            }
        } else {
            fs::create_dir_all(&marketplace_dir)
                .map_err(|e| AppError::io(&marketplace_dir, e))?;
            let marketplace_plugin_dir = marketplace_dir.join(".claude-plugin");
            fs::create_dir_all(&marketplace_plugin_dir)
                .map_err(|e| AppError::io(&marketplace_plugin_dir, e))?;
            if marketplace_json_path.exists() {
                fs::copy(&marketplace_json_path, marketplace_plugin_dir.join("marketplace.json"))
                    .map_err(|e| AppError::io(&marketplace_plugin_dir.join("marketplace.json"), e))?;
            }
        }
    }

    // 4. 更新 installed_plugins.json
    let installed_plugins_path = plugins_dir.join("installed_plugins.json");
    let mut installed_json: serde_json::Value = if installed_plugins_path.exists() {
        let content = fs::read_to_string(&installed_plugins_path)
            .map_err(|e| AppError::io(&installed_plugins_path, e))?;
        fragment::parse_json(&content, "解析 installed_plugins.json 失败")?
    } else {
        serde_json::json!({ "version": 2, "plugins": {} })
    };

    if !installed_json.is_object() {
        installed_json = serde_json::json!({ "version": 2, "plugins": {} });
    }
    if installed_json.get("version").is_none() {
        installed_json
            .as_object_mut()
            .unwrap()
            .insert("version".to_string(), serde_json::json!(2));
    }
    if installed_json.get("plugins").is_none() {
        installed_json
            .as_object_mut()
            .unwrap()
            .insert("plugins".to_string(), serde_json::json!({}));
    }

    let claude_dir = crate::config::get_claude_config_dir();
    let install_path_str = claude_dir
        .join("plugins")
        .join("cache")
        .join(marketplace_name)
        .join(plugin_name)
        .join(&version)
        .to_str()
        .unwrap_or("")
        .to_string();

    let mut entry = serde_json::json!({
        "scope": "user",
        "installPath": install_path_str,
        "version": version,
        "installedAt": now,
        "lastUpdated": now
    });
    if let Some(sha) = git_commit_sha {
        entry
            .as_object_mut()
            .unwrap()
            .insert("gitCommitSha".to_string(), serde_json::Value::String(sha));
    }

    if let Some(plugins_obj) = installed_json.get_mut("plugins").and_then(|v| v.as_object_mut()) {
        plugins_obj.insert(plugin_key.clone(), serde_json::json!([entry]));
    }

    let content = fragment::write_json(&installed_json)?;
    fs::write(&installed_plugins_path, content)
        .map_err(|e| AppError::io(&installed_plugins_path, e))?;

    // 5. 更新 known_marketplaces.json
    let known_marketplaces_path = plugins_dir.join("known_marketplaces.json");
    let mut marketplaces_json: serde_json::Value = if known_marketplaces_path.exists() {
        let content = fs::read_to_string(&known_marketplaces_path)
            .map_err(|e| AppError::io(&known_marketplaces_path, e))?;
        fragment::parse_json(&content, "解析 known_marketplaces.json 失败")?
    } else {
        serde_json::json!({})
    };

    if !marketplaces_json.is_object() {
        marketplaces_json = serde_json::json!({});
    }

    let marketplace_install_location = claude_dir
        .join("plugins")
        .join("marketplaces")
        .join(marketplace_name)
        .to_str()
        .unwrap_or("")
        .to_string();

    let source = serde_json::json!({
        "source": "github",
        "repo": extract_github_repo(&framework.repo_url)
    });

    let marketplace_entry = serde_json::json!({
        "source": source,
        "installLocation": marketplace_install_location,
        "lastUpdated": now
    });

    if let Some(obj) = marketplaces_json.as_object_mut() {
        obj.insert(marketplace_name.clone(), marketplace_entry);
    }

    let content = fragment::write_json(&marketplaces_json)?;
    fs::write(&known_marketplaces_path, content)
        .map_err(|e| AppError::io(&known_marketplaces_path, e))?;

    enable_plugin_in_settings(eco_dir, &plugin_key)?;

    if framework.id == "claude-hud" {
        auto_setup_hud(eco_dir, &cache_install_path)?;
    }

    log::info!(
        "已将插件 '{}' 注册到 Claude Code 插件系统 (key: {})",
        framework.id,
        plugin_key
    );
    Ok(())
}

/// 从 installed_plugins.json 和 known_marketplaces.json 中移除插件注册
pub fn unregister_plugin_from_installed_plugins(
    eco_dir: &Path,
    framework_id: &str,
) -> Result<(), AppError> {
    let framework = ecosystem_framework::find_framework(framework_id);
    let marketplace_name = framework
        .as_ref()
        .and_then(|f| f.marketplace_name.as_ref());

    let plugins_dir = eco_dir.join("plugins");

    // 从 installed_plugins.json 中移除
    let installed_plugins_path = plugins_dir.join("installed_plugins.json");
    if installed_plugins_path.exists() {
        let content = fs::read_to_string(&installed_plugins_path)
            .map_err(|e| AppError::io(&installed_plugins_path, e))?;
        let mut json: serde_json::Value = fragment::parse_json(&content, "解析 installed_plugins.json 失败")?;

        if let Some(plugins_obj) = json.get_mut("plugins").and_then(|v| v.as_object_mut()) {
            let actual_key: Option<String> = framework
                .as_ref()
                .and_then(|f| ecosystem_framework::framework_plugin_key(f));
            let prefix_match = format!("{framework_id}@");
            let keys_to_remove: Vec<String> = plugins_obj
                .keys()
                .filter(|k| {
                    let ks = k.as_str();
                    match &actual_key {
                        Some(ak) => ks == ak,
                        None => ks == framework_id || ks.starts_with(&prefix_match),
                    }
                })
                .cloned()
                .collect();
            for key in keys_to_remove {
                plugins_obj.remove(&key);
            }
        }

        let content = fragment::write_json(&json)?;
        fs::write(&installed_plugins_path, content)
            .map_err(|e| AppError::io(&installed_plugins_path, e))?;
    }

    // 从 known_marketplaces.json 中移除
    if let Some(mkt_name) = marketplace_name {
        let known_marketplaces_path = plugins_dir.join("known_marketplaces.json");
        if known_marketplaces_path.exists() {
            let content = fs::read_to_string(&known_marketplaces_path)
                .map_err(|e| AppError::io(&known_marketplaces_path, e))?;
            let mut json: serde_json::Value = fragment::parse_json(&content, "解析 known_marketplaces.json 失败")?;

            if let Some(obj) = json.as_object_mut() {
                obj.remove(mkt_name);
            }

            let content = fragment::write_json(&json)?;
            fs::write(&known_marketplaces_path, content)
                .map_err(|e| AppError::io(&known_marketplaces_path, e))?;
        }

        // 清理 marketplace 目录
        let marketplace_dir = plugins_dir.join("marketplaces").join(mkt_name);
        if marketplace_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&marketplace_dir) {
                log::warn!("清理 marketplace 目录失败 {}: {e}", marketplace_dir.display());
            }
        }

        // 清理 cache 目录
        let cache_dir = plugins_dir.join("cache").join(mkt_name);
        if cache_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&cache_dir) {
                log::warn!("清理 cache 目录失败 {}: {e}", cache_dir.display());
            }
        }

        // 清理 data 目录
        let data_dir = plugins_dir.join("data").join(format!("{framework_id}-{mkt_name}"));
        if data_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&data_dir) {
                log::warn!("清理 data 目录失败 {}: {e}", data_dir.display());
            }
        }
    }

    let plugin_key = framework
        .as_ref()
        .and_then(|f| ecosystem_framework::framework_plugin_key(f))
        .unwrap_or_else(|| framework_id.to_string());
    disable_plugin_in_settings(eco_dir, &plugin_key)?;

    if framework_id == "claude-hud" {
        super::hud_ops::cleanup_hud_settings(eco_dir)?;
    }

    log::info!(
        "已从 Claude Code 插件系统中移除插件 '{}'",
        framework_id
    );
    Ok(())
}

/// 在 eco 的 settings user-fragment 中启用插件
pub fn enable_plugin_in_settings(eco_dir: &Path, plugin_key: &str) -> Result<(), AppError> {
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

    let user_frag_path = fragment::fragment_path(&rootfiles_dir, "settings.json", "user-");

    let mut user_frag: serde_json::Value = if user_frag_path.exists() {
        let content = fs::read_to_string(&user_frag_path)
            .map_err(|e| AppError::io(&user_frag_path, e))?;
        fragment::parse_json(&content, "解析 user-fragment 失败")?
    } else {
        serde_json::json!({})
    };

    if !user_frag.is_object() {
        user_frag = serde_json::json!({});
    }

    if user_frag.get("enabledPlugins").is_none() {
        user_frag
            .as_object_mut()
            .unwrap()
            .insert("enabledPlugins".to_string(), serde_json::json!({}));
    }

    if let Some(ep) = user_frag.get_mut("enabledPlugins").and_then(|v| v.as_object_mut()) {
        ep.insert(plugin_key.to_string(), serde_json::json!(true));
    }

    let content = fragment::write_json(&user_frag)?;
    fs::write(&user_frag_path, content)
        .map_err(|e| AppError::io(&user_frag_path, e))?;

    ensure_settings_json_in_isolated_files(eco_dir)?;
    fragment::rebuild_all_root_files(eco_dir)?;

    log::info!("已在 user-fragment 中启用插件 '{}'", plugin_key);
    Ok(())
}

pub fn ensure_settings_json_in_isolated_files(eco_dir: &Path) -> Result<(), AppError> {
    let eco_json_path = eco_dir.join("eco.json");
    if !eco_json_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&eco_json_path).map_err(|e| AppError::io(&eco_json_path, e))?;
    let mut eco_json: serde_json::Value =
        fragment::parse_json(&content, "解析 eco.json 失败")?;

    if let Some(files) = eco_json.get_mut("isolatedFiles").and_then(|v| v.as_array_mut()) {
        let has_settings = files.iter().any(|v| v.as_str() == Some("settings.json"));
        if !has_settings {
            files.push(serde_json::Value::String("settings.json".to_string()));
            let updated = fragment::write_json(&eco_json)?;
            fs::write(&eco_json_path, updated).map_err(|e| AppError::io(&eco_json_path, e))?;
        }
    }

    Ok(())
}

/// 从 eco 的 settings user-fragment 中移除插件
pub fn disable_plugin_in_settings(eco_dir: &Path, plugin_key: &str) -> Result<(), AppError> {
    let rootfiles_dir = eco_dir.join("rootfiles");
    let user_frag_path = fragment::fragment_path(&rootfiles_dir, "settings.json", "user-");

    if !user_frag_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&user_frag_path)
        .map_err(|e| AppError::io(&user_frag_path, e))?;
    let mut user_frag: serde_json::Value =
        fragment::parse_json(&content, "解析 user-fragment 失败")?;

    if let Some(ep) = user_frag.get_mut("enabledPlugins").and_then(|v| v.as_object_mut()) {
        ep.remove(plugin_key);
    }

    let content = fragment::write_json(&user_frag)?;
    fs::write(&user_frag_path, content)
        .map_err(|e| AppError::io(&user_frag_path, e))?;

    fragment::rebuild_all_root_files(eco_dir)?;

    log::info!("已从 user-fragment 中移除插件 '{}'", plugin_key);
    Ok(())
}

/// 从 GitHub URL 提取 owner/repo 格式
pub fn extract_github_repo(url: &str) -> String {
    url
        .strip_prefix("https://github.com/")
        .and_then(|s| s.strip_suffix(".git"))
        .unwrap_or(url)
        .to_string()
}

/// 将 installPath（通常为 ~/.claude/plugins/...）解析为 eco 物理路径
pub fn resolve_plugin_hooks_dir(eco_dir: &Path, install_path: &str) -> PathBuf {
    let claude_plugins = crate::config::get_claude_config_dir().join("plugins");
    let eco_plugins = eco_dir.join("plugins");
    let path = Path::new(install_path);

    if let Ok(rel) = path.strip_prefix(&claude_plugins) {
        return eco_plugins.join(rel).join("hooks");
    }
    if let Ok(rel) = path.strip_prefix(&eco_plugins) {
        return eco_plugins.join(rel).join("hooks");
    }
    let legacy = eco_dir.join(".claude").join("plugins");
    if let Ok(rel) = path.strip_prefix(&legacy) {
        return eco_plugins.join(rel).join("hooks");
    }
    path.join("hooks")
}

/// 删除 skills/ 下由旧版 npx skills 复制的孤立目录
pub fn cleanup_orphan_plugin_skill_dirs(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) {
    let skills_dir = eco_dir.join("skills");
    if !skills_dir.is_dir() {
        return;
    }
    let prefix = &framework.file_prefix;
    let Ok(entries) = fs::read_dir(&skills_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(prefix) {
            continue;
        }
        let path = entry.path();
        if path.is_dir()
            && !path.join(".claude-plugin").exists()
            && !path.join("SKILL.md").exists()
        {
            log::info!("清理孤立 skill 目录: {}", path.display());
            if let Err(e) = fs::remove_dir_all(&path) {
                log::warn!("清理孤立 skill 目录失败 {}: {e}", path.display());
            }
        }
    }
}

pub fn rewrite_plugin_install_path(
    install_path: &str,
    claude_plugins: &Path,
    eco_plugins: &Path,
    legacy_prefix: &Path,
) -> Option<String> {
    let path = Path::new(install_path);
    let rel = path
        .strip_prefix(eco_plugins)
        .ok()
        .or_else(|| path.strip_prefix(legacy_prefix).ok())
        .or_else(|| path.strip_prefix(claude_plugins).ok())?;

    Some(
        claude_plugins
            .join(rel)
            .to_str()
            .map(String::from)
            .unwrap_or_else(|| install_path.to_string()),
    )
}

#[cfg(test)]
#[path = "plugin_ops_tests.rs"]
mod plugin_ops_tests;
