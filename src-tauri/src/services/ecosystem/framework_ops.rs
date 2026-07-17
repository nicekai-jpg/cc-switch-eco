use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem_framework;
use crate::store::AppState;

use super::cmd_utils::{
    command_exists, get_node_major_version, uv_has_python_311, get_git_commit_hash
};
use super::installers::do_install;
use super::plugin_install::validate_hook_delivery;
use super::plugin_ops::unregister_plugin_from_installed_plugins;

fn ensure_eco_exists(state: &AppState, eco_id: &str) -> Result<(), AppError> {
    if !state.db.ecosystem_exists(eco_id)? {
        return Err(AppError::Message(format!("生态 '{eco_id}' 不存在")));
    }
    Ok(())
}

/// 检查框架安装所需的依赖是否满足
pub fn check_framework_deps(framework: &ecosystem_framework::FrameworkRegistry) -> Result<(), AppError> {
    let mut missing: Vec<String> = Vec::new();

    /// 根据编译目标平台返回安装提示字符串
    macro_rules! platform_tip {
        ($macos:expr, $linux:expr, $windows:expr) => {
            if cfg!(target_os = "macos") {
                $macos
            } else if cfg!(target_os = "linux") {
                $linux
            } else if cfg!(target_os = "windows") {
                $windows
            } else {
                ""
            }
        };
    }

    // 所有框架都需要 git
    if !command_exists("git") {
        let tip = platform_tip!(
            " (推荐运行: brew install git)",
            " (推荐运行: sudo apt install git)",
            " (推荐运行: winget install Git.Git)"
        );
        missing.push(format!("git{tip}"));
    }

    match framework.install_method.as_str() {
        "npx" => {
            let node_tip = platform_tip!(
                " (Node.js 20+，推荐运行: brew install node)",
                " (Node.js 20+，推荐运行: sudo apt install nodejs)",
                " (Node.js 20+，推荐运行: winget install OpenJS.NodeJS.LTS)"
            );

            if !command_exists("node") {
                missing.push(format!("node{node_tip}"));
            } else if let Some(ver) = get_node_major_version() {
                if ver < 20 {
                    missing.push(format!("node 版本过低 (当前 v{ver}, 需要 20+{node_tip})"));
                }
            }
            if !command_exists("npx") {
                missing.push("npx (通常随 npm/Node.js 一起安装)".to_string());
            }
        }
        "script" => {
            if !command_exists("bash") {
                missing.push("bash".to_string());
            }
            // gstack 的 setup 脚本额外需要 bun
            if framework.id == "gstack" && !command_exists("bun") {
                let bun_tip = if cfg!(any(target_os = "macos", target_os = "linux")) {
                    " (推荐运行: curl -fsSL https://bun.sh/install | bash)"
                } else if cfg!(target_os = "windows") {
                    " (推荐运行: powershell -c \"irm https://bun.sh/install.ps1 | iex\")"
                } else {
                    " (https://bun.sh)"
                };
                missing.push(format!("bun{bun_tip}"));
            }
        }
        "uv" => {
            let uv_tip = platform_tip!(
                " (推荐运行: brew install uv 或 curl -LsSf https://astral.sh/uv/install.sh | sh)",
                " (推荐运行: curl -LsSf https://astral.sh/uv/install.sh | sh)",
                " (推荐运行: powershell -c \"irm https://astral.sh/uv/install.ps1 | iex\")"
            );

            if !command_exists("uv") {
                missing.push(format!("uv{uv_tip}"));
            } else if !uv_has_python_311() {
                // uv 存在但缺少 Python 3.11+
                missing.push("Python 3.11+ (推荐运行: uv python install 3.11)".to_string());
            }
        }
        "copy" => {
            // copy 方式只需要 git（已在上面检查）
        }
        "plugin" => {
            if framework.marketplace_name.is_some() && !command_exists("claude") {
                let tip = platform_tip!(
                    " (需安装 Claude Code CLI)",
                    " (需安装 Claude Code CLI)",
                    " (需安装 Claude Code CLI)"
                );
                missing.push(format!("claude{tip}"));
            }
        }
        _ => {}
    }

    if !missing.is_empty() {
        let tips = missing
            .iter()
            .map(|dep| format!("  • {dep}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(AppError::Message(format!(
            "框架 '{}' 创建缺少以下依赖，请安装后重试：\n\n{}",
            framework.name,
            tips
        )));
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

    // 检查框架安装依赖
    check_framework_deps(&framework)?;

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

    validate_hook_delivery(&framework, &fw_dir)?;

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

    // 清理 dir_mappings 映射的目录（如 plugins/{id}/）
    // 这些目录不以 file_prefix 开头，不会被 uninstall_by_prefix 清理
    if let Some(fw) = framework {
        for (_, dst_template) in &fw.dir_mappings {
            let dst = eco_dir.join(dst_template.replace("{id}", &framework_id));
            if dst.exists() {
                if let Err(e) = fs::remove_dir_all(&dst) {
                    log::warn!("清理 dir_mappings 目录失败 {}: {e}", dst.display());
                }
            }
        }

        // 对于 plugin 类型框架，从 installed_plugins.json 中移除注册
        if fw.install_method == "plugin" {
            unregister_plugin_from_installed_plugins(&eco_dir, framework_id)?;
        }
    }

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

    let framework = ecosystem_framework::find_framework(framework_id)
        .ok_or_else(|| AppError::Message(format!("框架 '{framework_id}' 不存在")))?;

    // 检查框架安装依赖
    check_framework_deps(&framework)?;

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

    validate_hook_delivery(&framework, &fw_dir)?;

    // 先卸载旧文件，再重新安装（复用已查到的 framework）
    let prefix = framework.file_prefix.as_str();
    uninstall_by_prefix(&eco_dir, prefix, framework_id)?;

    // 清理 dir_mappings 映射的目录（如 plugins/{id}/）
    for (_, dst_template) in &framework.dir_mappings {
        let dst = eco_dir.join(dst_template.replace("{id}", framework_id));
        if dst.exists() {
            if let Err(e) = fs::remove_dir_all(&dst) {
                log::warn!("清理 dir_mappings 目录失败 {}: {e}", dst.display());
            }
        }
    }

    // 对于 plugin 类型框架，先清理旧的注册信息（cache、installed_plugins 等）
    // 否则 do_install 重新注册时旧版本 cache 会残留
    if framework.install_method == "plugin" {
        if let Err(e) = unregister_plugin_from_installed_plugins(&eco_dir, framework_id) {
            log::warn!("更新时清理旧插件注册失败: {e}");
        }
    }

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

/// 卸载框架文件
fn uninstall_by_prefix(eco_dir: &Path, prefix: &str, framework_id: &str) -> Result<(), AppError> {
    let isolation = fragment::collect_eco_isolation(eco_dir);

    // 获取框架的 isolated_dirs，用于判断哪些目录需要整体删除
    let framework = ecosystem_framework::find_framework(framework_id);
    let isolated_dirs: Vec<String> = framework
        .as_ref()
        .map(|fw| fw.isolated_dirs.clone())
        .unwrap_or_default();

    // 从各隔离目录移除带前缀的文件
    for dir_name in &isolation.dirs {
        let dir = eco_dir.join(dir_name);
        if !dir.exists() {
            continue;
        }

        // isolated_dirs 中以 prefix 开头的目录（如 gsd-core/）由框架独占，
        // 其内部文件不以 prefix 开头，需要整体删除
        if dir_name.starts_with(prefix) && isolated_dirs.contains(dir_name) {
            fs::remove_dir_all(&dir).map_err(|e| AppError::io(&dir, e))?;
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


