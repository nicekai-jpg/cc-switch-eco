use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem_framework;
use super::cmd_utils::{augmented_path_for_subprocess, get_command_path};
use super::plugin_sync::{
    expose_plugin_dirs_to_eco_isolation,
    merge_claude_plugins_into_eco,
    normalize_plugin_install_paths,
    merge_claude_plugin_settings_to_eco,
};
use super::plugin_ops::{
    register_plugin_to_installed_plugins,
    enable_plugin_in_settings,
    ensure_settings_json_in_isolated_files,
    extract_github_repo,
    resolve_plugin_hooks_dir,
    cleanup_orphan_plugin_skill_dirs,
};
use super::hook_ops::{
    inject_plugin_hooks_to_settings,
    remove_plugin_hooks_from_settings_fragment,
};

/// 安装前校验 hook 交付方式是否与源码一致
///
/// hooks 命令引用 ${CLAUDE_PLUGIN_ROOT} 时，必须走 plugin 安装；
/// 否则 merge 到 settings 后会被 sanitize_hooks_for_global_settings 全部剥离。
pub fn validate_hook_delivery(
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    let hooks_json_path = if fw_dir.join("hooks").join("hooks.json").exists() {
        Some(fw_dir.join("hooks").join("hooks.json"))
    } else if fw_dir
        .join(".claude-plugin")
        .join("hooks")
        .join("hooks.json")
        .exists()
    {
        Some(
            fw_dir
                .join(".claude-plugin")
                .join("hooks")
                .join("hooks.json"),
        )
    } else {
        None
    };

    let Some(path) = hooks_json_path else {
        if framework.hook_delivery == "plugin" {
            log::info!(
                "框架「{}」声明 hook_delivery=plugin，但源码中未找到 hooks/hooks.json（跳过 hook 校验）",
                framework.name
            );
        }
        return Ok(());
    };

    let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    let uses_plugin_root = content.contains("${CLAUDE_PLUGIN_ROOT}");

    if uses_plugin_root && framework.hook_delivery == "plugin" && framework.install_method != "plugin"
    {
        return Err(AppError::Message(format!(
            "框架「{}」的 hooks 依赖 Claude Code 插件路径（${{CLAUDE_PLUGIN_ROOT}}），\
             不能使用「{}」方式安装。请在 FrameworkRegistry 中改用 install_method=plugin 并配置 marketplace_name。",
            framework.name, framework.install_method
        )));
    }

    if uses_plugin_root
        && framework.hook_delivery != "plugin"
        && framework.install_method != "plugin"
    {
        log::warn!(
            "框架「{}」源码 hooks.json 含 ${{CLAUDE_PLUGIN_ROOT}}，但 hook_delivery={}。\
             若官方安装器未自行写入 settings，合并后 hook 将被剥离。",
            framework.name,
            framework.hook_delivery
        );
    }

    if framework.hook_delivery == "plugin" {
        if framework.install_method != "plugin" {
            return Err(AppError::Message(format!(
                "框架「{}」的 hook_delivery=plugin，但 install_method 为「{}」",
                framework.name, framework.install_method
            )));
        }
        if framework.marketplace_name.is_none() {
            return Err(AppError::Message(format!(
                "框架「{}」的 hook_delivery=plugin，但未配置 marketplace_name",
                framework.name
            )));
        }
    }

    Ok(())
}

/// plugin 安装完成后验证 hooks 脚本已就位
pub fn verify_plugin_hooks_installed(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    if framework.hook_delivery != "plugin" {
        return Ok(());
    }

    let plugin_key = ecosystem_framework::framework_plugin_key(framework).ok_or_else(|| {
        AppError::Message(format!("框架 '{}' 缺少 marketplace_name", framework.id))
    })?;

    let plugins_dir = eco_dir.join("plugins");
    let installed_plugins_path = plugins_dir.join("installed_plugins.json");
    if !installed_plugins_path.exists() {
        return Err(AppError::Message(format!(
            "框架「{}」plugin 安装后缺少 installed_plugins.json",
            framework.name
        )));
    }

    let content = fs::read_to_string(&installed_plugins_path)
        .map_err(|e| AppError::io(&installed_plugins_path, e))?;
    let installed: serde_json::Value =
        fragment::parse_json(&content, "解析 installed_plugins.json 失败")?;

    let has_plugin = installed
        .get("plugins")
        .and_then(|p| p.get(&plugin_key))
        .is_some();
    if !has_plugin {
        return Err(AppError::Message(format!(
            "框架「{}」未在 installed_plugins.json 中注册（期望 key: {plugin_key}）",
            framework.name
        )));
    }

    let user_frag_path =
        fragment::fragment_path(&eco_dir.join("rootfiles"), "settings.json", "user-");
    if !user_frag_path.exists() {
        return Err(AppError::Message(format!(
            "框架「{}」plugin 安装后缺少 user-fragment（enabledPlugins 未写入）",
            framework.name
        )));
    }

    let user_content =
        fs::read_to_string(&user_frag_path).map_err(|e| AppError::io(&user_frag_path, e))?;
    let user_frag: serde_json::Value =
        fragment::parse_json(&user_content, "解析 user-fragment 失败")?;
    let enabled = user_frag
        .get("enabledPlugins")
        .and_then(|ep| ep.get(&plugin_key))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !enabled {
        return Err(AppError::Message(format!(
            "框架「{}」未在 enabledPlugins 中启用（key: {plugin_key}）",
            framework.name
        )));
    }

    // 验证 cache installPath 下 hooks 脚本存在（仅当框架提供 hooks/ 目录时）
    if framework.provided_dirs.contains(&"hooks".to_string()) {
        if let Some(entries) = installed
            .get("plugins")
            .and_then(|p| p.get(&plugin_key))
            .and_then(|v| v.as_array())
        {
            if let Some(install_path) = entries
                .first()
                .and_then(|e| e.get("installPath"))
                .and_then(|v| v.as_str())
            {
                let hooks_dir = resolve_plugin_hooks_dir(eco_dir, install_path);
                if !hooks_dir.is_dir() {
                    return Err(AppError::Message(format!(
                        "框架「{}」plugin 安装路径缺少 hooks 目录: {}",
                        framework.name,
                        hooks_dir.display()
                    )));
                }
            }
        }
    }

    log::info!("框架「{}」plugin hooks 校验通过", framework.name);
    Ok(())
}

/// 是否优先使用 Claude Code 官方 plugin CLI（HOME 重定向）安装
pub fn should_use_claude_plugin_cli(framework: &ecosystem_framework::FrameworkRegistry) -> bool {
    framework.install_method == "plugin"
        && framework.marketplace_name.is_some()
        // claude-hud 需要自定义 statusLine / config.json，走手动注册流程
        && framework.id != "claude-hud"
        // GSD 等由 npx 安装器直接写入 settings（绝对路径），不走 plugin CLI
        && framework.hook_delivery != "settings"
}

/// 使用 Claude Code 官方 plugin CLI 安装（HOME 重定向到 eco_dir，与 npx 同模式）
pub fn install_via_claude_plugin_command(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    _fw_dir: &Path,
) -> Result<(), AppError> {
    let marketplace_name = framework
        .marketplace_name
        .as_ref()
        .ok_or_else(|| AppError::Message(format!("框架 '{}' 未配置 marketplace_name", framework.id)))?;

    let claude_bin = get_command_path("claude").ok_or_else(|| {
        AppError::Message("未找到 claude CLI，无法执行官方 plugin 安装".to_string())
    })?;

    let eco_claude_dir = eco_dir.join(".claude");
    fs::create_dir_all(eco_claude_dir.join("plugins"))
        .map_err(|e| AppError::io(eco_claude_dir.join("plugins"), e))?;

    let repo = extract_github_repo(&framework.repo_url);
    run_claude_plugin_cli(eco_dir, &claude_bin, &["plugin", "marketplace", "add", &repo])?;

    let plugin_spec = format!(
        "{}@{marketplace_name}",
        ecosystem_framework::framework_plugin_name(framework)
    );
    run_claude_plugin_cli(eco_dir, &claude_bin, &["plugin", "install", &plugin_spec])?;

    let src_plugins = eco_claude_dir.join("plugins");
    if !src_plugins.join("installed_plugins.json").exists() {
        return Err(AppError::Message(format!(
            "官方 plugin 安装未生成 installed_plugins.json（框架: {}）",
            framework.id
        )));
    }

    merge_claude_plugins_into_eco(&src_plugins, &eco_dir.join("plugins"))?;
    normalize_plugin_install_paths(eco_dir)?;

    let settings_path = eco_claude_dir.join("settings.json");
    merge_claude_plugin_settings_to_eco(eco_dir, &settings_path)?;

    finalize_plugin_framework_install(eco_dir, framework)?;

    if eco_claude_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&eco_claude_dir) {
            log::warn!("清理临时 .claude 目录失败 {}: {e}", eco_claude_dir.display());
        }
    }

    log::info!(
        "官方 claude plugin 安装成功: {} (marketplace: {})",
        plugin_spec,
        marketplace_name
    );
    Ok(())
}

pub fn run_claude_plugin_cli(eco_dir: &Path, claude_bin: &str, args: &[&str]) -> Result<(), AppError> {
    let output = Command::new(claude_bin)
        .args(args)
        .env("HOME", eco_dir)
        .env("PATH", augmented_path_for_subprocess())
        .current_dir(eco_dir)
        .output()
        .map_err(|e| AppError::Message(format!("执行 claude 命令失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(AppError::Message(format!(
            "claude 命令执行失败 ({:?}):\nstdout: {stdout}\nstderr: {stderr}",
            args
        )));
    }
    Ok(())
}

/// 官方 plugin CLI 失败时，从 git 源码注册完整 plugin（cache + enabledPlugins）
pub fn install_plugin_from_git_source(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    register_plugin_to_installed_plugins(eco_dir, framework)?;
    finalize_plugin_framework_install(eco_dir, framework)?;
    log::info!(
        "已从 git 源码完成 plugin 注册: {} ({})",
        framework.id,
        ecosystem_framework::framework_plugin_key(framework).unwrap_or_default()
    );
    Ok(())
}

/// plugin 安装收尾：暴露 plugin cache 中的 skill/command/agent/hooks 到隔离目录、注入 plugin hooks 到 settings、清理孤立 skills 副本
pub fn finalize_plugin_framework_install(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    expose_plugin_dirs_to_eco_isolation(eco_dir, framework)?;
    let hooks_injected = inject_plugin_hooks_to_settings(eco_dir, framework);
    if !hooks_injected.is_ok() {
        remove_plugin_hooks_from_settings_fragment(eco_dir, &framework.file_prefix);
    }
    cleanup_orphan_plugin_skill_dirs(eco_dir, framework);
    ensure_settings_json_in_isolated_files(eco_dir)?;
    fragment::rebuild_all_root_files(eco_dir)?;
    Ok(())
}
