use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::fs_utils;
use crate::services::ecosystem::symlink;
use crate::services::ecosystem_framework;

use super::cmd_utils::{command_exists, get_command_path, uv_has_python_311};
use super::plugin_install::{
    should_use_claude_plugin_cli, install_via_claude_plugin_command,
    install_plugin_from_git_source, verify_plugin_hooks_installed,
    finalize_plugin_framework_install
};
use super::plugin_ops::register_plugin_to_installed_plugins;

/// 执行框架安装（官方命令 + 手动复制回退）
pub fn do_install(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    let mut used_official_claude_plugin = false;

    let install_result = match framework.install_method.as_str() {
        "npx" | "script" => install_via_official_command(eco_dir, framework, fw_dir),
        "uv" => install_via_uv_command(eco_dir, framework, fw_dir),
        "plugin" if should_use_claude_plugin_cli(framework) => {
            match install_via_claude_plugin_command(eco_dir, framework, fw_dir) {
                Ok(()) => {
                    used_official_claude_plugin = true;
                    Ok(())
                }
                Err(e) => {
                    log::warn!("官方 claude plugin 安装失败: {e}，回退到 git 源码 plugin 注册");
                    install_plugin_from_git_source(eco_dir, framework)
                }
            }
        }
        "plugin" | "copy" => install_manual_copy(eco_dir, framework, fw_dir),
        _ => Err(AppError::Message(format!(
            "未知的安装方式: {}",
            framework.install_method
        ))),
    };

    // 非 plugin-CLI 路径：官方命令失败时回退到手动复制
    let result = match install_result {
        Ok(()) => Ok(()),
        Err(e) if framework.install_method == "npx" || framework.install_method == "script" => {
            log::warn!("官方安装命令失败: {e}，回退到手动复制");
            install_manual_copy(eco_dir, framework, fw_dir)
        }
        Err(e) => Err(e),
    };

    // 安装完成后，将框架的 hooks/hooks.json 合并到 settings fragment
    // plugin 类型框架的 hooks 依赖 ${CLAUDE_PLUGIN_ROOT}，由 Claude Code 插件系统执行，不能写入全局 settings
    // GSD 由官方 npx 安装器直接写入 settings（绝对路径），跳过 git hooks.json 合并
    if result.is_ok()
        && framework.install_method != "plugin"
        && framework.id != "get-shit-done"
    {
        merge_hooks_json_to_fragment(eco_dir, fw_dir, &framework.file_prefix)?;
    }

    // 对于 plugin 类型框架，手动路径需注册；官方 CLI 路径已在 merge_claude_plugins_into_eco 中完成
    if result.is_ok() && framework.install_method == "plugin" {
        if !used_official_claude_plugin {
            // git 回退路径已在 install_plugin_from_git_source 中注册
            if !should_use_claude_plugin_cli(framework) {
                register_plugin_to_installed_plugins(eco_dir, framework)?;
                finalize_plugin_framework_install(eco_dir, framework)?;
            }
        }
        verify_plugin_hooks_installed(eco_dir, framework)?;
    }

    result
}

/// 使用官方命令安装框架（npx / script 方式）
pub fn install_via_official_command(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    // 在 Eco 目录下创建 .claude/ 目录结构，供 HOME 重定向使用
    let eco_claude_dir = eco_dir.join(".claude");
    for sub_dir in &["skills", "agents", "commands", "hooks", "plugins"] {
        fs::create_dir_all(eco_claude_dir.join(sub_dir))
            .map_err(|e| AppError::io(eco_claude_dir.join(sub_dir), e))?;
    }
    for isolated_dir in &framework.isolated_dirs {
        fs::create_dir_all(eco_claude_dir.join(isolated_dir))
            .map_err(|e| AppError::io(eco_claude_dir.join(isolated_dir), e))?;
    }

    // 运行官方安装命令
    let real_home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
    let result = match framework.install_method.as_str() {
        "npx" => run_npx_command(eco_dir, framework, &real_home),
        "script" => run_script_command(eco_dir, framework, fw_dir, &real_home),
        _ => Err(AppError::Message(format!(
            "不支持的安装方式: {}",
            framework.install_method
        ))),
    };

    if let Err(e) = result {
        if let Err(e) = fs::remove_dir_all(&eco_claude_dir) {
            log::warn!("清理临时目录失败 {}: {e}", eco_claude_dir.display());
        }
        return Err(AppError::Message(format!("官方安装命令失败: {e}")));
    }

    // 将 .claude/ 中的文件移动到 Eco 对应目录
    move_claude_files_to_eco(&eco_claude_dir, eco_dir, framework)?;

    // 清理 Eco 的 .claude/ 目录
    if let Err(e) = fs::remove_dir_all(&eco_claude_dir) {
        log::warn!("清理临时目录失败 {}: {e}", eco_claude_dir.display());
    }

    Ok(())
}

/// 将 Eco 的 .claude/ 目录下的文件移动到 Eco 对应目录
pub fn move_claude_files_to_eco(
    eco_claude_dir: &Path,
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    let isolation = fragment::collect_eco_isolation(eco_dir);

    move_isolated_dirs(eco_claude_dir, eco_dir, framework, &isolation)?;

    // 官方 npx 安装器将 hooks 命令写成 {eco_dir}/.claude/hooks/ 绝对路径；
    // move 后 hooks 位于 eco_dir/hooks/，切换生态时 ~/.claude/hooks 通过 symlink 映射。
    if framework.hook_delivery == "settings" {
        rewrite_installer_hook_paths_in_claude_settings(eco_claude_dir, eco_dir)?;
    }

    move_isolated_rootfiles(eco_claude_dir, eco_dir, &framework.file_prefix, &isolation)?;
    copy_non_isolated_files(eco_claude_dir, eco_dir, &framework.file_prefix, &isolation)?;

    fragment::rebuild_all_root_files(eco_dir)?;
    Ok(())
}

/// 移动隔离目录中的文件（skills/commands/hooks/agents/plugins 等）
///
/// 根据 framework 的 dir_layout 策略和 files_prefixed 字段通用处理。
fn move_isolated_dirs(
    eco_claude_dir: &Path,
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    isolation: &fragment::EcoIsolation,
) -> Result<(), AppError> {
    let prefix = &framework.file_prefix;
    let strategy = framework.dir_layout.strategy();

    for dir_name in &isolation.dirs {
        let src_dir = eco_claude_dir.join(dir_name);
        if !src_dir.exists() || !src_dir.is_dir() {
            continue;
        }
        let dst_dir = eco_dir.join(dir_name);
        fs::create_dir_all(&dst_dir).map_err(|e| AppError::io(&dst_dir, e))?;

        strategy.move_from_claude(&src_dir, &dst_dir, prefix, framework.files_prefixed)?;
    }
    Ok(())
}

/// 将 settings.json 中 hooks 命令路径从临时 `.claude/hooks/` 改写为 `~/.claude/hooks/`
fn rewrite_installer_hook_paths_in_claude_settings(
    eco_claude_dir: &Path,
    eco_dir: &Path,
) -> Result<(), AppError> {
    let settings_path = eco_claude_dir.join("settings.json");
    if !settings_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&settings_path).map_err(|e| AppError::io(&settings_path, e))?;
    let mut settings: serde_json::Value =
        fragment::parse_json(&content, "解析 settings.json 失败")?;

    let old_hooks_dir = path_prefix_for_rewrite(&eco_dir.join(".claude").join("hooks"));
    let new_hooks_dir = path_prefix_for_rewrite(&crate::config::get_claude_config_dir().join("hooks"));

    if let Some(hooks) = settings.get_mut("hooks") {
        rewrite_hook_paths_in_json(hooks, &old_hooks_dir, &new_hooks_dir);
    }

    fs::write(&settings_path, fragment::write_json(&settings)?)
        .map_err(|e| AppError::io(&settings_path, e))?;

    log::info!(
        "已重写 settings hooks 路径: {} → {}",
        old_hooks_dir,
        new_hooks_dir
    );
    Ok(())
}

fn path_prefix_for_rewrite(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches('/').to_string()
}

fn rewrite_hook_paths_in_json(
    value: &mut serde_json::Value,
    old_prefix: &str,
    new_prefix: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "command" {
                    if let Some(command) = child.as_str() {
                        *child = serde_json::Value::String(rewrite_hook_command_path(
                            command, old_prefix, new_prefix,
                        ));
                    }
                } else {
                    rewrite_hook_paths_in_json(child, old_prefix, new_prefix);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                rewrite_hook_paths_in_json(item, old_prefix, new_prefix);
            }
        }
        _ => {}
    }
}

pub fn rewrite_hook_command_path(command: &str, old_prefix: &str, new_prefix: &str) -> String {
    if old_prefix.is_empty() || !command.contains(old_prefix) {
        return command.to_string();
    }
    command.replace(old_prefix, new_prefix)
}

/// 移动隔离根文件（settings.json/CLAUDE.md 等）到 rootfiles 目录
fn move_isolated_rootfiles(
    eco_claude_dir: &Path,
    eco_dir: &Path,
    prefix: &str,
    isolation: &fragment::EcoIsolation,
) -> Result<(), AppError> {
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

    for file_name in &isolation.files {
        let src_path = eco_claude_dir.join(file_name);
        if !src_path.exists() || !src_path.is_file() {
            continue;
        }
        let dst_path = rootfiles_dir.join(file_name);
        if dst_path.exists() {
            fragment::merge_root_file(&src_path, &dst_path, prefix)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
        }
    }
    Ok(())
}

/// 复制非隔离根文件到 Eco 根目录（带前缀）
fn copy_non_isolated_files(
    eco_claude_dir: &Path,
    eco_dir: &Path,
    prefix: &str,
    isolation: &fragment::EcoIsolation,
) -> Result<(), AppError> {
    if let Ok(entries) = fs::read_dir(eco_claude_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.')
                || entry.path().is_dir()
                || isolation.files.contains(&name)
                || symlink::BASE_ISOLATED_DIRS.contains(&name.as_str())
                || isolation.dirs.contains(&name)
            {
                continue;
            }
            let dst_name = format!("{prefix}{name}");
            let dst_path = eco_dir.join(&dst_name);
            if entry.path().is_file() && !dst_path.exists() {
                if let Err(e) = fs::copy(entry.path(), &dst_path) {
                    log::warn!("复制文件失败: {e}");
                }
            }
        }
    }
    Ok(())
}

/// 将框架的 hooks/hooks.json 合并到 settings fragment
pub fn merge_hooks_json_to_fragment(
    eco_dir: &Path,
    fw_dir: &Path,
    prefix: &str,
) -> Result<(), AppError> {
    let hooks_json_path = if fw_dir.join("hooks").join("hooks.json").exists() {
        fw_dir.join("hooks").join("hooks.json")
    } else if fw_dir
        .join(".claude-plugin")
        .join("hooks")
        .join("hooks.json")
        .exists()
    {
        fw_dir
            .join(".claude-plugin")
            .join("hooks")
            .join("hooks.json")
    } else {
        return Ok(());
    };

    let content =
        fs::read_to_string(&hooks_json_path).map_err(|e| AppError::io(&hooks_json_path, e))?;
    let hooks_json: serde_json::Value = fragment::parse_json(&content, "解析 hooks.json 失败")?;

    let hooks_field = match hooks_json.get("hooks") {
        Some(h) => h.clone(),
        None => return Ok(()),
    };

    let fragment_content = serde_json::json!({ "hooks": hooks_field });

    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

    // 如果 settings.json 不在隔离列表中，需要添加（hooks 配置需要写入 settings fragment）
    let isolation = fragment::collect_eco_isolation(eco_dir);
    if !isolation.files.contains(&"settings.json".to_string()) {
        let mut updated_files = isolation.files.clone();
        updated_files.push("settings.json".to_string());
        let updated_isolation = fragment::EcoIsolation {
            dirs: isolation.dirs,
            files: updated_files,
        };
        fragment::update_eco_json_isolation(eco_dir, &updated_isolation)?;
    }

    let frag_path = fragment::fragment_path(&rootfiles_dir, "settings.json", prefix);

    if frag_path.exists() {
        let existing = fs::read_to_string(&frag_path).map_err(|e| AppError::io(&frag_path, e))?;
        let mut existing_json: serde_json::Value =
            fragment::parse_json(&existing, "解析 fragment 失败")?;

        let mut conflicts = Vec::new();
        fragment::json_deep_merge_with_array_dedup(
            &mut existing_json,
            &fragment_content,
            "",
            prefix,
            &mut conflicts,
        );

        fs::write(&frag_path, fragment::write_json(&existing_json)?)
            .map_err(|e| AppError::io(&frag_path, e))?;
    } else {
        fs::write(&frag_path, fragment::write_json(&fragment_content)?)
            .map_err(|e| AppError::io(&frag_path, e))?;
    }

    // 从 fragment 重建所有 JSON 根文件
    fragment::rebuild_all_root_files(eco_dir)?;

    log::info!("已将 hooks.json 合并到 {}", frag_path.display());
    Ok(())
}

/// 运行 npx 安装命令
pub fn run_npx_command(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    _real_home: &Path,
) -> Result<(), AppError> {
    let command = framework
        .install_command
        .as_deref()
        .ok_or_else(|| AppError::Message(format!("框架 '{}' 未配置安装命令", framework.id)))?;

    let args: Vec<String> = framework
        .install_args
        .iter()
        .map(|arg| resolve_template(arg, eco_dir, _real_home))
        .collect();

    let output = Command::new(command)
        .args(&args)
        .env("HOME", eco_dir)
        .current_dir(eco_dir)
        .output()
        .map_err(|e| AppError::Message(format!("执行 npx 命令失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(AppError::Message(format!(
            "npx 命令执行失败:\nstdout: {stdout}\nstderr: {stderr}"
        )));
    }

    log::info!("npx 命令执行成功: {} {:?}", command, args);
    Ok(())
}

/// 运行脚本安装命令
pub fn run_script_command(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
    _real_home: &Path,
) -> Result<(), AppError> {
    let script_relative = framework
        .install_command
        .as_deref()
        .ok_or_else(|| AppError::Message(format!("框架 '{}' 未配置安装脚本", framework.id)))?;

    let script_path = fw_dir.join(script_relative);
    if !script_path.exists() {
        return Err(AppError::Message(format!(
            "安装脚本不存在: {}",
            script_path.display()
        )));
    }

    let args: Vec<String> = framework
        .install_args
        .iter()
        .map(|arg| resolve_template(arg, eco_dir, _real_home))
        .collect();

    let mut cmd = Command::new("bash");
    cmd.arg(&script_path)
        .args(&args)
        .env("HOME", eco_dir)
        .current_dir(fw_dir);

    for (key, value) in &framework.install_env {
        let resolved = resolve_template(value, eco_dir, _real_home);
        cmd.env(key, resolved);
    }

    let output = cmd
        .output()
        .map_err(|e| AppError::Message(format!("执行脚本失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(AppError::Message(format!(
            "脚本执行失败:\nstdout: {stdout}\nstderr: {stderr}"
        )));
    }

    log::info!("脚本执行成功: {}", script_path.display());
    Ok(())
}

/// 使用 uv 工具安装框架（如 Spec Kit 的 specify-cli）
pub fn install_via_uv_command(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    _fw_dir: &Path,
) -> Result<(), AppError> {
    let real_home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));

    // 在 Eco 目录下创建 .claude/ 目录结构，供 HOME 重定向使用
    let eco_claude_dir = eco_dir.join(".claude");
    for sub_dir in &["skills", "agents", "commands", "hooks", "plugins"] {
        fs::create_dir_all(eco_claude_dir.join(sub_dir))
            .map_err(|e| AppError::io(eco_claude_dir.join(sub_dir), e))?;
    }
    for isolated_dir in &framework.isolated_dirs {
        fs::create_dir_all(eco_claude_dir.join(isolated_dir))
            .map_err(|e| AppError::io(eco_claude_dir.join(isolated_dir), e))?;
    }

    // Step 1: uv tool install 安装 CLI
    let command = framework
        .install_command
        .as_deref()
        .ok_or_else(|| AppError::Message(format!("框架 '{}' 未配置 uv 安装命令", framework.id)))?;

    let args: Vec<String> = framework
        .install_args
        .iter()
        .map(|arg| resolve_template(arg, eco_dir, &real_home))
        .collect();

    let output = Command::new(command)
        .args(&args)
        .env("HOME", &real_home)
        .current_dir(eco_dir)
        .output()
        .map_err(|e| AppError::Message(format!("执行 uv 命令失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Err(e) = fs::remove_dir_all(&eco_claude_dir) {
            log::warn!("清理临时目录失败 {}: {e}", eco_claude_dir.display());
        }
        return Err(AppError::Message(format!(
            "uv tool install 失败:\nstdout: {stdout}\nstderr: {stderr}"
        )));
    }

    log::info!("uv tool install 成功: {} {:?}", command, args);

    // Step 2: 运行 CLI init 命令（HOME 重定向到 eco_dir）
    let init_output = Command::new("uv")
        .args(["tool", "run", "specify", "init", ".", "--integration", "claude", "--force"])
        .env("HOME", eco_dir)
        .current_dir(eco_dir)
        .output()
        .map_err(|e| AppError::Message(format!("执行 specify init 失败: {e}")))?;

    if !init_output.status.success() {
        let stderr = String::from_utf8_lossy(&init_output.stderr);
        let stdout = String::from_utf8_lossy(&init_output.stdout);
        log::warn!("specify init 失败（将回退到手动复制）:\nstdout: {stdout}\nstderr: {stderr}");
        if let Err(e) = fs::remove_dir_all(&eco_claude_dir) {
            log::warn!("清理临时目录失败 {}: {e}", eco_claude_dir.display());
        }
        return Err(AppError::Message(format!(
            "specify init 失败:\nstdout: {stdout}\nstderr: {stderr}"
        )));
    }

    // Step 3: 将 .claude/ 中的文件移动 to Eco 对应目录
    move_claude_files_to_eco(&eco_claude_dir, eco_dir, framework)?;

    // 清理 Eco 的 .claude/ 目录
    if let Err(e) = fs::remove_dir_all(&eco_claude_dir) {
        log::warn!("清理临时目录失败 {}: {e}", eco_claude_dir.display());
    }

    Ok(())
}

/// 解析模板变量
pub fn resolve_template(template: &str, eco_dir: &Path, real_home: &Path) -> String {
    template
        .replace("{eco_dir}", eco_dir.to_str().unwrap_or(""))
        .replace("{real_home}", real_home.to_str().unwrap_or(""))
}

/// 手动复制框架文件到 Eco 目录
pub fn install_manual_copy(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    let prefix = &framework.file_prefix;
    let strategy = framework.dir_layout.strategy();

    // 检测是否有 .claude-plugin → plugins/{id} 的映射
    let plugin_dir_mapping: Option<String> = framework
        .dir_mappings
        .iter()
        .find(|(src_name, _)| src_name == ".claude-plugin")
        .map(|(_, dst)| dst.replace("{id}", &framework.id));

    for dir_name in &framework.provided_dirs {
        let src = fw_dir.join(dir_name);
        if !src.exists() {
            continue;
        }

        // 检查 dir_mappings：非标准目录映射
        if let Some(mapping) = framework
            .dir_mappings
            .iter()
            .find(|(src_name, _)| src_name == dir_name)
        {
            let dst = eco_dir.join(mapping.1.replace("{id}", &framework.id));
            fs_utils::copy_dir_recursive(&src, &dst)?;
            continue;
        }

        if !src.is_dir() {
            continue;
        }

        // 对于 plugin 类型框架，将非 .claude-plugin 的 provided_dirs 也复制到插件目录内
        if framework.install_method == "plugin" {
            if let Some(ref plugin_dst) = plugin_dir_mapping {
                let plugin_commands_dir = eco_dir.join(plugin_dst).join(dir_name);
                if !plugin_commands_dir.exists() {
                    fs_utils::copy_dir_recursive(&src, &plugin_commands_dir)?;
                }
            }
        }

        let dst = eco_dir.join(dir_name);
        fs::create_dir_all(&dst).map_err(|e| AppError::io(&dst, e))?;

        strategy.copy_to_eco(&src, &dst, prefix, framework.files_prefixed)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_hook_command_path() {
        let old = "/tmp/eco/.claude/hooks";
        let new = "/Users/me/.claude/hooks";
        let cmd = r#""/opt/homebrew/bin/node" "/tmp/eco/.claude/hooks/gsd-check-update.js""#;
        let rewritten = rewrite_hook_command_path(cmd, old, new);
        assert!(rewritten.contains("/Users/me/.claude/hooks/gsd-check-update.js"));
        assert!(!rewritten.contains("/tmp/eco/.claude/hooks"));
    }

    #[test]
    fn test_rewrite_installer_hook_paths_in_settings() {
        let dir = tempfile::tempdir().unwrap();
        let eco_dir = dir.path();
        let eco_claude = eco_dir.join(".claude");
        fs::create_dir_all(&eco_claude).unwrap();

        let old_hooks = eco_claude.join("hooks");
        let settings = serde_json::json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!("bash \"{}/gsd-session-state.sh\"", old_hooks.display())
                    }]
                }]
            }
        });
        fs::write(
            eco_claude.join("settings.json"),
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .unwrap();

        rewrite_installer_hook_paths_in_claude_settings(&eco_claude, eco_dir).unwrap();

        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(eco_claude.join("settings.json")).unwrap())
                .unwrap();
        let cmd = updated["hooks"]["SessionStart"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        let expected_prefix = crate::config::get_claude_config_dir().join("hooks");
        assert!(
            cmd.contains(&expected_prefix.to_string_lossy().to_string()),
            "expected {cmd} to contain {}",
            expected_prefix.display()
        );
    }
}
