use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::fs_utils;
use crate::services::ecosystem_framework;

use super::plugin_ops::{enable_plugin_in_settings, remove_all_stale_plugin_hooks};

/// 将 staging/fw_dir 转换为合适的 plugin 源码根目录
pub fn resolve_plugin_source_dir(eco_dir: &Path, framework_id: &str) -> PathBuf {
    let staging = eco_dir.join("plugins").join(framework_id);
    let fw_dir = eco_dir.join("frameworks").join(framework_id);
    if staging.join(".claude-plugin").exists() {
        staging
    } else {
        fw_dir
    }
}

/// 查找最新版本的插件目录
pub fn find_latest_version_dir(base: &Path) -> Option<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(base) else {
        return None;
    };
    let mut latest: Option<(String, std::path::PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().is_dir() {
            match &latest {
                Some((prev, _)) if name <= *prev => {}
                _ => latest = Some((name, entry.path())),
            }
        }
    }
    latest.map(|(_, p)| p)
}

/// 将 plugin cache 安装路径中的 skills/commands/agents 子目录内容
/// 以带前缀的方式复制到 eco 对应的隔离目录，使 Claude Code 能通过
/// ~/.claude/skills/ 和 ~/.claude/commands/ symlink 发现 these 内容。
pub fn expose_plugin_dirs_to_eco_isolation(
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

    let prefix = &framework.file_prefix;
    let expose_dirs = ["skills", "commands", "agents", "hooks"];

    for dir_name in &expose_dirs {
        let src_dir = version_dir.join(dir_name);
        if !src_dir.is_dir() {
            continue;
        }

        // plugin 框架的 skills/commands/agents 已通过 plugin namespace
        // （installed_plugins.json + enabledPlugins）被 Claude Code 发现，
        // flat exposure 会产生重复注册（如 superpowers-zh:xxx 和 superpowers-xxx）。
        // hooks 例外：hooks 通过 settings.json 注入，不走 namespace 发现。
        if dir_name != &"hooks" {
            log::info!(
                "跳过暴露插件 '{}' 目录：plugin namespace 已提供发现路径",
                dir_name
            );
            continue;
        }

        let eco_sub_dir = eco_dir.join(dir_name);
        fs::create_dir_all(&eco_sub_dir).map_err(|e| AppError::io(&eco_sub_dir, e))?;

        let Ok(entries) = fs::read_dir(&src_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let src_path = entry.path();

            let prefixed_name = if name.starts_with(prefix) {
                name.clone()
            } else {
                format!("{prefix}{name}")
            };

            if src_path.is_dir() {
                let dst_path = eco_sub_dir.join(&prefixed_name);
                if dst_path.exists() {
                    continue;
                }

                if let Err(e) = fs_utils::copy_dir_recursive(&src_path, &dst_path) {
                    log::warn!(
                        "复制 plugin {} 目录失败: {} -> {}: {e}",
                        dir_name,
                        src_path.display(),
                        dst_path.display()
                    );
                }
            } else if src_path.is_file() {
                if dir_name == &"hooks" && name == "hooks.json" {
                    continue;
                }
                let dst_path = eco_sub_dir.join(&prefixed_name);
                if dst_path.exists() {
                    continue;
                }
                if let Err(e) = fs::copy(&src_path, &dst_path) {
                    log::warn!(
                        "复制 plugin {} 文件失败: {} -> {}: {e}",
                        dir_name,
                        src_path.display(),
                        dst_path.display()
                    );
                }
            }
        }
    }

    Ok(())
}

/// 将 HOME 重定向产生的 .claude/plugins/ 合并进 eco/plugins/（保留 cache 等标准结构，不加前缀）
pub fn merge_claude_plugins_into_eco(src_plugins: &Path, dst_plugins: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dst_plugins).map_err(|e| AppError::io(dst_plugins, e))?;

    if !src_plugins.exists() {
        return Err(AppError::Message(format!(
            "plugin 源目录不存在: {}",
            src_plugins.display()
        )));
    }

    for entry in fs::read_dir(src_plugins).map_err(|e| AppError::io(src_plugins, e))? {
        let entry = entry.map_err(|e| AppError::io(src_plugins, e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst_plugins.join(&name);

        if src_path.is_file() {
            if name == "installed_plugins.json" || name == "known_marketplaces.json" {
                merge_plugin_json_file(&src_path, &dst_path)?;
            } else if !dst_path.exists() {
                fs::copy(&src_path, &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
            }
        } else if src_path.is_dir() {
            if !dst_path.exists() {
                fs_utils::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs_utils::copy_dir_recursive(&src_path, &dst_path)?;
            }
        }
    }
    Ok(())
}

pub fn merge_plugin_json_file(src: &Path, dst: &Path) -> Result<(), AppError> {
    let src_content = fs::read_to_string(src).map_err(|e| AppError::io(src, e))?;
    let src_json: serde_json::Value =
        fragment::parse_json(&src_content, "解析 plugin JSON 失败")?;

    if !dst.exists() {
        fs::write(dst, fragment::write_json(&src_json)?).map_err(|e| AppError::io(dst, e))?;
        return Ok(());
    }

    let dst_content = fs::read_to_string(dst).map_err(|e| AppError::io(dst, e))?;
    let mut dst_json: serde_json::Value =
        fragment::parse_json(&dst_content, "解析 plugin JSON 失败")?;

    if dst.file_name().is_some_and(|n| n == "installed_plugins.json") {
        if let Some(src_plugins) = src_json.get("plugins").and_then(|v| v.as_object()) {
            if !dst_json.is_object() {
                dst_json = serde_json::json!({ "version": 2, "plugins": {} });
            }
            if dst_json.get("plugins").is_none() {
                dst_json
                    .as_object_mut()
                    .unwrap()
                    .insert("plugins".to_string(), serde_json::json!({}));
            }
            if let Some(dst_plugins) = dst_json.get_mut("plugins").and_then(|v| v.as_object_mut())
            {
                for (key, val) in src_plugins {
                    dst_plugins.insert(key.clone(), val.clone());
                }
            }
        }
    } else if dst.file_name().is_some_and(|n| n == "known_marketplaces.json") {
        if let Some(src_obj) = src_json.as_object() {
            if !dst_json.is_object() {
                dst_json = serde_json::json!({});
            }
            if let Some(dst_obj) = dst_json.as_object_mut() {
                for (key, val) in src_obj {
                    dst_obj.insert(key.clone(), val.clone());
                }
            }
        }
    } else {
        let mut conflicts = Vec::new();
        fragment::json_deep_merge_with_array_dedup(
            &mut dst_json,
            &src_json,
            "",
            "plugin-",
            &mut conflicts,
        );
    }

    fs::write(dst, fragment::write_json(&dst_json)?).map_err(|e| AppError::io(dst, e))?;
    Ok(())
}

/// 将 installPath 从 eco 临时路径改写为 ~/.claude/plugins/...（运行时通过 symlink 映射到 eco）
pub fn normalize_plugin_install_paths(eco_dir: &Path) -> Result<(), AppError> {
    let installed_path = eco_dir.join("plugins").join("installed_plugins.json");
    if !installed_path.exists() {
        return Ok(());
    }

    let content =
        fs::read_to_string(&installed_path).map_err(|e| AppError::io(&installed_path, e))?;
    let mut json: serde_json::Value =
        fragment::parse_json(&content, "解析 installed_plugins.json 失败")?;

    let claude_plugins = crate::config::get_claude_config_dir().join("plugins");
    let eco_plugins = eco_dir.join("plugins");
    let legacy_prefix = eco_dir.join(".claude").join("plugins");

    if let Some(plugins_obj) = json.get_mut("plugins").and_then(|v| v.as_object_mut()) {
        for entries in plugins_obj.values_mut() {
            if let Some(arr) = entries.as_array_mut() {
                for entry in arr.iter_mut() {
                    if let Some(obj) = entry.as_object_mut() {
                        if let Some(install_path) =
                            obj.get("installPath").and_then(|v| v.as_str())
                        {
                            if let Some(new_path) = rewrite_plugin_install_path(
                                install_path,
                                &claude_plugins,
                                &eco_plugins,
                                &legacy_prefix,
                            ) {
                                obj.insert(
                                    "installPath".to_string(),
                                    serde_json::Value::String(new_path),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fs::write(&installed_path, fragment::write_json(&json)?)
        .map_err(|e| AppError::io(&installed_path, e))?;

    let known_path = eco_dir.join("plugins").join("known_marketplaces.json");
    if known_path.exists() {
        let content = fs::read_to_string(&known_path).map_err(|e| AppError::io(&known_path, e))?;
        let mut known: serde_json::Value =
            fragment::parse_json(&content, "解析 known_marketplaces.json 失败")?;
        if let Some(obj) = known.as_object_mut() {
            for entry in obj.values_mut() {
                if let Some(entry_obj) = entry.as_object_mut() {
                    if let Some(loc) = entry_obj
                        .get("installLocation")
                        .and_then(|v| v.as_str())
                    {
                        if let Some(new_loc) = rewrite_plugin_install_path(
                            loc,
                            &claude_plugins,
                            &eco_plugins,
                            &legacy_prefix,
                        ) {
                            entry_obj.insert(
                                "installLocation".to_string(),
                                serde_json::Value::String(new_loc),
                            );
                        }
                    }
                }
            }
        }
        fs::write(&known_path, fragment::write_json(&known)?)
            .map_err(|e| AppError::io(&known_path, e))?;
    }

    Ok(())
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

/// 将官方 plugin 安装写入的 enabledPlugins 合并到 eco user-fragment
pub fn merge_claude_plugin_settings_to_eco(
    eco_dir: &Path,
    settings_path: &Path,
) -> Result<(), AppError> {
    if !settings_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(settings_path).map_err(|e| AppError::io(settings_path, e))?;
    let settings: serde_json::Value =
        fragment::parse_json(&content, "解析 .claude/settings.json 失败")?;

    if let Some(enabled) = settings.get("enabledPlugins").and_then(|v| v.as_object()) {
        for (plugin_key, val) in enabled {
            if val.as_bool() == Some(true) {
                enable_plugin_in_settings(eco_dir, plugin_key)?;
            }
        }
    }

    // 官方 plugin CLI 写入的 hooks 包含 $CLAUDE_PLUGIN_ROOT，
    // 需要替换为绝对路径后写入 fragment，否则 Claude Code 运行时无法解析
    if let Some(hooks) = settings.get("hooks") {
        if hooks.is_object() {
            let claude_dir = crate::config::get_claude_config_dir();
            let plugins_dir = eco_dir.join("plugins");
            let hooks_str = serde_json::to_string(hooks).unwrap_or_default();
            let resolved = resolve_plugin_root_in_hooks(&hooks_str, &claude_dir.join("plugins"), &plugins_dir);
            let resolved_hooks: serde_json::Value =
                fragment::parse_json(&resolved, "解析替换后的 hooks 失败")?;

            let rootfiles_dir = eco_dir.join("rootfiles");
            fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

            // 写入 plugin-fragment（由 finalize_plugin_framework_install 统一管理）
            let frag_path = fragment::fragment_path(&rootfiles_dir, "settings.json", "plugin-");
            let mut frag: serde_json::Value = if frag_path.exists() {
                let c = fs::read_to_string(&frag_path).map_err(|e| AppError::io(&frag_path, e))?;
                fragment::parse_json(&c, "解析 plugin-fragment 失败")?
            } else {
                serde_json::json!({})
            };

            // 清理所有插件旧版本路径的残留 hooks
            if let Some(existing_hooks) = frag.get_mut("hooks") {
                remove_all_stale_plugin_hooks(existing_hooks, &plugins_dir);
            }

            let existing = frag.get("hooks").cloned().unwrap_or(serde_json::json!({}));
            let merged = super::plugin_ops::merge_hooks_objects(&existing, &resolved_hooks);
            frag.as_object_mut()
                .unwrap()
                .insert("hooks".to_string(), merged);

            let output = fragment::write_json(&frag)?;
            fs::write(&frag_path, output).map_err(|e| AppError::io(&frag_path, e))?;
        }
    }

    Ok(())
}

/// 将 hooks 中的 $CLAUDE_PLUGIN_ROOT 替换为绝对路径
/// 官方 CLI 写入的路径形如 ~/.claude/plugins/cache/{marketplace}/{plugin}/{version}/
/// 在 eco 中实际位于 eco_dir/plugins/cache/...，通过 symlink 映射
pub fn resolve_plugin_root_in_hooks(hooks_str: &str, claude_plugins: &Path, eco_plugins: &Path) -> String {
    let mut result = hooks_str.to_string();
    let claude_prefix = claude_plugins.to_str().unwrap_or("");
    let eco_prefix = eco_plugins.to_str().unwrap_or("");

    // 先替换绝对路径引用（eco_dir/.claude/plugins/... → ~/.claude/plugins/...）
    if !eco_prefix.is_empty() && result.contains(eco_prefix) {
        result = result.replace(eco_prefix, claude_prefix);
    }

    // 再替换 $CLAUDE_PLUGIN_ROOT
    // 需要从 installed_plugins.json 中查找每个插件 of installPath
    // 简化处理：遍历 cache 目录结构，构建 marketplace/plugin/version → path 映射
    let cache_dir = eco_plugins.join("cache");
    if cache_dir.is_dir() {
        if let Ok(marketplace_entries) = fs::read_dir(&cache_dir) {
            for marketplace_entry in marketplace_entries.flatten() {
                if !marketplace_entry.path().is_dir() {
                    continue;
                }
                let marketplace_name = marketplace_entry.file_name().to_string_lossy().to_string();
                let marketplace_dir = marketplace_entry.path();
                if let Ok(plugin_entries) = fs::read_dir(&marketplace_dir) {
                    for plugin_entry in plugin_entries.flatten() {
                        if !plugin_entry.path().is_dir() {
                            continue;
                        }
                        let plugin_name = plugin_entry.file_name().to_string_lossy().to_string();
                        let plugin_dir = plugin_entry.path();
                        if let Ok(version_entries) = fs::read_dir(&plugin_dir) {
                            for version_entry in version_entries.flatten() {
                                if !version_entry.path().is_dir() {
                                    continue;
                                }
                                let version = version_entry.file_name().to_string_lossy().to_string();
                                let install_path = claude_plugins
                                    .join("cache")
                                    .join(&marketplace_name)
                                    .join(&plugin_name)
                                    .join(&version);
                                let install_path_str = install_path.to_str().unwrap_or("");
                                if !install_path_str.is_empty() {
                                    result = result.replace("${CLAUDE_PLUGIN_ROOT}", install_path_str);
                                    result = result.replace("$CLAUDE_PLUGIN_ROOT", install_path_str);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

/// 收集 SSOT 技能目录中所有已存在的技能目录名（小写，用于去重比较）。
///
/// SSOT 路径取决于设置：`~/.cc-switch/skills/` 或 `~/.agents/skills/`。
/// 同时扫描 eco 隔离目录下的 skills/，因为 SSOT 同步后技能也会出现在那里。
pub fn collect_ssot_skill_names(eco_dir: &Path) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();

    // 扫描 SSOT 目录
    if let Ok(ssot_dir) = crate::services::skill::SkillService::get_ssot_dir() {
        collect_skill_dir_names(&ssot_dir, &mut names);
    }

    // 扫描 eco 隔离目录下的 skills/
    let eco_skills = eco_dir.join("skills");
    if eco_skills.is_dir() {
        collect_skill_dir_names(&eco_skills, &mut names);
    }

    names
}

fn collect_skill_dir_names(dir: &Path, names: &mut std::collections::HashSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || !entry.path().is_dir() {
            continue;
        }
        names.insert(name.to_lowercase());
    }
}

#[cfg(test)]
mod plugin_sync_tests {
    use super::*;

    #[test]
    fn test_rewrite_plugin_install_path_from_eco_claude_temp() {
        let eco_dir = PathBuf::from("/tmp/eco-test");
        let claude_plugins = PathBuf::from("/Users/me/.claude/plugins");
        let eco_plugins = eco_dir.join("plugins");
        let legacy = eco_dir.join(".claude/plugins");

        let old = legacy.join("cache/pua-skills/pua/3.5.0");
        let new = rewrite_plugin_install_path(
            old.to_str().unwrap(),
            &claude_plugins,
            &eco_plugins,
            &legacy,
        )
        .expect("should rewrite legacy .claude/plugins path");

        assert_eq!(
            new,
            claude_plugins
                .join("cache/pua-skills/pua/3.5.0")
                .to_string_lossy()
        );
    }

    #[test]
    fn test_resolve_plugin_source_dir_prefers_fw_dir_when_staging_missing() {
        let dir = tempfile::tempdir().unwrap();
        let eco_dir = dir.path();
        let fw_dir = eco_dir.join("frameworks").join("pua");
        fs::create_dir_all(fw_dir.join(".claude-plugin")).unwrap();
        fs::write(fw_dir.join(".claude-plugin").join("plugin.json"), "{}").unwrap();

        let resolved = resolve_plugin_source_dir(eco_dir, "pua");
        assert_eq!(resolved, fw_dir);
    }
}
