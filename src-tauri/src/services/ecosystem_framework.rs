//! Agent 框架注册表
//!
//! 定义主流 Claude Code Agent 构建框架的元数据，
//! 用于 Eco 创建时选择安装。
//!
//! 安装策略（遵循各框架官方推荐方式）：
//! - Superpowers 中文版: 官方 npx superpowers-zh --tool claude-code
//! - GDS: 官方 /plugin install gds-skills（Claude Code 内部命令，无法外部调用，保留手动复制）
//! - agency-agents-zh: 官方 ./scripts/install.sh --tool claude-code
//! - Oh My ClaudeCode: 官方 omc setup（npm i -g oh-my-claude-sisyphus 后执行）

use serde::{Deserialize, Serialize};

/// 框架注册信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkRegistry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub repo_url: String,
    pub repo_branch: String,
    pub provided_dirs: Vec<String>,
    /// 安装方式: "npx" | "script" | "copy" | "plugin"
    pub install_method: String,
    /// 官方安装命令（npx 方式为可执行文件名，script 方式为脚本相对路径）
    pub install_command: Option<String>,
    /// 安装命令参数（支持 {fw_dir} {eco_dir} {real_home} 模板变量）
    pub install_args: Vec<String>,
    /// 额外环境变量（key=变量名, value=模版值，支持同上模版变量）
    pub install_env: Vec<(String, String)>,
    /// 安装文件前缀（用于移动到 Eco 目录时加前缀，如 "superpowers-"）
    pub file_prefix: String,
}

/// 获取所有注册的框架
pub fn get_all_frameworks() -> Vec<FrameworkRegistry> {
    vec![
        FrameworkRegistry {
            id: "superpowers".to_string(),
            name: "Superpowers 中文版".to_string(),
            description: "AI 编程超能力中文增强版，20 个 skills（14 翻译 + 4 中国原创 + 2 上游保留）".to_string(),
            repo_url: "https://github.com/jnMetaCode/superpowers-zh.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string(), ".claude-plugin".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec!["superpowers-zh".to_string(), "--tool".to_string(), "claude-code".to_string()],
            install_env: vec![],
            file_prefix: "superpowers-".to_string(),
        },
        FrameworkRegistry {
            id: "gds".to_string(),
            name: "GDS Skills".to_string(),
            description: "Godot Development Sprint 技能集，用于游戏开发流程".to_string(),
            repo_url: "https://github.com/iptton-ai/claude-gds-skills.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string()],
            install_method: "plugin".to_string(),
            install_command: None,
            install_args: vec![],
            install_env: vec![],
            file_prefix: "gds-".to_string(),
        },
        FrameworkRegistry {
            id: "agency-agents-zh".to_string(),
            name: "Agency Agents 中文版".to_string(),
            description: "211 个即插即用的中文 AI 专家角色，覆盖工程/设计/营销等领域".to_string(),
            repo_url: "https://github.com/jnMetaCode/agency-agents-zh.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["agents".to_string()],
            install_method: "script".to_string(),
            install_command: Some("scripts/install.sh".to_string()),
            install_args: vec!["--tool".to_string(), "claude-code".to_string()],
            install_env: vec![("CLAUDE_AGENTS_DIR".to_string(), "{eco_dir}/agents".to_string())],
            file_prefix: "agency-".to_string(),
        },
        FrameworkRegistry {
            id: "ohmyclaudecode".to_string(),
            name: "Oh My ClaudeCode".to_string(),
            description: "Teams-first 多 Agent 编排框架，含 10 个 Agent、10 个命令、9 个 Hook".to_string(),
            repo_url: "https://github.com/Yeachan-Heo/oh-my-claudecode.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "agents".to_string(),
                "commands".to_string(),
                "hooks".to_string(),
                "skills".to_string(),
                ".claude-plugin".to_string(),
            ],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec!["oh-my-claude-sisyphus@latest".to_string(), "setup".to_string()],
            install_env: vec![],
            file_prefix: "omc-".to_string(),
        },
    ]
}

/// 根据 ID 查找框架
pub fn find_framework(id: &str) -> Option<FrameworkRegistry> {
    get_all_frameworks().into_iter().find(|f| f.id == id)
}