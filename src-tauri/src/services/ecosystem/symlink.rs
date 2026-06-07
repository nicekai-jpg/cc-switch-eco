use std::fs;
#[cfg(target_family = "unix")]
use std::os::unix::fs as unix_fs;
#[cfg(target_family = "windows")]
use std::os::windows::fs as windows_fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::fs_utils;
use crate::services::ecosystem_framework;

/// 基础隔离目录列表（始终隔离）
pub const BASE_ISOLATED_DIRS: &[&str] = &["skills", "commands", "hooks", "agents", "plugins"];

/// 创建符号链接
pub fn create_symlink(target: &Path, link: &Path) -> Result<(), AppError> {
    #[cfg(target_family = "unix")]
    {
        unix_fs::symlink(target, link).map_err(|e| {
            AppError::Message(format!(
                "创建符号链接失败: {} → {}: {e}",
                link.display(),
                target.display()
            ))
        })?;
    }
    #[cfg(target_family = "windows")]
    {
        windows_fs::symlink_dir(target, link).map_err(|e| {
            AppError::Message(format!(
                "创建符号链接失败: {} → {}: {e}",
                link.display(),
                target.display()
            ))
        })?;
    }
    Ok(())
}

/// 检查路径是否是符号链接
pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.is_symlink())
        .unwrap_or(false)
}

/// 切换 ~/.claude/ 下的 symlink 指向指定生态
///
/// 返回生态目录路径，供调用方执行迁移和重建。
pub fn switch_symlinks(id: &str) -> Result<std::path::PathBuf, AppError> {
    let claude_dir = crate::config::get_claude_config_dir();
    let eco_dir = super::ecosystem_dir(id);
    let isolation = fragment::collect_eco_isolation(&eco_dir);

    // 切换目录 symlink
    for dir_name in &isolation.dirs {
        let claude_path = claude_dir.join(dir_name);
        let eco_path = eco_dir.join(dir_name);

        fs::create_dir_all(&eco_path).map_err(|e| AppError::io(&eco_path, e))?;

        if claude_path.exists() || is_symlink(&claude_path) {
            if is_symlink(&claude_path) {
                fs::remove_file(&claude_path).map_err(|e| AppError::io(&claude_path, e))?;
            } else if claude_path.is_dir() {
                fs_utils::backup_and_replace_dir(&claude_path, &eco_path, dir_name)?;
            }
        }

        create_symlink(&eco_path, &claude_path)?;
    }

    // 切换根文件 - 用直接复制替换符号链接，彻底解决第三方 CLI 工具 rename 时删除 symlink 导致配置丢失的问题
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

    for file_name in &isolation.files {
        let claude_path = claude_dir.join(file_name);
        let eco_path = rootfiles_dir.join(file_name);

        if !eco_path.exists() {
            fs::write(&eco_path, "").map_err(|e| AppError::io(&eco_path, e))?;
        }

        // 如果 live 路径是 symlink，先删掉它
        if is_symlink(&claude_path) {
            let _ = fs::remove_file(&claude_path);
        }

        // 如果 live 路径是常规文件，且我们要切换的生态文件 eco_path 是空的，
        // 说明我们是首次切换，且正在捕获初始配置，故将 live 文件备份到 eco_path
        if claude_path.exists() && claude_path.is_file() {
            if fs::read_to_string(&eco_path).is_ok_and(|s| s.is_empty()) {
                if let Err(e) = fs::copy(&claude_path, &eco_path) {
                    log::warn!(
                        "备份文件失败: {} → {}: {e}",
                        claude_path.display(),
                        eco_path.display()
                    );
                }
            }
        }

        // 从 eco_path 复制到 claude_path，替换 live 配置
        if let Err(e) = fs::copy(&eco_path, &claude_path) {
            log::warn!(
                "复制根文件失败: {} → {}: {e}",
                eco_path.display(),
                claude_path.display()
            );
        } else {
            log::info!(
                "已复制生态根文件: {} → {}",
                eco_path.display(),
                claude_path.display()
            );
        }
    }

    // 清理不再需要的旧 symlink
    cleanup_stale_symlinks(&claude_dir, &isolation)?;

    Ok(eco_dir)
}

/// 清理不再需要的旧 symlink
fn cleanup_stale_symlinks(
    claude_dir: &Path,
    current_isolation: &fragment::EcoIsolation,
) -> Result<(), AppError> {
    let current_dirs: std::collections::HashSet<String> =
        current_isolation.dirs.iter().cloned().collect();
    let current_files: std::collections::HashSet<String> =
        current_isolation.files.iter().cloned().collect();

    // 动态收集所有可能的扩展目录（从框架注册表）
    let all_framework_dirs: Vec<String> = ecosystem_framework::get_all_frameworks()
        .iter()
        .flat_map(|f| f.isolated_dirs.iter().cloned())
        .collect();

    let all_dirs = fs_utils::merge_and_dedup(
        BASE_ISOLATED_DIRS.iter().map(|s| s.to_string()),
        all_framework_dirs.into_iter(),
    );

    for dir_name in &all_dirs {
        if current_dirs.contains(dir_name) {
            continue;
        }
        let claude_path = claude_dir.join(dir_name);
        if is_symlink(&claude_path) {
            if let Err(e) = fs::remove_file(&claude_path) {
                log::warn!("清理旧 symlink 失败 {}: {e}", claude_path.display());
            }
            if let Err(e) = fs::create_dir_all(&claude_path) {
                log::warn!("创建目录失败 {}: {e}", claude_path.display());
            }
        }
    }

    // 动态收集所有可能的根文件
    let all_possible_files: Vec<String> = ecosystem_framework::get_all_frameworks()
        .iter()
        .flat_map(|f| f.isolated_files.iter().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for file_name in &all_possible_files {
        if current_files.contains(file_name.as_str()) {
            continue;
        }
        let claude_path = claude_dir.join(file_name);
        if is_symlink(&claude_path) {
            if let Err(e) = fs::remove_file(&claude_path) {
                log::warn!("清理旧 symlink 失败 {}: {e}", claude_path.display());
            }
            if let Err(e) = fs::write(&claude_path, "") {
                log::warn!("创建空文件失败 {}: {e}", claude_path.display());
            }
        }
    }

    Ok(())
}
