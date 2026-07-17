//! 框架目录复制策略
//!
//! 定义不同框架源目录结构的复制/移动策略，
//! 使用策略模式实现开闭原则：新增布局方式只需添加策略实现。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem::fs_utils;

use serde::{Deserialize, Serialize};

/// 源目录内容组织方式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DirLayout {
    /// 直接文件，安装时添加前缀
    Flat,
    /// 有命名空间子目录，需展开（如 commands/gsd/ → commands/）
    Nested,
    /// 递归扫描子目录，扁平化复制（如 agency-agents 的分类目录）
    Recursive,
}

impl DirLayout {
    /// 工厂方法：根据布局类型创建对应的策略实例
    pub fn strategy(&self) -> Box<dyn DirCopyStrategy> {
        match self {
            DirLayout::Flat => Box::new(FlatStrategy),
            DirLayout::Nested => Box::new(NestedStrategy),
            DirLayout::Recursive => Box::new(RecursiveStrategy),
        }
    }
}

/// 目录复制策略接口
///
/// 每种框架源目录结构对应一个策略实现。
/// 新增布局方式只需实现此 trait + 在 DirLayout 添加枚举变体。
pub trait DirCopyStrategy: Send + Sync {
    /// 从框架源目录复制文件到 eco 目标目录（手动安装路径）
    fn copy_to_eco(
        &self,
        src: &Path,
        dst: &Path,
        prefix: &str,
        files_prefixed: bool,
        ssot_dedup: Option<&HashSet<String>>,
    ) -> Result<(), AppError>;

    /// 从 .claude/ 临时目录移动文件到 eco 目标目录（官方安装路径）
    fn move_from_claude(
        &self,
        src_dir: &Path,
        dst_dir: &Path,
        prefix: &str,
        files_prefixed: bool,
        ssot_dedup: Option<&HashSet<String>>,
    ) -> Result<(), AppError>;
}

// ================================================================
// Flat 策略：直接复制/移动，添加前缀
// ================================================================

pub struct FlatStrategy;

impl DirCopyStrategy for FlatStrategy {
    fn copy_to_eco(
        &self,
        src: &Path,
        dst: &Path,
        prefix: &str,
        files_prefixed: bool,
        ssot_dedup: Option<&HashSet<String>>,
    ) -> Result<(), AppError> {
        copy_entries(src, dst, prefix, files_prefixed, ssot_dedup)
    }

    fn move_from_claude(
        &self,
        src_dir: &Path,
        dst_dir: &Path,
        prefix: &str,
        files_prefixed: bool,
        ssot_dedup: Option<&HashSet<String>>,
    ) -> Result<(), AppError> {
        move_entries(src_dir, dst_dir, prefix, files_prefixed, ssot_dedup)
    }
}

// ================================================================
// Nested 策略：展开前缀子目录 + 前缀去重
// ================================================================

pub struct NestedStrategy;

impl DirCopyStrategy for NestedStrategy {
    fn copy_to_eco(
        &self,
        src: &Path,
        dst: &Path,
        prefix: &str,
        files_prefixed: bool,
        ssot_dedup: Option<&HashSet<String>>,
    ) -> Result<(), AppError> {
        // 展开前缀子目录（如 commands/gsd/ → commands/）
        let prefix_ns = prefix.strip_suffix('-').unwrap_or(prefix);
        let prefix_subdir = src.join(prefix_ns);
        if prefix_subdir.exists() && prefix_subdir.is_dir() {
            copy_entries(&prefix_subdir, dst, prefix, files_prefixed, ssot_dedup)?;
        }
        // 复制源目录下的直接文件和 skill 目录
        copy_entries_nested(&src, dst, prefix, files_prefixed, ssot_dedup)
    }

    fn move_from_claude(
        &self,
        src_dir: &Path,
        dst_dir: &Path,
        prefix: &str,
        files_prefixed: bool,
        ssot_dedup: Option<&HashSet<String>>,
    ) -> Result<(), AppError> {
        if let Ok(entries) = fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }

                // 展开前缀子目录（非 skill 目录）
                if entry.path().is_dir() && name.starts_with(prefix) {
                    // skill 目录包含 SKILL.md，保留目录结构整体移动
                    if entry.path().join("SKILL.md").exists() {
                        let dst_name = compute_dst_name(&name, prefix, files_prefixed);
                        if should_skip_ssot_dedup(&name, &dst_name, ssot_dedup) {
                            log::info!("跳过移动 skill '{}'：SSOT 已管理同名技能", name);
                            continue;
                        }
                        let dst_path = dst_dir.join(&dst_name);
                        remove_existing(&dst_path)?;
                        move_entry(&entry.path(), &dst_path)?;
                        continue;
                    }

                    // 非 skill 目录：展开子目录内容
                    if let Ok(sub_entries) = fs::read_dir(&entry.path()) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_name = sub_entry.file_name().to_string_lossy().to_string();
                            if sub_name.starts_with('.') {
                                continue;
                            }
                            let dst_name = compute_dst_name(&sub_name, prefix, files_prefixed);
                            if should_skip_ssot_dedup(&sub_name, &dst_name, ssot_dedup) {
                                continue;
                            }
                            let dst_path = dst_dir.join(&dst_name);
                            remove_existing(&dst_path)?;
                            move_entry(&sub_entry.path(), &dst_path)?;
                        }
                    }
                    if let Err(e) = fs::remove_dir_all(&entry.path()) {
                        log::warn!("清理子目录失败 {}: {e}", entry.path().display());
                    }
                    continue;
                }

                // 普通文件
                let dst_name = compute_dst_name(&name, prefix, files_prefixed);
                if should_skip_ssot_dedup(&name, &dst_name, ssot_dedup) {
                    continue;
                }
                let dst_path = dst_dir.join(&dst_name);
                remove_existing(&dst_path)?;
                move_entry(&entry.path(), &dst_path)?;
            }
        }
        Ok(())
    }
}

// ================================================================
// Recursive 策略：递归扫描子目录，扁平化复制
// ================================================================

pub struct RecursiveStrategy;

impl DirCopyStrategy for RecursiveStrategy {
    fn copy_to_eco(
        &self,
        src: &Path,
        dst: &Path,
        prefix: &str,
        files_prefixed: bool,
        ssot_dedup: Option<&HashSet<String>>,
    ) -> Result<(), AppError> {
        copy_recursive_flat(src, dst, prefix, files_prefixed, ssot_dedup)
    }

    fn move_from_claude(
        &self,
        src_dir: &Path,
        dst_dir: &Path,
        prefix: &str,
        files_prefixed: bool,
        ssot_dedup: Option<&HashSet<String>>,
    ) -> Result<(), AppError> {
        // 官方安装路径下，recursive 框架的文件通常已在扁平结构中
        move_entries(src_dir, dst_dir, prefix, files_prefixed, ssot_dedup)
    }
}

// ================================================================
// 共享辅助函数
// ================================================================

/// 计算目标文件名：如果 files_prefixed 且文件名已含前缀，跳过重复添加
fn compute_dst_name(name: &str, prefix: &str, files_prefixed: bool) -> String {
    if files_prefixed && name.starts_with(prefix) {
        name.to_string()
    } else {
        format!("{prefix}{name}")
    }
}

/// 复制目录内容到目标目录
fn copy_entries(
    src_dir: &Path,
    dst_dir: &Path,
    prefix: &str,
    files_prefixed: bool,
    ssot_dedup: Option<&HashSet<String>>,
) -> Result<(), AppError> {
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let dst_name = compute_dst_name(&name, prefix, files_prefixed);
            if should_skip_ssot_dedup(&name, &dst_name, ssot_dedup) {
                log::info!("跳过复制 skill '{}'：SSOT 已管理同名技能", name);
                continue;
            }
            let dst_path = dst_dir.join(&dst_name);
            if !dst_path.exists() {
                fs_utils::copy_path_to(&entry.path(), &dst_path)?;
            }
        }
    }
    Ok(())
}

/// 移动目录内容到目标目录
fn move_entries(
    src_dir: &Path,
    dst_dir: &Path,
    prefix: &str,
    files_prefixed: bool,
    ssot_dedup: Option<&HashSet<String>>,
) -> Result<(), AppError> {
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let dst_name = compute_dst_name(&name, prefix, files_prefixed);
            if should_skip_ssot_dedup(&name, &dst_name, ssot_dedup) {
                log::info!("跳过移动 skill '{}'：SSOT 已管理同名技能", name);
                continue;
            }
            let dst_path = dst_dir.join(&dst_name);
            remove_existing(&dst_path)?;
            move_entry(&entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

/// 复制目录内容到目标目录（Nested 策略专用）
///
/// 与 copy_entries 不同，此函数对包含 SKILL.md 的子目录保留目录结构整体复制，
/// 而不是展开其内容。这确保 GSD 的 skill 目录（如 gsd-plan-phase/SKILL.md）
/// 不会被错误地展开为扁平文件。
fn copy_entries_nested(
    src_dir: &Path,
    dst_dir: &Path,
    prefix: &str,
    files_prefixed: bool,
    ssot_dedup: Option<&HashSet<String>>,
) -> Result<(), AppError> {
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }

            // skill 目录包含 SKILL.md，保留目录结构整体复制
            if entry.path().is_dir() && name.starts_with(prefix) && entry.path().join("SKILL.md").exists() {
                let dst_name = compute_dst_name(&name, prefix, files_prefixed);
                if should_skip_ssot_dedup(&name, &dst_name, ssot_dedup) {
                    log::info!("跳过复制 skill '{}'：SSOT 已管理同名技能", name);
                    continue;
                }
                let dst_path = dst_dir.join(&dst_name);
                if !dst_path.exists() {
                    fs_utils::copy_dir_recursive(&entry.path(), &dst_path)?;
                }
                continue;
            }

            let dst_name = compute_dst_name(&name, prefix, files_prefixed);
            if should_skip_ssot_dedup(&name, &dst_name, ssot_dedup) {
                continue;
            }
            let dst_path = dst_dir.join(&dst_name);
            if !dst_path.exists() {
                fs_utils::copy_path_to(&entry.path(), &dst_path)?;
            }
        }
    }
    Ok(())
}

/// 递归扫描子目录，扁平化复制所有文件到目标目录
fn copy_recursive_flat(
    src_dir: &Path,
    dst_dir: &Path,
    prefix: &str,
    files_prefixed: bool,
    ssot_dedup: Option<&HashSet<String>>,
) -> Result<(), AppError> {
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                copy_recursive_flat(&path, dst_dir, prefix, files_prefixed, ssot_dedup)?;
            } else {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".md") || name.starts_with('.') {
                    continue;
                }
                let dst_name = compute_dst_name(&name, prefix, files_prefixed);
                if should_skip_ssot_dedup(&name, &dst_name, ssot_dedup) {
                    continue;
                }
                let dst_path = dst_dir.join(&dst_name);
                if !dst_path.exists() {
                    fs::copy(&path, &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                }
            }
        }
    }
    Ok(())
}

/// 判断技能是否应因 SSOT 去重而跳过。
/// 检查原始名和带前缀的目标名是否都已在 SSOT 中存在。
fn should_skip_ssot_dedup(
    original_name: &str,
    dst_name: &str,
    ssot_dedup: Option<&HashSet<String>>,
) -> bool {
    match ssot_dedup {
        Some(names) => {
            names.contains(&original_name.to_lowercase())
                || names.contains(&dst_name.to_lowercase())
        }
        None => false,
    }
}

/// 删除已存在的目标路径
fn remove_existing(dst_path: &Path) -> Result<(), AppError> {
    if dst_path.exists() {
        if dst_path.is_dir() {
            fs::remove_dir_all(dst_path).map_err(|e| AppError::io(dst_path, e))?;
        } else {
            fs::remove_file(dst_path).map_err(|e| AppError::io(dst_path, e))?;
        }
    }
    Ok(())
}

/// 移动文件/目录，跨设备时回退到复制+删除
fn move_entry(src: &Path, dst: &Path) -> Result<(), AppError> {
    fs::rename(src, dst).or_else(|_| {
        fs_utils::copy_path_to(src, dst)?;
        if src.is_dir() {
            fs::remove_dir_all(src)
        } else {
            fs::remove_file(src)
        }
        .map_err(|e| AppError::io(src, e))
    })
}
