use std::fs;
use std::path::Path;
use std::process::Command;

/// 检查命令是否存在于 PATH
/// 获取命令的绝对路径（支持常见安装路径扫描，应对 macOS GUI 包中 PATH 环境变量受限的问题）
pub fn get_command_path(name: &str) -> Option<String> {
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

pub fn command_exists(name: &str) -> bool {
    get_command_path(name).is_some()
}

/// macOS GUI 应用 PATH 受限，为子进程补全常见 CLI 路径
pub fn augmented_path_for_subprocess() -> String {
    let mut paths = vec![
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
    ];
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".bun/bin").to_string_lossy().to_string());
        paths.push(home.join(".local/bin").to_string_lossy().to_string());
        let nvm_dir = home.join(".nvm/versions/node");
        if nvm_dir.exists() {
            if let Ok(entries) = fs::read_dir(nvm_dir) {
                for entry in entries.flatten() {
                    let bin_dir = entry.path().join("bin");
                    if bin_dir.exists() {
                        paths.push(bin_dir.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    if let Ok(existing) = std::env::var("PATH") {
        paths.push(existing);
    }
    paths.join(":")
}

/// 获取 Node.js 主版本号
pub fn get_node_major_version() -> Option<u32> {
    let output = Command::new("node").arg("--version").output().ok()?;
    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // 格式: v26.0.0
    let ver = ver.strip_prefix('v')?;
    let major = ver.split('.').next()?;
    major.parse().ok()
}

/// 检查 uv 是否有 Python 3.11+ 可用
pub fn uv_has_python_311() -> bool {
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

/// 获取 git 仓库的当前 commit hash
pub fn get_git_commit_hash(repo_dir: &Path) -> Option<String> {
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
