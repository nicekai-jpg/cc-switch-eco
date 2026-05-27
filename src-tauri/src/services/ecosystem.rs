//! Ecosystem 生态隔离服务
//!
//! 管理 Claude Code 的生态切换，类似 Python 的 uv 虚拟环境。
//! 每个生态包含独立的 skills/commands/hooks/agents/plugins 目录，
//! 通过 symlink 隔离到 `~/.claude/` 下。

use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::PathBuf;

use crate::config::get_app_config_dir;
use crate::database::Ecosystem;
use crate::error::AppError;
use crate::store::AppState;

/// 需要隔离的目录列表
const ISOLATED_DIRS: &[&str] = &["skills", "commands", "hooks", "agents", "plugins"];

pub struct EcosystemService;

impl EcosystemService {
    /// 获取生态根目录 (~/.cc-switch/ecosystems/)
    fn ecosystems_root() -> PathBuf {
        get_app_config_dir().join("ecosystems")
    }

    /// 获取指定生态的目录
    fn ecosystem_dir(id: &str) -> PathBuf {
        Self::ecosystems_root().join(id)
    }

    /// 获取 Claude 配置目录 (~/.claude/)
    fn claude_dir() -> PathBuf {
        crate::config::get_claude_config_dir()
    }

    /// 创建新生态
    pub fn create(state: &AppState, name: &str, description: &str) -> Result<Ecosystem, AppError> {
        let id = Self::sanitize_id(name);

        if state.db.ecosystem_exists(&id)? {
            return Err(AppError::Message(format!("生态 '{id}' 已存在")));
        }

        // 创建生态目录及子目录
        let eco_dir = Self::ecosystem_dir(&id);
        fs::create_dir_all(&eco_dir).map_err(|e| AppError::io(&eco_dir, e))?;

        for dir_name in ISOLATED_DIRS {
            let sub_dir = eco_dir.join(dir_name);
            fs::create_dir_all(&sub_dir).map_err(|e| AppError::io(&sub_dir, e))?;
        }

        // 创建 eco.json 元数据文件
        let eco_json = serde_json::json!({
            "name": name,
            "description": description,
            "isolatedDirs": ISOLATED_DIRS,
        });
        let eco_json_path = eco_dir.join("eco.json");
        fs::write(&eco_json_path, serde_json::to_string_pretty(&eco_json).unwrap_or_default())
            .map_err(|e| AppError::io(&eco_json_path, e))?;

        // 保存到 DB
        let now = chrono::Utc::now().timestamp_millis();
        let eco = Ecosystem {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            is_current: false,
            created_at: now,
        };
        state.db.save_ecosystem(&eco)?;

        log::info!("生态 '{id}' 创建成功");
        Ok(eco)
    }

    /// 切换到指定生态
    pub fn switch(state: &AppState, id: &str) -> Result<(), AppError> {
        if !state.db.ecosystem_exists(id)? {
            return Err(AppError::Message(format!("生态 '{id}' 不存在")));
        }

        let eco_dir = Self::ecosystem_dir(id);
        if !eco_dir.exists() {
            return Err(AppError::Message(format!(
                "生态目录不存在: {}",
                eco_dir.display()
            )));
        }

        // 更新 DB 当前生态
        state.db.set_current_ecosystem(id)?;

        // 切换 symlink
        Self::switch_symlinks(id)?;

        // 重新写入 settings.json（合并生态字段）
        crate::services::provider::ProviderService::sync_current_to_live(state)?;

        log::info!("已切换到生态 '{id}'");
        Ok(())
    }

    /// 删除生态
    pub fn delete(state: &AppState, id: &str) -> Result<(), AppError> {
        // 不能删除当前激活的生态
        let current = state.db.get_current_ecosystem()?;
        if let Some(current) = &current {
            if current.id == id {
                return Err(AppError::Message("不能删除当前激活的生态".to_string()));
            }
        }

        // 删除生态目录
        let eco_dir = Self::ecosystem_dir(id);
        if eco_dir.exists() {
            fs::remove_dir_all(&eco_dir).map_err(|e| AppError::io(&eco_dir, e))?;
        }

        // 删除 DB 记录
        state.db.delete_ecosystem(id)?;

        log::info!("生态 '{id}' 已删除");
        Ok(())
    }

    /// 列出所有生态
    pub fn list(state: &AppState) -> Result<Vec<Ecosystem>, AppError> {
        state.db.get_all_ecosystems()
    }

    /// 获取当前生态
    pub fn get_current(state: &AppState) -> Result<Option<Ecosystem>, AppError> {
        state.db.get_current_ecosystem()
    }

    /// 切换 ~/.claude/ 下的 symlink 指向指定生态
    fn switch_symlinks(id: &str) -> Result<(), AppError> {
        let claude_dir = Self::claude_dir();
        let eco_dir = Self::ecosystem_dir(id);

        for dir_name in ISOLATED_DIRS {
            let claude_path = claude_dir.join(dir_name);
            let eco_path = eco_dir.join(dir_name);

            // 确保生态子目录存在
            fs::create_dir_all(&eco_path).map_err(|e| AppError::io(&eco_path, e))?;

            // 如果 claude_path 已存在（是目录或 symlink），先处理
            if claude_path.exists() || Self::is_symlink(&claude_path) {
                // 如果已经是 symlink，直接删除
                if Self::is_symlink(&claude_path) {
                    fs::remove_file(&claude_path)
                        .map_err(|e| AppError::io(&claude_path, e))?;
                } else if claude_path.is_dir() {
                    // 如果是真实目录，需要先备份再替换
                    Self::backup_and_replace_dir(&claude_path, &eco_path, dir_name)?;
                }
            }

            // 创建 symlink
            Self::create_symlink(&eco_path, &claude_path)?;
        }

        Ok(())
    }

    /// 备份真实目录内容到生态目录，然后删除真实目录
    fn backup_and_replace_dir(
        claude_path: &PathBuf,
        eco_path: &PathBuf,
        dir_name: &str,
    ) -> Result<(), AppError> {
        // 将真实目录的内容复制到生态目录
        if claude_path.is_dir() {
            for entry in fs::read_dir(claude_path).map_err(|e| AppError::io(claude_path, e))? {
                let entry = entry.map_err(|e| AppError::io(claude_path, e))?;
                let src = entry.path();
                let dst = eco_path.join(entry.file_name());

                if src.is_dir() {
                    // 如果目标已存在，跳过（避免覆盖生态已有内容）
                    if dst.exists() {
                        continue;
                    }
                    Self::copy_dir_recursive(&src, &dst)?;
                } else if src.is_file() {
                    if !dst.exists() {
                        fs::copy(&src, &dst).map_err(|e| AppError::io(&dst, e))?;
                    }
                }
                // 跳过 symlink（避免递归）
            }

            // 删除真实目录
            fs::remove_dir_all(claude_path).map_err(|e| AppError::io(claude_path, e))?;
        }

        log::info!("已备份 ~/.claude/{dir_name} 内容到生态目录");
        Ok(())
    }

    /// 递归复制目录
    fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<(), AppError> {
        fs::create_dir_all(dst).map_err(|e| AppError::io(dst, e))?;
        for entry in fs::read_dir(src).map_err(|e| AppError::io(src, e))? {
            let entry = entry.map_err(|e| AppError::io(src, e))?;
            let entry_path = entry.path();
            let dest_path = dst.join(entry.file_name());

            if entry_path.is_dir() && !Self::is_symlink(&entry_path) {
                Self::copy_dir_recursive(&entry_path, &dest_path)?;
            } else if entry_path.is_file() {
                fs::copy(&entry_path, &dest_path).map_err(|e| AppError::io(&dest_path, e))?;
            }
        }
        Ok(())
    }

    /// 创建符号链接
    fn create_symlink(target: &PathBuf, link: &PathBuf) -> Result<(), AppError> {
        unix_fs::symlink(target, link)
            .map_err(|e| AppError::Message(format!(
                "创建符号链接失败: {} → {}: {e}",
                link.display(),
                target.display()
            )))?;
        Ok(())
    }

    /// 检查路径是否是符号链接
    fn is_symlink(path: &PathBuf) -> bool {
        fs::symlink_metadata(path)
            .map(|m| m.is_symlink())
            .unwrap_or(false)
    }

    /// 清理生态 ID（只保留字母、数字、连字符、下划线）
    fn sanitize_id(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect::<String>()
            .to_lowercase()
    }
}