use std::fs;
use std::path::Path;

use crate::error::AppError;

/// 递归复制目录
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dst).map_err(|e| AppError::io(dst, e))?;
    for entry in fs::read_dir(src).map_err(|e| AppError::io(src, e))? {
        let entry = entry.map_err(|e| AppError::io(src, e))?;
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() && !crate::services::ecosystem::symlink::is_symlink(&entry_path) {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else if entry_path.is_file() {
            fs::copy(&entry_path, &dest_path).map_err(|e| AppError::io(&dest_path, e))?;
        }
    }
    Ok(())
}

/// 复制文件或目录到目标路径
pub fn copy_path_to(src: &Path, dst: &Path) -> Result<(), AppError> {
    if src.is_dir() {
        copy_dir_recursive(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }
        fs::copy(src, dst).map_err(|e| AppError::io(dst, e))?;
        Ok(())
    }
}

/// 清理生态 ID（只保留字母、数字、连字符、下划线）
pub fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .to_lowercase()
}
