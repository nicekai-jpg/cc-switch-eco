use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::fs_utils;
use crate::services::ecosystem::symlink;
use crate::services::ecosystem_framework;
use crate::store::AppState;

fn ensure_eco_exists(state: &AppState, eco_id: &str) -> Result<(), AppError> {
    if !state.db.ecosystem_exists(eco_id)? {
        return Err(AppError::Message(format!("生态 '{eco_id}' 不存在")));
    }
    Ok(())
}

/// 安装框架到指定生态
pub fn install_framework(
    state: &AppState,
    eco_id: &str,
    framework_id: &str,
) -> Result<(), AppError> {
    let framework = ecosystem_framework::find_framework(framework_id)
        .ok_or_else(|| AppError::Message(format!("框架 '{framework_id}' 不存在")))?;

    ensure_eco_exists(state, eco_id)?;

    let eco_dir = super::ecosystem_dir(eco_id);
    let fw_dir = eco_dir.join("frameworks").join(framework_id);

    if fw_dir.exists() {
        return Err(AppError::Message(format!(
            "框架 '{framework_id}' 已安装在生态 '{eco_id}' 中"
        )));
    }

    // git clone 获取源码
    fs::create_dir_all(fw_dir.parent().unwrap()).map_err(|e| AppError::io(&fw_dir, e))?;

    let output = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            &framework.repo_branch,
            &framework.repo_url,
            fw_dir.to_str().unwrap_or(""),
        ])
        .output()
        .map_err(|e| AppError::Message(format!("执行 git clone 失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Err(e) = fs::remove_dir_all(&fw_dir) {
            log::warn!("清理失败的 clone 目录 {}: {e}", fw_dir.display());
        }
        return Err(AppError::Message(format!("git clone 失败: {stderr}")));
    }

    // 安装文件到 Eco 隔离目录
    do_install(&eco_dir, &framework, &fw_dir)?;

    // 更新 eco.json
    let commit_hash = get_git_commit_hash(&fw_dir).unwrap_or_default();
    add_framework_to_eco_json(&eco_dir, framework_id, &commit_hash)?;

    // 更新隔离列表
    let isolation = fragment::collect_eco_isolation(&eco_dir);
    fragment::update_eco_json_isolation(&eco_dir, &isolation)?;

    log::info!("框架 '{framework_id}' 已安装到生态 '{eco_id}'");
    Ok(())
}

/// 卸载框架
pub fn uninstall_framework(
    state: &AppState,
    eco_id: &str,
    framework_id: &str,
) -> Result<(), AppError> {
    ensure_eco_exists(state, eco_id)?;

    let eco_dir = super::ecosystem_dir(eco_id);
    let fw_dir = eco_dir.join("frameworks").join(framework_id);

    if !fw_dir.exists() {
        return Err(AppError::Message(format!(
            "框架 '{framework_id}' 未安装在生态 '{eco_id}' 中"
        )));
    }

    let framework = ecosystem_framework::find_framework(framework_id);
    let prefix = framework
        .as_ref()
        .map(|f| f.file_prefix.as_str())
        .unwrap_or(framework_id);

    uninstall_by_prefix(&eco_dir, prefix, framework_id)?;

    // 删除框架 git 仓库
    fs::remove_dir_all(&fw_dir).map_err(|e| AppError::io(&fw_dir, e))?;

    // 更新 eco.json
    remove_framework_from_eco_json(&eco_dir, framework_id)?;

    log::info!("框架 '{framework_id}' 已从生态 '{eco_id}' 卸载");
    Ok(())
}

/// 更新框架（git pull + 重新安装）
pub fn update_framework(
    state: &AppState,
    eco_id: &str,
    framework_id: &str,
) -> Result<(), AppError> {
    ensure_eco_exists(state, eco_id)?;

    let eco_dir = super::ecosystem_dir(eco_id);
    let fw_dir = eco_dir.join("frameworks").join(framework_id);

    if !fw_dir.exists() {
        return Err(AppError::Message(format!(
            "框架 '{framework_id}' 未安装在生态 '{eco_id}' 中"
        )));
    }

    // git pull 更新源码
    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(&fw_dir)
        .output()
        .map_err(|e| AppError::Message(format!("执行 git pull 失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Message(format!("git pull 失败: {stderr}")));
    }

    // 先卸载旧文件，再重新安装
    let framework = ecosystem_framework::find_framework(framework_id)
        .ok_or_else(|| AppError::Message(format!("框架 '{framework_id}' 不存在")))?;

    let prefix = framework.file_prefix.as_str();
    uninstall_by_prefix(&eco_dir, prefix, framework_id)?;

    // 重新安装
    do_install(&eco_dir, &framework, &fw_dir)?;

    // 更新 eco.json
    let commit_hash = get_git_commit_hash(&fw_dir).unwrap_or_default();
    add_framework_to_eco_json(&eco_dir, framework_id, &commit_hash)?;

    // 更新隔离列表
    let isolation = fragment::collect_eco_isolation(&eco_dir);
    fragment::update_eco_json_isolation(&eco_dir, &isolation)?;

    // 从 fragment 重建所有 JSON 根文件
    fragment::rebuild_all_root_files(&eco_dir)?;

    log::info!("框架 '{framework_id}' 在生态 '{eco_id}' 中已更新");
    Ok(())
}

/// 获取生态已安装的框架列表
pub fn get_ecosystem_frameworks(eco_id: &str) -> Result<Vec<String>, AppError> {
    let eco_dir = super::ecosystem_dir(eco_id);
    let eco_json_path = eco_dir.join("eco.json");

    if !eco_json_path.exists() {
        return Ok(vec![]);
    }

    let content =
        fs::read_to_string(&eco_json_path).map_err(|e| AppError::io(&eco_json_path, e))?;
    let json: serde_json::Value = fragment::parse_json(&content, "解析 eco.json 失败")?;

    let frameworks = json
        .get("frameworks")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(frameworks)
}

// ================================================================
// 内部实现
// ================================================================

/// 执行框架安装（官方命令 + 手动复制回退）
fn do_install(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    let install_result = match framework.install_method.as_str() {
        "npx" | "script" => install_via_official_command(eco_dir, framework, fw_dir),
        "plugin" | "copy" => install_manual_copy(eco_dir, framework, fw_dir),
        _ => Err(AppError::Message(format!(
            "未知的安装方式: {}",
            framework.install_method
        ))),
    };

    // 官方命令失败时回退到手动复制
    let result = match install_result {
        Ok(()) => Ok(()),
        Err(e) => {
            log::warn!("官方安装命令失败: {e}，回退到手动复制");
            install_manual_copy(eco_dir, framework, fw_dir)
        }
    };

    // 安装完成后，将框架的 hooks/hooks.json 合并到 settings fragment
    if result.is_ok() {
        merge_hooks_json_to_fragment(eco_dir, fw_dir, &framework.file_prefix)?;
    }

    result
}

/// 使用官方命令安装框架（npx / script 方式）
fn install_via_official_command(
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
fn move_claude_files_to_eco(
    eco_claude_dir: &Path,
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    let isolation = fragment::collect_eco_isolation(eco_dir);

    move_isolated_dirs(eco_claude_dir, eco_dir, framework, &isolation)?;
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
///
/// 框架（如 OMC）的 hooks 配置定义在源码的 hooks/hooks.json 中，
/// 但安装命令不负责将其写入 settings.json。
/// 此函数在安装完成后，将 hooks 配置合并到 settings fragment 的 hooks 字段。
fn merge_hooks_json_to_fragment(
    eco_dir: &Path,
    fw_dir: &Path,
    prefix: &str,
) -> Result<(), AppError> {
    // 按优先级查找 hooks.json：hooks/hooks.json（OMC、superpowers），
    // .claude-plugin/hooks/hooks.json（ruflo）
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
fn run_npx_command(
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
fn run_script_command(
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

/// 解析模板变量
fn resolve_template(template: &str, eco_dir: &Path, real_home: &Path) -> String {
    template
        .replace("{eco_dir}", eco_dir.to_str().unwrap_or(""))
        .replace("{real_home}", real_home.to_str().unwrap_or(""))
}

/// 手动复制框架文件到 Eco 目录
///
/// 根据 FrameworkRegistry 的 dir_layout 策略、files_prefixed、dir_mappings 通用处理。
fn install_manual_copy(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    let prefix = &framework.file_prefix;
    let strategy = framework.dir_layout.strategy();

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
            if !dst.exists() {
                fs_utils::copy_dir_recursive(&src, &dst)?;
            }
            continue;
        }

        if !src.is_dir() {
            continue;
        }

        let dst = eco_dir.join(dir_name);
        fs::create_dir_all(&dst).map_err(|e| AppError::io(&dst, e))?;

        strategy.copy_to_eco(&src, &dst, prefix, framework.files_prefixed)?;
    }

    Ok(())
}

/// 按前缀卸载框架文件
fn uninstall_by_prefix(eco_dir: &Path, prefix: &str, framework_id: &str) -> Result<(), AppError> {
    let isolation = fragment::collect_eco_isolation(eco_dir);

    // 从各隔离目录移除带前缀的文件
    for dir_name in &isolation.dirs {
        let dir = eco_dir.join(dir_name);
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(prefix) {
                    let path = entry.path();
                    if path.is_dir() {
                        fs::remove_dir_all(&path).map_err(|e| AppError::io(&path, e))?;
                    } else {
                        fs::remove_file(&path).map_err(|e| AppError::io(&path, e))?;
                    }
                }
            }
        }
    }

    // 从 rootfiles 中移除框架写入的根文件内容
    let rootfiles_dir = eco_dir.join("rootfiles");
    if rootfiles_dir.exists() {
        let framework = ecosystem_framework::find_framework(framework_id);
        if let Some(fw) = framework {
            for file_name in &fw.isolated_files {
                let file_path = rootfiles_dir.join(file_name);
                if file_path.exists() {
                    fragment::remove_framework_from_rootfile(&file_path, prefix)?;
                }
            }
        }
    }

    // 从 Eco 根目录移除带前缀的文件
    if let Ok(entries) = fs::read_dir(eco_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) && entry.path().is_file() {
                if let Err(e) = fs::remove_file(entry.path()) {
                    log::warn!("删除文件失败: {e}");
                }
            }
        }
    }

    // 清理可能残留的 .claude/ 目录
    let eco_claude_dir = eco_dir.join(".claude");
    if eco_claude_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&eco_claude_dir) {
            log::warn!("清理残留目录失败 {}: {e}", eco_claude_dir.display());
        }
    }

    // 更新 eco.json 的隔离列表
    let new_isolation = fragment::collect_eco_isolation(eco_dir);
    fragment::update_eco_json_isolation(eco_dir, &new_isolation)?;

    // 从 fragment 重建所有 JSON 根文件
    fragment::rebuild_all_root_files(eco_dir)?;

    Ok(())
}

/// 获取 git 仓库的当前 commit hash
fn get_git_commit_hash(repo_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo_dir)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// 添加框架到 eco.json
fn add_framework_to_eco_json(
    eco_dir: &Path,
    framework_id: &str,
    commit_hash: &str,
) -> Result<(), AppError> {
    let eco_json_path = eco_dir.join("eco.json");

    let mut json: serde_json::Value = if eco_json_path.exists() {
        let content =
            fs::read_to_string(&eco_json_path).map_err(|e| AppError::io(&eco_json_path, e))?;
        fragment::parse_json(&content, "解析 eco.json 失败")?
    } else {
        serde_json::json!({})
    };

    if !json.is_object() {
        json = serde_json::json!({});
    }
    let map = json.as_object_mut().unwrap();

    // 添加到 frameworks 数组
    if !map.contains_key("frameworks") {
        map.insert("frameworks".to_string(), serde_json::json!([]));
    }
    if let Some(arr) = map.get_mut("frameworks").and_then(|v| v.as_array_mut()) {
        if !arr.iter().any(|v| v.as_str() == Some(framework_id)) {
            arr.push(serde_json::Value::String(framework_id.to_string()));
        }
    }

    // 添加到 frameworkDetails
    if !map.contains_key("frameworkDetails") {
        map.insert("frameworkDetails".to_string(), serde_json::json!({}));
    }
    if let Some(obj) = map
        .get_mut("frameworkDetails")
        .and_then(|v| v.as_object_mut())
    {
        let now = chrono::Utc::now().timestamp_millis();
        obj.insert(
            framework_id.to_string(),
            serde_json::json!({
                "installedAt": now,
                "commitHash": commit_hash,
            }),
        );
    }

    let content = fragment::write_json(&json)?;
    fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

    Ok(())
}

/// 从 eco.json 中移除框架信息
fn remove_framework_from_eco_json(eco_dir: &Path, framework_id: &str) -> Result<(), AppError> {
    let eco_json_path = eco_dir.join("eco.json");

    if !eco_json_path.exists() {
        return Ok(());
    }

    let content =
        fs::read_to_string(&eco_json_path).map_err(|e| AppError::io(&eco_json_path, e))?;
    let mut json: serde_json::Value = fragment::parse_json(&content, "解析 eco.json 失败")?;

    // 从 frameworks 数组中移除
    if let Some(arr) = json.get_mut("frameworks").and_then(|v| v.as_array_mut()) {
        arr.retain(|v| v.as_str() != Some(framework_id));
    }

    // 从 frameworkDetails 中移除
    if let Some(obj) = json
        .get_mut("frameworkDetails")
        .and_then(|v| v.as_object_mut())
    {
        obj.remove(framework_id);
    }

    let content = fragment::write_json(&json)?;
    fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

    Ok(())
}
