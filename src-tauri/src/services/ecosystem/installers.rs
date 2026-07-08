use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::fs_utils;
use crate::services::ecosystem_framework;
use super::plugin_install::{
    should_use_claude_plugin_cli, install_via_claude_plugin_command,
    install_plugin_from_git_source, verify_plugin_hooks_installed,
    finalize_plugin_framework_install
};
use super::plugin_ops::register_plugin_to_installed_plugins;
use super::install_utils::{move_claude_files_to_eco, resolve_template};

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
