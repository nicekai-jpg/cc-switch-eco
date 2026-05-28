//! Ecosystem 生态隔离服务
//!
//! 管理 Claude Code 的生态切换，类似 Python 的 uv 虚拟环境。
//! 每个生态包含独立的 skills/commands/hooks/agents/plugins 目录，
//! 通过 symlink 隔离到 `~/.claude/` 下。
//!
//! 框架安装策略（遵循各框架官方推荐方式）：
//! - Superpowers: 官方 /plugin install → 将仓库作为 plugin 安装到 plugins/ 目录
//! - GDS: 官方 /plugin install → 将仓库作为 plugin 安装到 plugins/ 目录
//! - agency-agents-zh: 官方 install.sh --tool claude-code → 将 .md 文件安装到 agents/ 目录
//! - Oh_my_OpenClaude: 官方 cp -R 到 plugins/omo/ → 整个仓库作为 plugin

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
        match framework.install_method.as_str() {
            "plugin" => Self::install_as_plugin(&eco_dir, framework_id, &fw_dir)?,
            "script" => Self::install_via_script(&eco_dir, framework_id, &fw_dir)?,
            _ => return Err(AppError::Message(format!("未知的安装方式: {}", framework.install_method))),
        }

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
        let install_method = framework.as_ref().map(|f| f.install_method.as_str()).unwrap_or("plugin");

        // 按安装方式反向移除文件
        match install_method {
            "plugin" => Self::uninstall_plugin(&eco_dir, framework_id)?,
            "script" => Self::uninstall_script(&eco_dir, framework_id)?,
            _ => {}
        }

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
        let framework = ecosystem_framework::find_framework(framework_id);
        let install_method = framework.as_ref().map(|f| f.install_method.as_str()).unwrap_or("plugin");

        match install_method {
            "plugin" => {
                Self::uninstall_plugin(&eco_dir, framework_id)?;
                Self::install_as_plugin(&eco_dir, framework_id, &fw_dir)?;
            }
            "script" => {
                Self::uninstall_script(&eco_dir, framework_id)?;
                Self::install_via_script(&eco_dir, framework_id, &fw_dir)?;
            }
            _ => {}
        }

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
    // 官方安装方式实现
    // ================================================================

    /// Plugin 方式安装（Superpowers、GDS、Oh_my_OpenClaude）
    ///
    /// 官方方式：/plugin marketplace add <repo> + /plugin install <name>
    /// 实际效果：将仓库目录复制到 ~/.claude/plugins/<name>/
    /// 我们的做法：将仓库目录复制到 Eco 的 plugins/<id>/ 目录
    fn install_as_plugin(eco_dir: &PathBuf, framework_id: &str, fw_dir: &PathBuf) -> Result<(), AppError> {
        match framework_id {
            // Superpowers: 官方 /plugin install swift@claude-superpowers
            // 实际将 plugins/swift/ 复制到 ~/.claude/plugins/swift/
            // 同时 .claude/skills/ 中的技能也会被加载
            "superpowers" => {
                // 安装 plugin（官方方式：复制 plugin 目录到 plugins/）
                let plugin_src = fw_dir.join("plugins").join("swift");
                if plugin_src.exists() {
                    let plugin_dst = eco_dir.join("plugins").join("superpowers-swift");
                    Self::copy_dir_recursive(&plugin_src, &plugin_dst)?;
                }

                // 安装 skills（官方 plugin 系统会自动加载 .claude/skills/）
                let skills_src = fw_dir.join(".claude").join("skills");
                if skills_src.exists() {
                    let skills_dst = eco_dir.join("skills");
                    fs::create_dir_all(&skills_dst).map_err(|e| AppError::io(&skills_dst, e))?;
                    for entry in fs::read_dir(&skills_src).map_err(|e| AppError::io(&skills_src, e))? {
                        let entry = entry.map_err(|e| AppError::io(&skills_src, e))?;
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') {
                            continue;
                        }
                        let dst = skills_dst.join(format!("superpowers-{name}"));
                        if !dst.exists() {
                            if entry.path().is_dir() {
                                Self::copy_dir_recursive(&entry.path(), &dst)?;
                            } else {
                                fs::copy(&entry.path(), &dst).map_err(|e| AppError::io(&dst, e))?;
                            }
                        }
                    }
                }
            }

            // GDS: 官方 /plugin install gds-skills
            // 实际将 skills/ 复制到 ~/.claude/skills/（通过 plugin 系统加载）
            "gds" => {
                let skills_src = fw_dir.join("skills");
                if skills_src.exists() {
                    let skills_dst = eco_dir.join("skills");
                    fs::create_dir_all(&skills_dst).map_err(|e| AppError::io(&skills_dst, e))?;
                    for entry in fs::read_dir(&skills_src).map_err(|e| AppError::io(&skills_src, e))? {
                        let entry = entry.map_err(|e| AppError::io(&skills_src, e))?;
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') {
                            continue;
                        }
                        let dst = skills_dst.join(format!("gds-{name}"));
                        if !dst.exists() {
                            if entry.path().is_dir() {
                                Self::copy_dir_recursive(&entry.path(), &dst)?;
                            } else {
                                fs::copy(&entry.path(), &dst).map_err(|e| AppError::io(&dst, e))?;
                            }
                        }
                    }
                }
            }

            // Oh_my_OpenClaude: 官方 cp -R Oh_my_OpenClaude ~/.claude/plugins/omo
            // 然后运行 claude plugin install --path ~/.claude/plugins/omo
            "ohmyopenclaude" => {
                // 官方方式：整个仓库复制到 plugins/omo/
                let plugin_dst = eco_dir.join("plugins").join("omo");
                if !plugin_dst.exists() {
                    Self::copy_dir_recursive(fw_dir, &plugin_dst)?;
                }

                // 同时将 agents/commands/hooks/skills 也复制到 Eco 对应目录
                // 这样即使 plugin 系统未完全加载，文件也能通过 symlink 生效
                for dir_name in &["agents", "commands", "hooks", "skills"] {
                    let src = fw_dir.join(dir_name);
                    if src.exists() && src.is_dir() {
                        let dst = eco_dir.join(dir_name);
                        fs::create_dir_all(&dst).map_err(|e| AppError::io(&dst, e))?;
                        for entry in fs::read_dir(&src).map_err(|e| AppError::io(&src, e))? {
                            let entry = entry.map_err(|e| AppError::io(&src, e))?;
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.starts_with('.') {
                                continue;
                            }
                            let dst_path = dst.join(format!("omo-{name}"));
                            if !dst_path.exists() {
                                if entry.path().is_dir() {
                                    Self::copy_dir_recursive(&entry.path(), &dst_path)?;
                                } else {
                                    fs::copy(&entry.path(), &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                                }
                            }
                        }
                    }
                }
            }

            _ => {
                return Err(AppError::Message(format!("未知的 plugin 框架: {framework_id}")));
            }
        }

        Ok(())
    }

    /// Script 方式安装（agency-agents-zh）
    ///
    /// 官方方式：./scripts/install.sh --tool claude-code
    /// 实际效果：扫描各分类目录中的 .md 文件，将有 YAML front matter 的文件
    ///          复制到 ~/.claude/agents/（扁平结构，不保留子目录）
    /// 我们的做法：运行 install.sh，但将目标目录设为 Eco 的 agents/ 目录
    fn install_via_script(eco_dir: &PathBuf, framework_id: &str, fw_dir: &PathBuf) -> Result<(), AppError> {
        match framework_id {
            "agency-agents-zh" => {
                let agents_dst = eco_dir.join("agents");
                fs::create_dir_all(&agents_dst).map_err(|e| AppError::io(&agents_dst, e))?;

                // 尝试运行官方 install.sh，设置 CLAUDE_AGENTS_DIR 环境变量指向 Eco 目录
                let script_path = fw_dir.join("scripts").join("install.sh");
                if script_path.exists() {
                    let output = Command::new("bash")
                        .arg(&script_path)
                        .arg("--tool")
                        .arg("claude-code")
                        .env("CLAUDE_AGENTS_DIR", &agents_dst)
                        .env("HOME", eco_dir.parent().unwrap_or(eco_dir))
                        .current_dir(fw_dir)
                        .output();

                    match output {
                        Ok(out) if out.status.success() => {
                            log::info!("agency-agents-zh install.sh 执行成功");
                            return Ok(());
                        }
                        Ok(out) => {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            log::warn!("install.sh 执行失败: {stderr}，回退到手动复制");
                        }
                        Err(e) => {
                            log::warn!("无法执行 install.sh: {e}，回退到手动复制");
                        }
                    }
                }

                // 回退方案：手动复制（模拟 install.sh 的行为）
                // install.sh 会扫描各分类目录，将含 YAML front matter 的 .md 文件
                // 扁平复制到 agents/ 目录
                let categories = [
                    "academic", "design", "engineering", "finance",
                    "game-development", "hr", "integrations", "legal",
                    "marketing", "paid-media", "product", "project-management",
                    "sales", "spatial-computing", "specialized", "strategy",
                    "supply-chain", "support", "testing",
                ];

                for cat in &categories {
                    let cat_dir = fw_dir.join(cat);
                    if !cat_dir.exists() || !cat_dir.is_dir() {
                        continue;
                    }
                    Self::copy_agent_md_files(&cat_dir, &agents_dst, "agency-")?;
                }
            }

            _ => {
                return Err(AppError::Message(format!("未知的 script 框架: {framework_id}")));
            }
        }

        Ok(())
    }

    /// 递归扫描目录，将含 YAML front matter 的 .md 文件扁平复制到目标目录
    /// 模拟 agency-agents-zh 的 install.sh 行为
    fn copy_agent_md_files(src_dir: &PathBuf, dst_dir: &PathBuf, prefix: &str) -> Result<(), AppError> {
        for entry in fs::read_dir(src_dir).map_err(|e| AppError::io(src_dir, e))? {
            let entry = entry.map_err(|e| AppError::io(src_dir, e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') {
                continue;
            }

            if path.is_dir() && !Self::is_symlink(&path) {
                // 递归子目录（如 game-development/unity/）
                Self::copy_agent_md_files(&path, dst_dir, prefix)?;
            } else if path.is_file() && name.ends_with(".md") {
                // 检查是否有 YAML front matter（以 --- 开头）
                if let Ok(content) = fs::read_to_string(&path) {
                    if !content.starts_with("---") {
                        continue;
                    }

                    // 添加前缀避免冲突
                    let dst_path = dst_dir.join(format!("{prefix}{name}"));
                    if !dst_path.exists() {
                        fs::copy(&path, &dst_path).map_err(|e| AppError::io(&dst_path, e))?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 卸载 plugin 方式安装的框架
    fn uninstall_plugin(eco_dir: &PathBuf, framework_id: &str) -> Result<(), AppError> {
        let prefix = match framework_id {
            "superpowers" => "superpowers-",
            "gds" => "gds-",
            "ohmyopenclaude" => "omo-",
            _ => framework_id,
        };

        // 从 plugins/ 目录移除
        let plugins_dir = eco_dir.join("plugins");
        if plugins_dir.exists() {
            match framework_id {
                "superpowers" => {
                    let p = plugins_dir.join("superpowers-swift");
                    if p.exists() {
                        fs::remove_dir_all(&p).map_err(|e| AppError::io(&p, e))?;
                    }
                }
                "ohmyopenclaude" => {
                    let p = plugins_dir.join("omo");
                    if p.exists() {
                        fs::remove_dir_all(&p).map_err(|e| AppError::io(&p, e))?;
                    }
                }
                _ => {}
            }
        }

        // 从各隔离目录移除带前缀的文件
        for dir_name in &["skills", "commands", "hooks", "agents"] {
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

        Ok(())
    }

    /// 卸载 script 方式安装的框架
    fn uninstall_script(eco_dir: &PathBuf, framework_id: &str) -> Result<(), AppError> {
        let prefix = match framework_id {
            "agency-agents-zh" => "agency-",
            _ => framework_id,
        };

        // 从 agents/ 目录移除带前缀的文件
        let agents_dir = eco_dir.join("agents");
        if agents_dir.exists() {
            if let Ok(entries) = fs::read_dir(&agents_dir) {
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
}