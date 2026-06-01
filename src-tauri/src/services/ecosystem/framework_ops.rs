use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::AppError;
use crate::services::ecosystem_framework;
use crate::store::AppState;

/// 安装框架到指定生态
pub fn install_framework(
    state: &AppState,
    eco_id: &str,
    framework_id: &str,
) -> Result<(), AppError> {
    let framework = ecosystem_framework::find_framework(framework_id)
        .ok_or_else(|| AppError::Message(format!("框架 '{framework_id}' 不存在")))?;

    if !state.db.ecosystem_exists(eco_id)? {
        return Err(AppError::Message(format!("生态 '{eco_id}' 不存在")));
    }

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
        let _ = fs::remove_dir_all(&fw_dir);
        return Err(AppError::Message(format!("git clone 失败: {stderr}")));
    }

    // 安装文件到 Eco 隔离目录
    do_install(&eco_dir, &framework, &fw_dir)?;

    // 更新 eco.json
    let commit_hash = get_git_commit_hash(&fw_dir).unwrap_or_default();
    add_framework_to_eco_json(&eco_dir, framework_id, &commit_hash)?;

    log::info!("框架 '{framework_id}' 已安装到生态 '{eco_id}'");
    Ok(())
}

/// 卸载框架
pub fn uninstall_framework(
    state: &AppState,
    eco_id: &str,
    framework_id: &str,
) -> Result<(), AppError> {
    if !state.db.ecosystem_exists(eco_id)? {
        return Err(AppError::Message(format!("生态 '{eco_id}' 不存在")));
    }

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
    if !state.db.ecosystem_exists(eco_id)? {
        return Err(AppError::Message(format!("生态 '{eco_id}' 不存在")));
    }

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

    // 从 fragment 重建所有 JSON 根文件
    crate::services::ecosystem::fragment::rebuild_all_root_files(&eco_dir)?;

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
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| AppError::Message(format!("解析 eco.json 失败: {e}")))?;

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
    match install_result {
        Ok(()) => Ok(()),
        Err(e) => {
            log::warn!("官方安装命令失败: {e}，回退到手动复制");
            install_manual_copy(eco_dir, framework, fw_dir)
        }
    }
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
        let _ = fs::remove_dir_all(&eco_claude_dir);
        return Err(AppError::Message(format!("官方安装命令失败: {e}")));
    }

    // 将 .claude/ 中的文件移动到 Eco 对应目录
    move_claude_files_to_eco(&eco_claude_dir, eco_dir, &framework.file_prefix)?;

    // 清理 Eco 的 .claude/ 目录
    let _ = fs::remove_dir_all(&eco_claude_dir);

    Ok(())
}

/// 将 Eco 的 .claude/ 目录下的文件移动到 Eco 对应目录
fn move_claude_files_to_eco(
    eco_claude_dir: &Path,
    eco_dir: &Path,
    prefix: &str,
) -> Result<(), AppError> {
    let isolation = crate::services::ecosystem::fragment::collect_eco_isolation(eco_dir);

    for dir_name in &isolation.dirs {
        let src_dir = eco_claude_dir.join(dir_name);
        if !src_dir.exists() || !src_dir.is_dir() {
            continue;
        }
        let dst_dir = eco_dir.join(dir_name);
        fs::create_dir_all(&dst_dir).map_err(|e| AppError::io(&dst_dir, e))?;

        if let Ok(entries) = fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let dst_name = format!("{prefix}{name}");
                let dst_path = dst_dir.join(&dst_name);
                if dst_path.exists() {
                    if dst_path.is_dir() {
                        fs::remove_dir_all(&dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                    } else {
                        fs::remove_file(&dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                    }
                }
                fs::rename(entry.path(), &dst_path).or_else(|_| {
                    crate::services::ecosystem::fs_utils::copy_path_to(&entry.path(), &dst_path)?;
                    if entry.path().is_dir() {
                        fs::remove_dir_all(entry.path())
                    } else {
                        fs::remove_file(entry.path())
                    }
                    .map_err(|e| AppError::io(entry.path(), e))
                })?;
            }
        }
    }

    // 处理根文件
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

    for file_name in &isolation.files {
        let src_path = eco_claude_dir.join(file_name);
        if !src_path.exists() || !src_path.is_file() {
            continue;
        }
        let dst_path = rootfiles_dir.join(file_name);
        if dst_path.exists() {
            crate::services::ecosystem::fragment::merge_root_file(&src_path, &dst_path, prefix)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
        }
    }

    // 处理其他非隔离根文件
    if let Ok(entries) = fs::read_dir(eco_claude_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.')
                || entry.path().is_dir()
                || isolation.files.contains(&name)
                || crate::services::ecosystem::symlink::BASE_ISOLATED_DIRS.contains(&name.as_str())
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

    // 从 fragment 重建所有 JSON 根文件
    crate::services::ecosystem::fragment::rebuild_all_root_files(eco_dir)?;

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
fn install_manual_copy(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    for dir_name in &framework.provided_dirs {
        let src = fw_dir.join(dir_name);
        if !src.exists() {
            continue;
        }

        // .claude-plugin 特殊处理
        if dir_name == ".claude-plugin" {
            let plugin_dst = eco_dir.join("plugins").join(&framework.id);
            if !plugin_dst.exists() {
                crate::services::ecosystem::fs_utils::copy_dir_recursive(&src, &plugin_dst)?;
            }
            continue;
        }

        if !src.is_dir() {
            continue;
        }

        let dst = eco_dir.join(dir_name);
        fs::create_dir_all(&dst).map_err(|e| AppError::io(&dst, e))?;

        if let Ok(entries) = fs::read_dir(&src) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let dst_name = format!("{}{}", framework.file_prefix, name);
                let dst_path = dst.join(&dst_name);
                if !dst_path.exists() {
                    crate::services::ecosystem::fs_utils::copy_path_to(&entry.path(), &dst_path)?;
                }
            }
        }
    }

    // agency-agents-zh 特殊处理
    if framework.id == "agency-agents-zh" {
        copy_agency_agents_fallback(fw_dir, eco_dir, &framework.file_prefix)?;
    }

    Ok(())
}

/// agency-agents-zh 回退方案：递归扫描分类目录中的 .md 文件
fn copy_agency_agents_fallback(
    fw_dir: &Path,
    eco_dir: &Path,
    prefix: &str,
) -> Result<(), AppError> {
    let agents_dst = eco_dir.join("agents");
    fs::create_dir_all(&agents_dst).map_err(|e| AppError::io(&agents_dst, e))?;

    for entry in fs::read_dir(fw_dir).map_err(|e| AppError::io(fw_dir, e))? {
        let entry = entry.map_err(|e| AppError::io(fw_dir, e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "scripts" || name == "frameworks" {
            continue;
        }
        copy_agent_md_files(&path, &agents_dst, prefix)?;
    }

    Ok(())
}

/// 递归扫描目录，将含 YAML front matter 的 .md 文件扁平复制到目标目录
fn copy_agent_md_files(src_dir: &Path, dst_dir: &Path, prefix: &str) -> Result<(), AppError> {
    for entry in fs::read_dir(src_dir).map_err(|e| AppError::io(src_dir, e))? {
        let entry = entry.map_err(|e| AppError::io(src_dir, e))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() && !crate::services::ecosystem::symlink::is_symlink(&path) {
            copy_agent_md_files(&path, dst_dir, prefix)?;
        } else if path.is_file() && name.ends_with(".md") {
            if let Ok(content) = fs::read_to_string(&path) {
                if !content.starts_with("---") {
                    continue;
                }
                let dst_path = dst_dir.join(format!("{prefix}{name}"));
                if !dst_path.exists() {
                    fs::copy(&path, &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                }
            }
        }
    }

    Ok(())
}

/// 按前缀卸载框架文件
fn uninstall_by_prefix(eco_dir: &Path, prefix: &str, framework_id: &str) -> Result<(), AppError> {
    let isolation = crate::services::ecosystem::fragment::collect_eco_isolation(eco_dir);

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
                    crate::services::ecosystem::fragment::remove_framework_from_rootfile(
                        &file_path, prefix,
                    )?;
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
        let _ = fs::remove_dir_all(&eco_claude_dir);
    }

    // 更新 eco.json 的隔离列表
    let new_isolation = crate::services::ecosystem::fragment::collect_eco_isolation(eco_dir);
    crate::services::ecosystem::fragment::update_eco_json_isolation(eco_dir, &new_isolation)?;

    // 从 fragment 重建所有 JSON 根文件
    crate::services::ecosystem::fragment::rebuild_all_root_files(eco_dir)?;

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
        serde_json::from_str(&content)
            .map_err(|e| AppError::Message(format!("解析 eco.json 失败: {e}")))?
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

    let content = serde_json::to_string_pretty(&json).unwrap_or_default();
    fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

    // 更新隔离列表
    let isolation = crate::services::ecosystem::fragment::collect_eco_isolation(eco_dir);
    crate::services::ecosystem::fragment::update_eco_json_isolation(eco_dir, &isolation)?;

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
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| AppError::Message(format!("解析 eco.json 失败: {e}")))?;

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

    let content = serde_json::to_string_pretty(&json).unwrap_or_default();
    fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

    Ok(())
}
