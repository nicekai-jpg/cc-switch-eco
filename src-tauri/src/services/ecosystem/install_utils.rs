use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::symlink;
use crate::services::ecosystem_framework;

use super::installer_hooks::rewrite_installer_hook_paths_in_claude_settings;

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
/// 根据 framework 的 dir_layout 策略 and files_prefixed 字段通用处理。
pub fn move_isolated_dirs(
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
pub fn move_isolated_rootfiles(
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
pub fn copy_non_isolated_files(
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

/// 解析模板变量
pub fn resolve_template(template: &str, eco_dir: &Path, real_home: &Path) -> String {
    template
        .replace("{eco_dir}", eco_dir.to_str().unwrap_or(""))
        .replace("{real_home}", real_home.to_str().unwrap_or(""))
}
