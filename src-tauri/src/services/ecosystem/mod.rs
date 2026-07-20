//! Ecosystem 生态隔离服务
//!
//! 管理 Claude Code 的生态切换，类似 Python 的 uv 虚拟环境。
//! 每个生态包含独立的 skills/commands/hooks/agents/plugins 目录，
//! 通过 symlink 隔离到 `~/.claude/` 下。

pub mod dir_strategy;
pub mod fragment;
pub mod fragment_rebuild;
pub mod fragment_pref;
pub mod fragment_isolation;
mod framework_ops;
mod fs_utils;
pub mod migration;
mod symlink;
pub mod cmd_utils;
pub mod hud_ops;
pub mod plugin_ops;
pub mod plugin_sync;
pub mod hook_ops;
pub mod installers;
pub mod plugin_install;
pub mod install_utils;
pub mod installer_hooks;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::get_app_config_dir;
use crate::database::Ecosystem;
use crate::error::AppError;
use crate::store::AppState;

/// 生态状态信息（从 eco.json 读取，不存数据库）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EcosystemStatus {
    pub eco_id: String,
    pub frameworks: Vec<String>,
    pub framework_details: HashMap<String, FrameworkDetail>,
    pub merge_conflicts: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub install_errors: Vec<String>,
}

/// 框架安装详情
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkDetail {
    pub installed_at: i64,
    pub commit_hash: String,
}

/// 获取生态根目录 (~/.cc-switch-eco/ecosystems/)
pub fn ecosystems_root() -> PathBuf {
    get_app_config_dir().join("ecosystems")
}

/// 获取指定生态的目录
pub fn ecosystem_dir(id: &str) -> PathBuf {
    ecosystems_root().join(id)
}

pub struct EcosystemService;

impl EcosystemService {
    /// 创建新生态
    pub fn create(
        state: &AppState,
        name: &str,
        description: &str,
        frameworks: Vec<String>,
    ) -> Result<(Ecosystem, Vec<String>), AppError> {
        let id = fs_utils::sanitize_id(name);

        if state.db.ecosystem_exists(&id)? {
            return Err(AppError::Message(format!("生态 '{id}' 已存在")));
        }

        if frameworks.is_empty() {
            return Err(AppError::Message("请至少选择一个框架以创建生态".to_string()));
        }

        // 检查所有选中框架的依赖是否满足，不满足则提前报错
        for fw_id in &frameworks {
            let framework = crate::services::ecosystem_framework::find_framework(fw_id)
                .ok_or_else(|| AppError::Message(format!("框架 '{fw_id}' 不存在")))?;
            framework_ops::check_framework_deps(&framework)?;
        }

        let eco_dir = ecosystem_dir(&id);
        fs::create_dir_all(&eco_dir).map_err(|e| AppError::io(&eco_dir, e))?;

        for dir_name in symlink::BASE_ISOLATED_DIRS {
            let sub_dir = eco_dir.join(dir_name);
            fs::create_dir_all(&sub_dir).map_err(|e| AppError::io(&sub_dir, e))?;
        }

        let rootfiles_dir = eco_dir.join("rootfiles");
        fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

        // 收集初始隔离信息
        let isolation = fragment::collect_framework_isolation(&frameworks);
        let all_dirs = fs_utils::merge_and_dedup(
            symlink::BASE_ISOLATED_DIRS.iter().map(|s| s.to_string()),
            isolation.dirs.into_iter(),
        );

        // 创建 eco.json
        let eco_json = serde_json::json!({
            "name": name,
            "description": description,
            "isolatedDirs": all_dirs,
            "isolatedFiles": isolation.files,
            "frameworks": frameworks,
            "frameworkDetails": {},
        });
        let eco_json_path = eco_dir.join("eco.json");
        let content = fragment::write_json(&eco_json)?;
        fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

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

        // 安装选中的框架，收集所有失败信息
        let mut install_errors: Vec<String> = Vec::new();
        for fw_id in &frameworks {
            if let Err(e) = framework_ops::install_framework(state, &id, fw_id) {
                log::warn!("安装框架 '{fw_id}' 失败: {e}");
                install_errors.push(format!("• {fw_id}: {e}"));
            }
        }

        if !install_errors.is_empty() {
            log::warn!(
                "生态 '{id}' 已创建，但部分框架安装失败：\n{}",
                install_errors.join("\n")
            );
            // 将安装错误持久化到 eco.json
            let eco_json_path = eco_dir.join("eco.json");
            if eco_json_path.exists() {
                if let Ok(content) = fs::read_to_string(&eco_json_path) {
                    if let Ok(mut json) = fragment::parse_json(&content, "解析 eco.json 失败") {
                        if let Some(obj) = json.as_object_mut() {
                            obj.insert(
                                "installErrors".to_string(),
                                serde_json::Value::Array(
                                    install_errors.iter()
                                        .map(|e| serde_json::Value::String(e.clone()))
                                        .collect(),
                                ),
                            );
                        }
                        if let Ok(new_content) = fragment::write_json(&json) {
                            let _ = fs::write(&eco_json_path, new_content);
                        }
                    }
                }
            }
        }

        log::info!("生态 '{id}' 创建成功，自动切换到该生态");
        Self::switch(state, &id)?;
        let eco = Ecosystem {
            is_current: true,
            ..eco
        };
        Ok((eco, install_errors))
    }

    /// 切换到指定生态
    pub fn switch(state: &AppState, id: &str) -> Result<(), AppError> {
        if !state.db.ecosystem_exists(id)? {
            return Err(AppError::Message(format!("生态 '{id}' 不存在")));
        }

        let eco_dir = ecosystem_dir(id);
        if !eco_dir.exists() {
            return Err(AppError::Message(format!(
                "生态目录不存在: {}",
                eco_dir.display()
            )));
        }

        // 切换前：保存当前 Eco 的用户偏好到 user-fragment
        if let Ok(Some(current)) = state.db.get_current_ecosystem() {
            if current.id != id {
                let isolation = fragment::collect_eco_isolation(&ecosystem_dir(&current.id));
                fragment::snapshot_user_preferences(&current.id, &isolation)?;
            }
        }

        state.db.set_current_ecosystem(id)?;
        let eco_dir = symlink::switch_symlinks(id)?;

        // 旧版 Eco 兼容迁移
        let isolation = fragment::collect_eco_isolation(&eco_dir);
        migration::migrate_legacy_rootfiles(&eco_dir, &isolation)?;

        // 从 fragment 重建所有 JSON 根文件
        fragment::rebuild_all_root_files(&eco_dir)?;

        crate::services::provider::ProviderService::sync_current_to_live(state)?;

        crate::claude_settings::reapply_bypass_permissions_if_enabled(&state.db)?;

        log::info!("已切换到生态 '{id}'");
        Ok(())
    }

    /// 删除生态
    pub fn delete(state: &AppState, id: &str) -> Result<(), AppError> {
        let current = state.db.get_current_ecosystem()?;
        if let Some(current) = &current {
            if current.id == id {
                return Err(AppError::Message("不能删除当前激活的生态".to_string()));
            }
        }

        let eco_dir = ecosystem_dir(id);
        if eco_dir.exists() {
            fs::remove_dir_all(&eco_dir).map_err(|e| AppError::io(&eco_dir, e))?;
        }

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

    /// 安装框架
    pub fn install_framework(
        state: &AppState,
        eco_id: &str,
        framework_id: &str,
    ) -> Result<(), AppError> {
        framework_ops::install_framework(state, eco_id, framework_id)
    }

    /// 卸载框架
    pub fn uninstall_framework(
        state: &AppState,
        eco_id: &str,
        framework_id: &str,
    ) -> Result<(), AppError> {
        framework_ops::uninstall_framework(state, eco_id, framework_id)
    }

    /// 更新框架
    pub fn update_framework(
        state: &AppState,
        eco_id: &str,
        framework_id: &str,
    ) -> Result<(), AppError> {
        framework_ops::update_framework(state, eco_id, framework_id)
    }

    /// 获取生态已安装的框架列表
    pub fn get_ecosystem_frameworks(eco_id: &str) -> Result<Vec<String>, AppError> {
        framework_ops::get_ecosystem_frameworks(eco_id)
    }

    /// 获取生态状态（mergeConflicts、installErrors 等）
    pub fn get_status(eco_id: &str) -> Result<EcosystemStatus, AppError> {
        let eco_dir = ecosystem_dir(eco_id);
        let eco_json_path = eco_dir.join("eco.json");

        if !eco_json_path.exists() {
            return Ok(EcosystemStatus {
                eco_id: eco_id.to_string(),
                frameworks: vec![],
                framework_details: HashMap::new(),
                merge_conflicts: HashMap::new(),
                install_errors: vec![],
            });
        }

        let content =
            fs::read_to_string(&eco_json_path).map_err(|e| AppError::io(&eco_json_path, e))?;
        let json: serde_json::Value =
            fragment::parse_json(&content, "解析 eco.json 失败")?;

        let frameworks = json
            .get("frameworks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let framework_details = json
            .get("frameworkDetails")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        let installed_at = v.get("installedAt").and_then(|n| n.as_i64()).unwrap_or(0);
                        let commit_hash = v
                            .get("commitHash")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some((k.clone(), FrameworkDetail { installed_at, commit_hash }))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let merge_conflicts = json
            .get("mergeConflicts")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        let conflicts: Vec<String> = v
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|c| c.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        if conflicts.is_empty() {
                            None
                        } else {
                            Some((k.clone(), conflicts))
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let install_errors = json
            .get("installErrors")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(EcosystemStatus {
            eco_id: eco_id.to_string(),
            frameworks,
            framework_details,
            merge_conflicts,
            install_errors,
        })
    }

    /// 保存用户偏好到 user-fragment
    pub fn save_user_preferences(eco_id: &str, file_name: &str) -> Result<(), AppError> {
        fragment::save_user_preferences(eco_id, file_name)
    }

    /// 从 user-fragment 移除指定 key
    pub fn remove_user_preference(
        eco_id: &str,
        file_name: &str,
        key_path: &str,
    ) -> Result<(), AppError> {
        fragment::remove_user_preference(eco_id, file_name, key_path)
    }
}
