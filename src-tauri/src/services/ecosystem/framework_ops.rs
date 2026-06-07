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
        "copy" | "plugin" => {
            // copy/plugin 方式只需要 git（已在上面检查）
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

/// 检查命令是否存在于 PATH
/// 获取命令的绝对路径（支持常见安装路径扫描，应对 macOS GUI 包中 PATH 环境变量受限的问题）
fn get_command_path(name: &str) -> Option<String> {
    // 1. 尝试使用标准的 which 查找
    if let Ok(output) = Command::new("which").arg(name).output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && Path::new(&path).exists() {
                return Some(path);
            }
        }
    }

    // 2. 在 macOS/Linux 的常见路径中扫描
    let mut search_paths = vec![
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
    ];

    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join(".bun/bin").to_string_lossy().to_string());
        search_paths.push(home.join(".local/bin").to_string_lossy().to_string());
        
        // 支持 nvm
        let nvm_dir = home.join(".nvm/versions/node");
        if nvm_dir.exists() {
            if let Ok(entries) = fs::read_dir(nvm_dir) {
                for entry in entries.flatten() {
                    let bin_dir = entry.path().join("bin");
                    if bin_dir.exists() {
                        search_paths.push(bin_dir.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    for prefix in search_paths {
        let binary_path = Path::new(&prefix).join(name);
        if binary_path.exists() && binary_path.is_file() {
            return Some(binary_path.to_string_lossy().to_string());
        }
    }

    None
}

fn command_exists(name: &str) -> bool {
    get_command_path(name).is_some()
}

/// 获取 Node.js 主版本号
fn get_node_major_version() -> Option<u32> {
    let output = Command::new("node").arg("--version").output().ok()?;
    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // 格式: v26.0.0
    let ver = ver.strip_prefix('v')?;
    let major = ver.split('.').next()?;
    major.parse().ok()
}

/// 检查 uv 是否有 Python 3.11+ 可用
fn uv_has_python_311() -> bool {
    let output = Command::new("uv")
        .args(["python", "list", "--only-installed"])
        .output();

    if let Ok(output) = output {
        if !output.status.success() {
            return false;
        }
        let list = String::from_utf8_lossy(&output.stdout);
        for line in list.lines() {
            // 格式: cpython-3.13.12-macos-aarch64-none    /path/to/python3.13
            if line.starts_with("cpython-3.") {
                let ver_part = match line.strip_prefix("cpython-3.") {
                    Some(v) => v,
                    None => continue,
                };
                let minor_str = match ver_part.split('.').next() {
                    Some(v) => v,
                    None => continue,
                };
                if let Ok(minor) = minor_str.parse::<u32>() {
                    if minor >= 11 {
                        return true;
                    }
                }
            }
        }
    }
    false
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

/// 执行框架安装（官方命令 + 手动复制回退）
fn do_install(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    let install_result = match framework.install_method.as_str() {
        "npx" | "script" => install_via_official_command(eco_dir, framework, fw_dir),
        "uv" => install_via_uv_command(eco_dir, framework, fw_dir),
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

    // 对于 plugin 类型框架，注册到 installed_plugins.json
    if result.is_ok() && framework.install_method == "plugin" {
        register_plugin_to_installed_plugins(eco_dir, framework)?;
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

/// 使用 uv 工具安装框架（如 Spec Kit 的 specify-cli）
///
/// 流程：1) uv tool install 安装 CLI 工具  2) 运行 CLI init 命令  3) 移动文件到 Eco 目录
fn install_via_uv_command(
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
    // Spec Kit: specify init . --integration claude
    // 使用 uv tool run 运行 specify，以解决 specify 安装路径没有包含在 PATH 环境变量中的问题
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

    // Step 3: 将 .claude/ 中的文件移动到 Eco 对应目录
    move_claude_files_to_eco(&eco_claude_dir, eco_dir, framework)?;

    // 清理 Eco 的 .claude/ 目录
    if let Err(e) = fs::remove_dir_all(&eco_claude_dir) {
        log::warn!("清理临时目录失败 {}: {e}", eco_claude_dir.display());
    }

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
///
/// 对于 plugin 类型框架，当 .claude-plugin 通过 dir_mappings 映射到 plugins/{id} 时，
/// provided_dirs 中除 .claude-plugin 外的其他目录（如 commands）也需要复制到插件目录内，
/// 因为 plugin.json 中的路径（如 ./commands/setup.md）是相对于插件目录的。
fn install_manual_copy(
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
            // dir_mappings 目标可能已部分存在（如其他映射先创建了子目录），
            // 使用 copy_dir_recursive 合并而非跳过
            fs_utils::copy_dir_recursive(&src, &dst)?;
            continue;
        }

        if !src.is_dir() {
            continue;
        }

        // 对于 plugin 类型框架，将非 .claude-plugin 的 provided_dirs 也复制到插件目录内
        // 这样 plugin.json 中的相对路径引用（如 ./commands/setup.md）才能正确解析
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

/// 将 plugin 类型框架注册到 Claude Code 的插件发现系统
///
/// Claude Code 的插件系统需要三个文件协同工作：
/// 1. installed_plugins.json — 记录已安装插件，key 格式为 pluginName@marketplaceName
/// 2. known_marketplaces.json — 记录 marketplace 来源
/// 3. marketplaces/{marketplaceName}/ — marketplace 仓库克隆（含 .claude-plugin/marketplace.json）
///
/// 此外还需要：
/// - cache/{marketplaceName}/{pluginName}/{version}/ — 插件安装路径
/// - data/{pluginName}-{marketplaceName}/ — 插件数据目录
///
/// 因为 ~/.claude/plugins/ 是指向 eco_dir/plugins/ 的 symlink，
/// 所有路径都写入 eco_dir/plugins/ 下，通过 symlink 透明映射到 ~/.claude/plugins/。
fn register_plugin_to_installed_plugins(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    let marketplace_name = framework
        .marketplace_name
        .as_ref()
        .ok_or_else(|| AppError::Message(format!(
            "框架 '{}' 是 plugin 类型但未配置 marketplace_name",
            framework.id
        )))?;

    let plugins_dir = eco_dir.join("plugins");
    let plugin_name = &framework.id;
    let plugin_key = format!("{plugin_name}@{marketplace_name}");

    // 读取 plugin.json 获取版本信息
    let plugin_json_path = plugins_dir
        .join(plugin_name)
        .join(".claude-plugin")
        .join("plugin.json");
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

    // 读取 marketplace.json 获取 git commit sha
    let marketplace_json_path = plugins_dir
        .join(plugin_name)
        .join(".claude-plugin")
        .join("marketplace.json");
    let git_commit_sha = get_git_commit_hash(&eco_dir.join("frameworks").join(plugin_name));

    let now = chrono::Utc::now().to_rfc3339();

    // 1. 创建 cache/{marketplaceName}/{pluginName}/{version}/ 目录并复制插件内容
    let cache_install_path = plugins_dir
        .join("cache")
        .join(marketplace_name)
        .join(plugin_name)
        .join(&version);
    let plugin_src = plugins_dir.join(plugin_name);
    if !cache_install_path.exists() {
        fs_utils::copy_dir_recursive(&plugin_src, &cache_install_path)?;
    }

    // Claude Code 解析 plugin.json 中的路径（如 "./commands/setup.md"）时，
    // 是相对于 installPath（即 cache_install_path）而非 .claude-plugin/ 目录。
    // 因此需要将 .claude-plugin/ 下的子目录（commands、hooks 等）也复制到 installPath 根目录。
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

    // 3. 创建 marketplaces/{marketplaceName}/ 目录（含 .claude-plugin/marketplace.json）
    let marketplace_dir = plugins_dir.join("marketplaces").join(marketplace_name);
    if !marketplace_dir.exists() {
        // 从框架 git 仓库复制整个目录作为 marketplace
        let fw_dir = eco_dir.join("frameworks").join(plugin_name);
        if fw_dir.exists() {
            fs_utils::copy_dir_recursive(&fw_dir, &marketplace_dir)?;
            // 删除 .git 目录以减小体积
            let git_dir = marketplace_dir.join(".git");
            if git_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&git_dir) {
                    log::warn!("清理 marketplace .git 目录失败: {e}");
                }
            }
        } else {
            // 如果没有 git 仓库，创建最小 marketplace 结构
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

    // installPath 使用 ~/.claude/plugins/cache/{marketplaceName}/{pluginName}/{version}/
    // 因为 ~/.claude/plugins/ 是 symlink，Claude Code 通过 symlink 解析到 eco 目录
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

    // marketplace installLocation 使用 ~/.claude/plugins/marketplaces/{marketplaceName}/
    let marketplace_install_location = claude_dir
        .join("plugins")
        .join("marketplaces")
        .join(marketplace_name)
        .to_str()
        .unwrap_or("")
        .to_string();

    // 从 repo_url 提取 GitHub repo 信息
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

    // 6. 自动 enable 插件（写入 ~/.claude/settings.json 的 enabledPlugins）
    enable_plugin_in_settings(eco_dir, &plugin_key)?;

    // 7. 对于 claude-hud 插件，自动完成 HUD setup（statusLine + config.json）
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

/// 自动完成 Claude HUD 的 setup 配置
///
/// HUD 的 /claude-hud:setup 命令需要用户手动运行，这里在安装时自动完成：
/// 1. 检测 runtime（优先 bun，回退 node）
/// 2. 生成 statusLine 命令并写入 HUD 的 settings fragment
/// 3. 创建 ~/.claude/plugins/claude-hud/config.json 默认配置
fn auto_setup_hud(
    eco_dir: &Path,
    cache_install_path: &Path,
) -> Result<(), AppError> {
    // 1. 检测 runtime：优先 bun（更快），回退 node
    let runtime = if command_exists("bun") {
        get_command_path("bun")
    } else if command_exists("node") {
        get_command_path("node")
    } else {
        log::warn!("HUD auto-setup: 未找到 bun 或 node，跳过 statusLine 配置");
        return Ok(());
    };

    let runtime_path = match runtime {
        Some(p) => p,
        None => {
            log::warn!("HUD auto-setup: 无法获取 runtime 路径，跳过 statusLine 配置");
            return Ok(());
        }
    };

    // 2. 确定源文件：bun 用 src/index.ts，node 用 dist/index.js
    let use_bun = runtime_path.contains("bun");
    let source = if use_bun {
        "src/index.ts"
    } else {
        "dist/index.js"
    };

    // 检查源文件是否存在于 cache 目录
    let source_file = cache_install_path.join(source);
    if !source_file.exists() {
        log::warn!(
            "HUD auto-setup: 源文件 {} 不存在于 {}，跳过 statusLine 配置",
            source,
            cache_install_path.display()
        );
        return Ok(());
    }

    // 3. 生成 statusLine 命令
    // macOS/Linux 格式（与 HUD setup.md Step 1 一致）
    // 使用动态版本查找，这样插件更新后无需重新 setup
    //
    // 原始命令中的 '"'"' 是 bash 单引号嵌套技巧：
    // 结束当前单引号 → 双引号包裹一个单引号 → 重新开始单引号
    // 在 Rust 字符串中直接写 '"'"' 即可
    let statusline_command = if use_bun {
        format!(
            "bash -c 'cols=$(stty size </dev/tty 2>/dev/null | awk '\"'\"'{{print $2}}'\"'\"'); \
             export COLUMNS=$(( ${{cols:-120}} > 4 ? ${{cols:-120}} - 4 : 1 )); \
             plugin_dir=$(ls -d \"${{CLAUDE_CONFIG_DIR:-$HOME/.claude}}\"/plugins/cache/*/claude-hud/*/ 2>/dev/null | \
               awk -F/ '\"'\"'{{ print $(NF-1) \"\\t\" $(0) }}'\"'\"' | \
               grep -E '\"'\"'^[0-9]+\\.[0-9]+\\.[0-9]+[[:space:]]'\"'\"' | \
               sort -t. -k1,1n -k2,2n -k3,3n -k4,4n | tail -1 | cut -f2-); \
             exec \"{runtime_path}\" --env-file /dev/null \"${{plugin_dir}}{source}\"'"
        )
    } else {
        format!(
            "bash -c 'cols=$(stty size </dev/tty 2>/dev/null | awk '\"'\"'{{print $2}}'\"'\"'); \
             export COLUMNS=$(( ${{cols:-120}} > 4 ? ${{cols:-120}} - 4 : 1 )); \
             plugin_dir=$(ls -d \"${{CLAUDE_CONFIG_DIR:-$HOME/.claude}}\"/plugins/cache/*/claude-hud/*/ 2>/dev/null | \
               awk -F/ '\"'\"'{{ print $(NF-1) \"\\t\" $(0) }}'\"'\"' | \
               grep -E '\"'\"'^[0-9]+\\.[0-9]+\\.[0-9]+[[:space:]]'\"'\"' | \
               sort -t. -k1,1n -k2,2n -k3,3n -k4,4n | tail -1 | cut -f2-); \
             exec \"{runtime_path}\" \"${{plugin_dir}}{source}\"'"
        )
    };

    // 4. 将 statusLine 写入 HUD 的 settings fragment
    // 这样 rebuild_all_root_files 会自动合并 statusLine 到 settings.json
    // 避免直接写入 ~/.claude/settings.json 导致 symlink 被破坏或 fragment 重建时丢失
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

    // 确保 settings.json 在隔离列表中
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

    let hud_prefix = "hud-";
    let frag_path = fragment::fragment_path(&rootfiles_dir, "settings.json", hud_prefix);

    let statusline_fragment = serde_json::json!({
        "statusLine": {
            "type": "command",
            "command": statusline_command
        }
    });

    // 合并到已有 fragment（保留其他 HUD 写入的 settings 配置）
    if frag_path.exists() {
        let existing = fs::read_to_string(&frag_path).map_err(|e| AppError::io(&frag_path, e))?;
        let mut existing_json: serde_json::Value =
            fragment::parse_json(&existing, "解析 HUD fragment 失败")?;

        let mut conflicts = Vec::new();
        fragment::json_deep_merge_with_array_dedup(
            &mut existing_json,
            &statusline_fragment,
            "",
            hud_prefix,
            &mut conflicts,
        );

        fs::write(&frag_path, fragment::write_json(&existing_json)?)
            .map_err(|e| AppError::io(&frag_path, e))?;
    } else {
        fs::write(&frag_path, fragment::write_json(&statusline_fragment)?)
            .map_err(|e| AppError::io(&frag_path, e))?;
    }

    // 从 fragment 重建 settings.json
    fragment::rebuild_all_root_files(eco_dir)?;

    log::info!(
        "HUD auto-setup: 已写入 statusLine fragment (runtime: {}, source: {})",
        runtime_path,
        source
    );

    // 5. 创建 config.json 默认配置
    // HUD 读取 ~/.claude/plugins/claude-hud/config.json
    // ~/.claude/plugins/ 是 symlink → eco_dir/plugins/
    // 所以写入 eco_dir/plugins/claude-hud/config.json 即可
    let hud_config_dir = eco_dir.join("plugins").join("claude-hud");
    fs::create_dir_all(&hud_config_dir).map_err(|e| AppError::io(&hud_config_dir, e))?;

    let config_path = hud_config_dir.join("config.json");
    if !config_path.exists() {
        // 默认配置：expanded 布局，显示 model + context bar + git + token breakdown + usage
        let default_config = serde_json::json!({
            "language": "en",
            "lineLayout": "expanded",
            "showSeparators": false,
            "gitStatus": {
                "enabled": true,
                "showDirty": true,
                "showAheadBehind": false,
                "showFileStats": false
            },
            "display": {
                "showModel": true,
                "showContextBar": true,
                "showTokenBreakdown": true,
                "showUsage": true,
                "usageBarEnabled": true
            }
        });

        let config_content = fragment::write_json(&default_config)?;
        fs::write(&config_path, config_content).map_err(|e| AppError::io(&config_path, e))?;

        log::info!("HUD auto-setup: 已创建默认 config.json");
    }

    Ok(())
}



/// 在 eco 的 settings user-fragment 中启用插件
///
/// enabledPlugins 是跨框架共享的配置，不属于任何特定框架的 fragment，
/// 因此写入 user-fragment（始终最后合并，优先级最高）。
/// 这样 rebuild_all_root_files 不会丢失 enabledPlugins。
fn enable_plugin_in_settings(eco_dir: &Path, plugin_key: &str) -> Result<(), AppError> {
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

    // 确保 enabledPlugins 字段存在
    if user_frag.get("enabledPlugins").is_none() {
        user_frag
            .as_object_mut()
            .unwrap()
            .insert("enabledPlugins".to_string(), serde_json::json!({}));
    }

    // 添加插件到 enabledPlugins
    if let Some(ep) = user_frag.get_mut("enabledPlugins").and_then(|v| v.as_object_mut()) {
        ep.insert(plugin_key.to_string(), serde_json::json!(true));
    }

    let content = fragment::write_json(&user_frag)?;
    fs::write(&user_frag_path, content)
        .map_err(|e| AppError::io(&user_frag_path, e))?;

    // 重建 settings.json
    fragment::rebuild_all_root_files(eco_dir)?;

    log::info!("已在 user-fragment 中启用插件 '{}'", plugin_key);
    Ok(())
}

/// 从 GitHub URL 提取 owner/repo 格式
/// 如 https://github.com/jarrodwatts/claude-hud.git → jarrodwatts/claude-hud
fn extract_github_repo(url: &str) -> String {
    url
        .strip_prefix("https://github.com/")
        .and_then(|s| s.strip_suffix(".git"))
        .unwrap_or(url)
        .to_string()
}

/// 从 installed_plugins.json 和 known_marketplaces.json 中移除插件注册
fn unregister_plugin_from_installed_plugins(
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
            // 移除所有包含该 framework_id 的 key（可能是 id 或 id@marketplace）
            let keys_to_remove: Vec<String> = plugins_obj
                .keys()
                .filter(|k| k.as_str() == framework_id || k.starts_with(&format!("{framework_id}@")))
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

    // 从 settings.json 的 enabledPlugins 中移除
    let plugin_key = if let Some(mkt_name) = marketplace_name {
        format!("{framework_id}@{mkt_name}")
    } else {
        framework_id.to_string()
    };
    disable_plugin_in_settings(eco_dir, &plugin_key)?;

    // 对于 claude-hud，清理 statusLine 和 config.json
    if framework_id == "claude-hud" {
        cleanup_hud_settings(eco_dir)?;
    }

    log::info!(
        "已从 Claude Code 插件系统中移除插件 '{}'",
        framework_id
    );
    Ok(())
}

/// 清理 HUD 的 statusLine fragment 和 config.json
///
/// 卸载 claude-hud 时需要：
/// 1. 删除 HUD 的 settings fragment（rootfiles/settings.hud-fragment.json）
/// 2. 重建 settings.json（statusLine 会自动消失）
/// 3. 删除 ~/.claude/plugins/claude-hud/config.json（通过 symlink 在 eco 目录下）
fn cleanup_hud_settings(eco_dir: &Path) -> Result<(), AppError> {
    // 删除 HUD 的 settings fragment
    let rootfiles_dir = eco_dir.join("rootfiles");
    let hud_frag = fragment::fragment_path(&rootfiles_dir, "settings.json", "hud-");
    if hud_frag.exists() {
        fs::remove_file(&hud_frag).map_err(|e| AppError::io(&hud_frag, e))?;
    }

    // 重建 settings.json（statusLine 会自动消失）
    fragment::rebuild_all_root_files(eco_dir)?;

    // 移除 config.json（在 eco 的 plugins 目录下，通过 symlink 映射到 ~/.claude/plugins/）
    let hud_config_dir = eco_dir.join("plugins").join("claude-hud");
    let config_path = hud_config_dir.join("config.json");
    if config_path.exists() {
        if let Err(e) = fs::remove_file(&config_path) {
            log::warn!("清理 HUD config.json 失败 {}: {e}", config_path.display());
        }
    }

    log::info!("已清理 HUD statusLine fragment 和 config.json");
    Ok(())
}

/// 从 eco 的 settings user-fragment 中移除插件
///
/// 与 enable_plugin_in_settings 对应，从 user-fragment 的 enabledPlugins 中移除插件。
fn disable_plugin_in_settings(eco_dir: &Path, plugin_key: &str) -> Result<(), AppError> {
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

    // 重建 settings.json
    fragment::rebuild_all_root_files(eco_dir)?;

    log::info!("已从 user-fragment 中移除插件 '{}'", plugin_key);
    Ok(())
}
