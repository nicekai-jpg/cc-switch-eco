//! Ecosystem 生态隔离服务
//!
//! 管理 Claude Code 的生态切换，类似 Python 的 uv 虚拟环境。
//! 每个生态包含独立的 skills/commands/hooks/agents/plugins 目录，
//! 通过 symlink 隔离到 `~/.claude/` 下。
//!
//! 框架安装策略（遵循各框架官方推荐方式）：
//! - Superpowers 中文版: 官方 npx superpowers-zh --tool claude-code → 安装到 ~/.claude/ → 观察差异 → 移动到 Eco
//! - GDS: 官方 /plugin install（内部命令，无法外部调用）→ 手动复制到 Eco
//! - agency-agents-zh: 官方 ./scripts/install.sh --tool claude-code → 安装到 ~/.claude/ → 观察差异 → 移动到 Eco
//! - Oh My ClaudeCode: 官方 npx oh-my-claude-sisyphus setup → 安装到 ~/.claude/ → 观察差异 → 移动到 Eco

use std::fs;
#[cfg(target_family = "unix")]
use std::os::unix::fs as unix_fs;
#[cfg(target_family = "windows")]
use std::os::windows::fs as windows_fs;
use std::path::PathBuf;
use std::process::Command;

use crate::config::get_app_config_dir;
use crate::database::Ecosystem;
use crate::error::AppError;
use crate::services::ecosystem_framework;
use crate::store::AppState;

/// 需要隔离的目录列表
const ISOLATED_DIRS: &[&str] = &["skills", "commands", "hooks", "agents", "plugins"];

pub struct EcosystemService;

impl EcosystemService {
    /// 获取生态根目录 (~/.cc-switch-eco/ecosystems/)
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
    pub fn create(state: &AppState, name: &str, description: &str, frameworks: Vec<String>) -> Result<Ecosystem, AppError> {
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
            "frameworks": frameworks,
            "frameworkDetails": {},
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

        // 安装选中的框架
        for fw_id in &frameworks {
            if let Err(e) = Self::install_framework(state, &id, fw_id) {
                log::warn!("安装框架 '{fw_id}' 失败: {e}");
            }
        }

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
        #[cfg(target_family = "unix")]
        {
            unix_fs::symlink(target, link)
                .map_err(|e| AppError::Message(format!(
                    "创建符号链接失败: {} → {}: {e}",
                    link.display(),
                    target.display()
                )))?;
        }
        #[cfg(target_family = "windows")]
        {
            windows_fs::symlink_dir(target, link)
                .map_err(|e| AppError::Message(format!(
                    "创建符号链接失败: {} → {}: {e}",
                    link.display(),
                    target.display()
                )))?;
        }
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

    // ================================================================
    // 框架安装/卸载/更新 — 遵循官方推荐方式
    // ================================================================

    /// 安装框架到指定生态
    ///
    /// 安装流程：
    /// 1. git clone 框架仓库到 frameworks/<id>/（获取源码，不可避免）
    /// 2. 按各框架官方推荐方式将文件安装到 Eco 隔离目录
    /// 3. 更新 eco.json
    pub fn install_framework(state: &AppState, eco_id: &str, framework_id: &str) -> Result<(), AppError> {
        let framework = ecosystem_framework::find_framework(framework_id)
            .ok_or_else(|| AppError::Message(format!("框架 '{framework_id}' 不存在")))?;

        if !state.db.ecosystem_exists(eco_id)? {
            return Err(AppError::Message(format!("生态 '{eco_id}' 不存在")));
        }

        let eco_dir = Self::ecosystem_dir(eco_id);
        let fw_dir = eco_dir.join("frameworks").join(framework_id);

        // 检查是否已安装
        if fw_dir.exists() {
            return Err(AppError::Message(format!("框架 '{framework_id}' 已安装在生态 '{eco_id}' 中")));
        }

        // Step 1: git clone 获取源码
        fs::create_dir_all(fw_dir.parent().unwrap()).map_err(|e| AppError::io(&fw_dir, e))?;

        let output = Command::new("git")
            .args(["clone", "--depth", "1", "--branch", &framework.repo_branch, &framework.repo_url, fw_dir.to_str().unwrap_or("")])
            .output()
            .map_err(|e| AppError::Message(format!("执行 git clone 失败: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = fs::remove_dir_all(&fw_dir);
            return Err(AppError::Message(format!("git clone 失败: {stderr}")));
        }

        // Step 2: 按官方方式安装文件到 Eco 隔离目录
        let install_result = match framework.install_method.as_str() {
            "npx" | "script" => Self::install_via_official_command(&eco_dir, &framework, &fw_dir),
            "plugin" | "copy" => Self::install_manual_copy(&eco_dir, &framework, &fw_dir),
            _ => Err(AppError::Message(format!("未知的安装方式: {}", framework.install_method))),
        };

        // 官方命令失败时回退到手动复制
        let install_result = match install_result {
            Ok(()) => Ok(()),
            Err(e) => {
                log::warn!("官方安装命令失败: {e}，回退到手动复制");
                Self::install_manual_copy(&eco_dir, &framework, &fw_dir)
            }
        };

        install_result?;

        // Step 3: 获取 commit hash 并更新 eco.json
        let commit_hash = Self::get_git_commit_hash(&fw_dir).unwrap_or_default();
        Self::update_eco_json_frameworks(&eco_dir, framework_id, &commit_hash)?;

        log::info!("框架 '{framework_id}' 已安装到生态 '{eco_id}'");
        Ok(())
    }

    /// 卸载框架
    pub fn uninstall_framework(state: &AppState, eco_id: &str, framework_id: &str) -> Result<(), AppError> {
        if !state.db.ecosystem_exists(eco_id)? {
            return Err(AppError::Message(format!("生态 '{eco_id}' 不存在")));
        }

        let eco_dir = Self::ecosystem_dir(eco_id);
        let fw_dir = eco_dir.join("frameworks").join(framework_id);

        if !fw_dir.exists() {
            return Err(AppError::Message(format!("框架 '{framework_id}' 未安装在生态 '{eco_id}' 中")));
        }

        let framework = ecosystem_framework::find_framework(framework_id);

        // 统一使用前缀匹配卸载
        let prefix = framework.as_ref().map(|f| f.file_prefix.as_str()).unwrap_or(framework_id);
        Self::uninstall_by_prefix(&eco_dir, prefix, framework_id)?;

        // 删除框架 git 仓库
        fs::remove_dir_all(&fw_dir).map_err(|e| AppError::io(&fw_dir, e))?;

        // 更新 eco.json
        Self::remove_eco_json_framework(&eco_dir, framework_id)?;

        log::info!("框架 '{framework_id}' 已从生态 '{eco_id}' 卸载");
        Ok(())
    }

    /// 更新框架（git pull + 重新安装）
    pub fn update_framework(state: &AppState, eco_id: &str, framework_id: &str) -> Result<(), AppError> {
        if !state.db.ecosystem_exists(eco_id)? {
            return Err(AppError::Message(format!("生态 '{eco_id}' 不存在")));
        }

        let eco_dir = Self::ecosystem_dir(eco_id);
        let fw_dir = eco_dir.join("frameworks").join(framework_id);

        if !fw_dir.exists() {
            return Err(AppError::Message(format!("框架 '{framework_id}' 未安装在生态 '{eco_id}' 中")));
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

        // 先卸载旧文件，再重新安装
        let framework = ecosystem_framework::find_framework(framework_id)
            .ok_or_else(|| AppError::Message(format!("框架 '{framework_id}' 不存在")))?;

        let prefix = framework.file_prefix.as_str();
        Self::uninstall_by_prefix(&eco_dir, prefix, framework_id)?;

        // 重新安装
        let install_result = match framework.install_method.as_str() {
            "npx" | "script" => Self::install_via_official_command(&eco_dir, &framework, &fw_dir),
            "plugin" | "copy" => Self::install_manual_copy(&eco_dir, &framework, &fw_dir),
            _ => Err(AppError::Message(format!("未知的安装方式: {}", framework.install_method))),
        };

        let install_result = match install_result {
            Ok(()) => Ok(()),
            Err(e) => {
                log::warn!("官方安装命令失败: {e}，回退到手动复制");
                Self::install_manual_copy(&eco_dir, &framework, &fw_dir)
            }
        };

        install_result?;

        // 更新 eco.json
        let commit_hash = Self::get_git_commit_hash(&fw_dir).unwrap_or_default();
        Self::update_eco_json_frameworks(&eco_dir, framework_id, &commit_hash)?;

        log::info!("框架 '{framework_id}' 在生态 '{eco_id}' 中已更新");
        Ok(())
    }

    /// 获取生态已安装的框架列表
    pub fn get_ecosystem_frameworks(eco_id: &str) -> Result<Vec<String>, AppError> {
        let eco_dir = Self::ecosystem_dir(eco_id);
        let eco_json_path = eco_dir.join("eco.json");

        if !eco_json_path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&eco_json_path)
            .map_err(|e| AppError::io(&eco_json_path, e))?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| AppError::Message(format!("解析 eco.json 失败: {e}")))?;

        let frameworks = json.get("frameworks")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(frameworks)
    }

    // ================================================================
    // 官方安装方式：快照-安装-对比-移动
    // ================================================================

    /// 使用官方命令安装框架（npx / script 方式）
    ///
    /// 流程：
    /// 1. 在 Eco 目录下创建 .claude/ 目录结构
    /// 2. 运行官方安装命令（设置 HOME=<eco_dir> 使其写入 Eco 的 .claude/ 目录）
    /// 3. 将 .claude/ 中的文件移动到 Eco 的 skills/agents/commands/hooks/ 目录，加前缀
    /// 4. 清理 Eco 的 .claude/ 目录
    fn install_via_official_command(
        eco_dir: &PathBuf,
        framework: &ecosystem_framework::FrameworkRegistry,
        fw_dir: &PathBuf,
    ) -> Result<(), AppError> {
        // Step 1: 在 Eco 目录下创建 .claude/ 目录结构，供 HOME 重定向使用
        let eco_claude_dir = eco_dir.join(".claude");
        for sub_dir in &["skills", "agents", "commands", "hooks", "plugins"] {
            fs::create_dir_all(eco_claude_dir.join(sub_dir))
                .map_err(|e| AppError::io(&eco_claude_dir.join(sub_dir), e))?;
        }

        // Step 2: 运行官方安装命令
        let real_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let result = match framework.install_method.as_str() {
            "npx" => Self::run_npx_command(eco_dir, framework, fw_dir, &real_home),
            "script" => Self::run_script_command(eco_dir, framework, fw_dir, &real_home),
            _ => Err(AppError::Message(format!("不支持的安装方式: {}", framework.install_method))),
        };

        if let Err(e) = result {
            // 官方命令失败，清理 .claude/ 目录后返回错误（由调用方回退到手动复制）
            let _ = fs::remove_dir_all(&eco_claude_dir);
            return Err(AppError::Message(format!("官方安装命令失败: {e}")));
        }

        // Step 4: 将 Eco 的 .claude/ 中的文件移动到 Eco 对应目录
        // （.claude/ 是我们刚创建的空目录，官方命令写入的文件都是新的）
        Self::move_claude_files_to_eco(&eco_claude_dir, eco_dir, &framework.file_prefix)?;

        // Step 5: 清理 Eco 的 .claude/ 目录
        let _ = fs::remove_dir_all(&eco_claude_dir);

        Ok(())
    }

    /// 将 Eco 的 .claude/ 目录下的文件移动到 Eco 对应的 skills/agents/ 等目录
    ///
    /// 例如：.claude/skills/brainstorming/ → skills/superpowers-brainstorming/
    fn move_claude_files_to_eco(
        eco_claude_dir: &PathBuf,
        eco_dir: &PathBuf,
        prefix: &str,
    ) -> Result<(), AppError> {
        for dir_name in ISOLATED_DIRS {
            let src_dir = eco_claude_dir.join(dir_name);
            if !src_dir.exists() || !src_dir.is_dir() {
                continue;
            }
            let dst_dir = eco_dir.join(dir_name);
            fs::create_dir_all(&dst_dir).map_err(|e| AppError::io(&dst_dir, e))?;

            if let Ok(entries) = fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue;
                    }
                    let dst_name = format!("{prefix}{name}");
                    let dst_path = dst_dir.join(&dst_name);
                    if dst_path.exists() {
                        if dst_path.is_dir() {
                            fs::remove_dir_all(&dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                        } else {
                            fs::remove_file(&dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                        }
                    }
                    fs::rename(entry.path(), &dst_path)
                        .or_else(|_| {
                            // rename 跨设备可能失败，回退到 copy + remove
                            Self::copy_path_to(&entry.path(), &dst_path)?;
                            if entry.path().is_dir() {
                                fs::remove_dir_all(entry.path())
                            } else {
                                fs::remove_file(entry.path())
                            }
                            .map_err(|e| AppError::io(&entry.path(), e))
                        })?;
                }
            }
        }

        // 处理 .claude/ 根目录的文件（如 CLAUDE.md）
        if let Ok(entries) = fs::read_dir(eco_claude_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "skills" || name == "agents"
                    || name == "commands" || name == "hooks" || name == "plugins" {
                    continue;
                }
                // 将 CLAUDE.md 等文件移动到 Eco 根目录，加前缀
                let dst_name = format!("{prefix}{name}");
                let dst_path = eco_dir.join(&dst_name);
                if entry.path().is_file() {
                    fs::copy(entry.path(), &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                }
            }
        }

        Ok(())
    }

    /// 运行 npx 安装命令
    fn run_npx_command(
        eco_dir: &PathBuf,
        framework: &ecosystem_framework::FrameworkRegistry,
        _fw_dir: &PathBuf,
        _real_home: &PathBuf,
    ) -> Result<(), AppError> {
        let command = framework.install_command.as_deref().ok_or_else(|| {
            AppError::Message(format!("框架 '{}' 未配置安装命令", framework.id))
        })?;

        let args: Vec<String> = framework.install_args.iter()
            .map(|arg| Self::resolve_template(arg, eco_dir, _fw_dir, _real_home))
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
        eco_dir: &PathBuf,
        framework: &ecosystem_framework::FrameworkRegistry,
        fw_dir: &PathBuf,
        _real_home: &PathBuf,
    ) -> Result<(), AppError> {
        let script_relative = framework.install_command.as_deref().ok_or_else(|| {
            AppError::Message(format!("框架 '{}' 未配置安装脚本", framework.id))
        })?;

        let script_path = fw_dir.join(script_relative);
        if !script_path.exists() {
            return Err(AppError::Message(format!("安装脚本不存在: {}", script_path.display())));
        }

        let args: Vec<String> = framework.install_args.iter()
            .map(|arg| Self::resolve_template(arg, eco_dir, fw_dir, _real_home))
            .collect();

        let mut cmd = Command::new("bash");
        cmd.arg(&script_path)
            .args(&args)
            .env("HOME", eco_dir)
            .current_dir(fw_dir);

        // 添加额外环境变量
        for (key, value) in &framework.install_env {
            let resolved = Self::resolve_template(value, eco_dir, fw_dir, _real_home);
            cmd.env(key, resolved);
        }

        let output = cmd.output()
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

    /// 解析模板变量
    fn resolve_template(template: &str, eco_dir: &PathBuf, fw_dir: &PathBuf, real_home: &PathBuf) -> String {
        template
            .replace("{eco_dir}", eco_dir.to_str().unwrap_or(""))
            .replace("{fw_dir}", fw_dir.to_str().unwrap_or(""))
            .replace("{real_home}", real_home.to_str().unwrap_or(""))
    }

    // ================================================================
    // 手动复制（回退方案 & copy/plugin 方式）
    // ================================================================

    /// 手动复制框架文件到 Eco 目录（官方命令失败时的回退方案）
    ///
    /// 将仓库的 provided_dirs 中的文件复制到 Eco 对应目录，加前缀。
    fn install_manual_copy(
        eco_dir: &PathBuf,
        framework: &ecosystem_framework::FrameworkRegistry,
        fw_dir: &PathBuf,
    ) -> Result<(), AppError> {
        for dir_name in &framework.provided_dirs {
            let src = fw_dir.join(dir_name);
            if !src.exists() {
                continue;
            }

            // .claude-plugin 特殊处理：复制到 plugins/ 目录
            if dir_name == ".claude-plugin" {
                let plugin_dst = eco_dir.join("plugins").join(&framework.id);
                if !plugin_dst.exists() {
                    Self::copy_dir_recursive(&src, &plugin_dst)?;
                }
                continue;
            }

            if !src.is_dir() {
                continue;
            }

            let dst = eco_dir.join(dir_name);
            fs::create_dir_all(&dst).map_err(|e| AppError::io(&dst, e))?;

            if let Ok(entries) = fs::read_dir(&src) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue;
                    }
                    let dst_name = format!("{}{}", framework.file_prefix, name);
                    let dst_path = dst.join(&dst_name);
                    if !dst_path.exists() {
                        Self::copy_path_to(&entry.path(), &dst_path)?;
                    }
                }
            }
        }

        // agency-agents-zh 特殊处理：需要递归扫描分类目录中的 .md 文件
        if framework.id == "agency-agents-zh" {
            Self::copy_agency_agents_fallback(fw_dir, eco_dir, &framework.file_prefix)?;
        }

        Ok(())
    }

    /// agency-agents-zh 回退方案：递归扫描分类目录，将含 YAML front matter 的 .md 文件扁平复制
    fn copy_agency_agents_fallback(fw_dir: &PathBuf, eco_dir: &PathBuf, prefix: &str) -> Result<(), AppError> {
        let agents_dst = eco_dir.join("agents");
        fs::create_dir_all(&agents_dst).map_err(|e| AppError::io(&agents_dst, e))?;

        for entry in fs::read_dir(fw_dir).map_err(|e| AppError::io(fw_dir, e))? {
            let entry = entry.map_err(|e| AppError::io(fw_dir, e))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "scripts" || name == "frameworks" {
                continue;
            }
            Self::copy_agent_md_files(&path, &agents_dst, prefix)?;
        }

        Ok(())
    }

    /// 递归扫描目录，将含 YAML front matter 的 .md 文件扁平复制到目标目录
    fn copy_agent_md_files(src_dir: &PathBuf, dst_dir: &PathBuf, prefix: &str) -> Result<(), AppError> {
        for entry in fs::read_dir(src_dir).map_err(|e| AppError::io(src_dir, e))? {
            let entry = entry.map_err(|e| AppError::io(src_dir, e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') {
                continue;
            }

            if path.is_dir() && !Self::is_symlink(&path) {
                Self::copy_agent_md_files(&path, dst_dir, prefix)?;
            } else if path.is_file() && name.ends_with(".md") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if !content.starts_with("---") {
                        continue;
                    }
                    let dst_path = dst_dir.join(format!("{prefix}{name}"));
                    if !dst_path.exists() {
                        fs::copy(&path, &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                    }
                }
            }
        }

        Ok(())
    }

    // ================================================================
    // 卸载
    // ================================================================

    /// 按前缀卸载框架文件
    fn uninstall_by_prefix(eco_dir: &PathBuf, prefix: &str, _framework_id: &str) -> Result<(), AppError> {
        // 从各隔离目录移除带前缀的文件
        for dir_name in ISOLATED_DIRS {
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

        // 从 Eco 根目录移除带前缀的文件（如 CLAUDE.md 等）
        if let Ok(entries) = fs::read_dir(eco_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(prefix) && entry.path().is_file() {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        // 清理可能残留的 .claude/ 目录
        let eco_claude_dir = eco_dir.join(".claude");
        if eco_claude_dir.exists() {
            let _ = fs::remove_dir_all(&eco_claude_dir);
        }

        Ok(())
    }

    /// 获取 git 仓库的当前 commit hash
    fn get_git_commit_hash(repo_dir: &PathBuf) -> Option<String> {
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

    /// 更新 eco.json 中的框架信息
    fn update_eco_json_frameworks(eco_dir: &PathBuf, framework_id: &str, commit_hash: &str) -> Result<(), AppError> {
        let eco_json_path = eco_dir.join("eco.json");

        let mut json: serde_json::Value = if eco_json_path.exists() {
            let content = fs::read_to_string(&eco_json_path)
                .map_err(|e| AppError::io(&eco_json_path, e))?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
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
        if let Some(obj) = map.get_mut("frameworkDetails").and_then(|v| v.as_object_mut()) {
            let now = chrono::Utc::now().timestamp_millis();
            obj.insert(framework_id.to_string(), serde_json::json!({
                "installedAt": now,
                "commitHash": commit_hash,
            }));
        }

        let content = serde_json::to_string_pretty(&json).unwrap_or_default();
        fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

        Ok(())
    }

    /// 从 eco.json 中移除框架信息
    fn remove_eco_json_framework(eco_dir: &PathBuf, framework_id: &str) -> Result<(), AppError> {
        let eco_json_path = eco_dir.join("eco.json");

        if !eco_json_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&eco_json_path)
            .map_err(|e| AppError::io(&eco_json_path, e))?;
        let mut json: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

        // 从 frameworks 数组中移除
        if let Some(arr) = json.get_mut("frameworks").and_then(|v| v.as_array_mut()) {
            arr.retain(|v| v.as_str() != Some(framework_id));
        }

        // 从 frameworkDetails 中移除
        if let Some(obj) = json.get_mut("frameworkDetails").and_then(|v| v.as_object_mut()) {
            obj.remove(framework_id);
        }

        let content = serde_json::to_string_pretty(&json).unwrap_or_default();
        fs::write(&eco_json_path, content).map_err(|e| AppError::io(&eco_json_path, e))?;

        Ok(())
    }

    /// 复制文件或目录到目标路径
    fn copy_path_to(src: &PathBuf, dst: &PathBuf) -> Result<(), AppError> {
        if src.is_dir() {
            Self::copy_dir_recursive(src, dst)
        } else {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
            }
            fs::copy(src, dst).map_err(|e| AppError::io(dst, e))?;
            Ok(())
        }
    }
}