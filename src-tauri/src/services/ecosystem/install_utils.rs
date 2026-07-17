use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::fs_utils;
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

    // 处理 npx 安装器写到 HOME 根目录（而非 .claude/）的 isolated_dirs
    // 例如 GSD 的 gsd-core/ 被安装器写到 {eco_dir}/gsd-core/ 而非 {eco_dir}/.claude/gsd-core/
    ensure_isolated_dirs_from_eco_root(eco_dir, framework)?;

    fragment::rebuild_all_root_files(eco_dir)?;
    Ok(())
}

/// 移动隔离目录中的文件（skills/commands/hooks/agents/plugins 等）
///
/// 根据 framework 的 dir_layout 策略 and files_prefixed 字段通用处理。
/// 对于 isolated_dirs（如 gsd-core/），内部结构不应被策略加前缀，直接递归移动。
pub fn move_isolated_dirs(
    eco_claude_dir: &Path,
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    isolation: &fragment::EcoIsolation,
) -> Result<(), AppError> {
    let prefix = &framework.file_prefix;
    let strategy = framework.dir_layout.strategy();

    let ssot_skill_names = super::plugin_sync::collect_ssot_skill_names(eco_dir);

    for dir_name in &isolation.dirs {
        let src_dir = eco_claude_dir.join(dir_name);
        if !src_dir.exists() || !src_dir.is_dir() {
            continue;
        }
        let dst_dir = eco_dir.join(dir_name);
        fs::create_dir_all(&dst_dir).map_err(|e| AppError::io(&dst_dir, e))?;

        // isolated_dirs（如 gsd-core/）内部结构不应被策略加前缀，直接递归移动
        if framework.isolated_dirs.contains(dir_name) {
            // 将源目录内容移动到目标目录
            if let Ok(entries) = fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue;
                    }

                    // skills 目录：跳过 SSOT 已管理的技能，避免重复注册
                    if dir_name == "skills" && is_skill_duplicated_in_ssot(&name, prefix, framework.files_prefixed, &ssot_skill_names) {
                        log::info!(
                            "跳过移动 skill '{}'：SSOT 已管理同名技能",
                            name
                        );
                        continue;
                    }

                    let dst_path = dst_dir.join(&name);
                    if dst_path.exists() {
                        if dst_path.is_dir() {
                            fs::remove_dir_all(&dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                        } else {
                            fs::remove_file(&dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                        }
                    }
                    fs::rename(&entry.path(), &dst_path).or_else(|_| {
                        fs_utils::copy_path_to(&entry.path(), &dst_path)?;
                        if entry.path().is_dir() {
                            fs::remove_dir_all(&entry.path())
                        } else {
                            fs::remove_file(&entry.path())
                        }
                        .map_err(|e| AppError::io(&entry.path(), e))
                    })?;
                }
            }
            continue;
        }

        // skills 目录：收集 SSOT 已管理的技能名，传递给策略过滤
        let ssot_dedup = if dir_name == "skills" {
            Some(&ssot_skill_names)
        } else {
            None
        };

        strategy.move_from_claude(&src_dir, &dst_dir, prefix, framework.files_prefixed, ssot_dedup)?;
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

/// 处理 npx 安装器写到 HOME 根目录（而非 .claude/）的 isolated_dirs
///
/// 某些框架的 npx 安装器会将特定目录（如 GSD 的 gsd-core/）直接写到
/// HOME 重定向后的根目录（{eco_dir}/gsd-core/），而不是 .claude/ 子目录内。
/// 这些目录已经在正确位置，只需确保它们被 isolation 系统识别。
fn ensure_isolated_dirs_from_eco_root(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
) -> Result<(), AppError> {
    for dir_name in &framework.isolated_dirs {
        let dir_path = eco_dir.join(dir_name);
        if dir_path.exists() && dir_path.is_dir() {
            log::info!(
                "isolated dir '{}' 已在 eco 根目录下（npx 安装器写到 HOME 根）",
                dir_name
            );
        }
    }
    Ok(())
}

/// 从 framework 源码目录补充复制 npx 安装器未部署的目录
///
/// npx 安装器可能不会部署所有 provided_dirs 中的目录（如 scripts/），
/// 此函数检查 eco 目录下缺失的 provided_dirs，从 framework git clone 源码补充复制。
///
/// 对于 isolated_dirs（如 gsd-core/），内部结构不应被策略加前缀，直接递归复制。
/// 对于其他目录（如 skills/、commands/），使用框架的 dir_layout 策略复制。
pub fn supplement_from_framework_source(
    eco_dir: &Path,
    framework: &ecosystem_framework::FrameworkRegistry,
    fw_dir: &Path,
) -> Result<(), AppError> {
    let prefix = &framework.file_prefix;
    let strategy = framework.dir_layout.strategy();

    let ssot_skill_names = super::plugin_sync::collect_ssot_skill_names(eco_dir);

    for dir_name in &framework.provided_dirs {
        let src = fw_dir.join(dir_name);
        if !src.exists() || !src.is_dir() {
            continue;
        }

        // 检查 eco 目录下是否已有该目录的内容
        let dst = eco_dir.join(dir_name);
        if dst.exists() && has_entries(&dst, prefix, framework.files_prefixed) {
            continue;
        }

        // 目录缺失或为空，从 framework 源码补充复制
        log::info!(
            "补充复制 framework 源码目录 '{}' 到 eco（npx 安装器未部署）",
            dir_name
        );

        // 检查 dir_mappings
        if let Some(mapping) = framework
            .dir_mappings
            .iter()
            .find(|(src_name, _)| src_name == dir_name)
        {
            let mapping_dst = eco_dir.join(mapping.1.replace("{id}", &framework.id));
            fs_utils::copy_dir_recursive(&src, &mapping_dst)?;
            continue;
        }

        // isolated_dirs（如 gsd-core/）内部结构不应被策略加前缀，直接递归复制
        if framework.isolated_dirs.contains(dir_name) {
            fs::create_dir_all(&dst).map_err(|e| AppError::io(&dst, e))?;
            fs_utils::copy_dir_recursive(&src, &dst)?;
            continue;
        }

        // skills 目录：传递 SSOT 去重集合，跳过已由 SSOT 管理的技能
        let ssot_dedup = if dir_name == "skills" {
            Some(&ssot_skill_names)
        } else {
            None
        };

        fs::create_dir_all(&dst).map_err(|e| AppError::io(&dst, e))?;
        strategy.copy_to_eco(&src, &dst, prefix, framework.files_prefixed, ssot_dedup)?;
    }

    Ok(())
}

/// 检查目录下是否有带前缀的条目
///
/// 对于 isolated_dirs（如 gsd-core/），其内部文件不以 file_prefix 开头，
/// 但目录本身已在 eco 根目录下，应视为已有内容。
fn has_entries(dir: &Path, prefix: &str, files_prefixed: bool) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            // files_prefixed 时，检查文件名是否以 prefix 开头
            // 但对于目录类型的条目（如 gsd-core/bin/），其名称不以 prefix 开头，
            // 仍应视为有效内容
            if files_prefixed {
                if name.starts_with(prefix) || entry.path().is_dir() {
                    return true;
                }
            } else {
                return true;
            }
        }
    }
    false
}

fn is_skill_duplicated_in_ssot(
    name: &str,
    prefix: &str,
    files_prefixed: bool,
    ssot_names: &std::collections::HashSet<String>,
) -> bool {
    let dst_name = if files_prefixed && name.starts_with(prefix) {
        name.to_string()
    } else {
        format!("{prefix}{name}")
    };
    ssot_names.contains(&name.to_lowercase()) || ssot_names.contains(&dst_name.to_lowercase())
}
