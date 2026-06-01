use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem::fragment;

/// 旧版 Eco 兼容：将没有 fragment 的 JSON 根文件迁移为 user-fragment
pub fn migrate_legacy_rootfiles(
    eco_dir: &Path,
    isolation: &fragment::EcoIsolation,
) -> Result<(), AppError> {
    let rootfiles_dir = eco_dir.join("rootfiles");
    if !rootfiles_dir.exists() {
        return Ok(());
    }

    for file_name in &isolation.files {
        if !file_name.ends_with(".json") {
            continue;
        }
        let root_file = rootfiles_dir.join(file_name);
        if !root_file.exists() {
            continue;
        }

        // 检查是否已有 fragment 文件
        let fragments = fragment::list_fragments(&rootfiles_dir, file_name);
        let user_fragment = fragment::fragment_path(&rootfiles_dir, file_name, "user-");
        let has_user_fragment = user_fragment.exists();

        if !fragments.is_empty() || has_user_fragment {
            continue;
        }

        // 旧版 Eco：将现有文件复制为 user fragment（用户偏好始终优先）
        let content = fs::read_to_string(&root_file).map_err(|e| AppError::io(&root_file, e))?;
        if !content.trim().is_empty() && content.trim() != "{}" {
            fs::write(&user_fragment, &content).map_err(|e| AppError::io(&user_fragment, e))?;
            log::info!(
                "旧版 Eco 迁移: {} → {}",
                root_file.display(),
                user_fragment.display()
            );
        }
    }

    Ok(())
}
