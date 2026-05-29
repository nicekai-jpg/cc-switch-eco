//! Agent 框架注册表
//!
//! 定义主流 Claude Code Agent 构建框架的元数据，
//! 用于 Eco 创建时选择安装。
//!
//! 安装策略（遵循各框架官方推荐方式）：
//! - Superpowers 中文版: npx superpowers-zh --tool claude-code
//! - Agency Agents 中文版: ./scripts/install.sh --tool claude-code
//! - Oh My ClaudeCode: npx oh-my-claude-sisyphus@latest setup
//! - Ruflo: npx ruflo@latest install
//! - Spec Kit: npx @anthropic-ai/spec-kit@latest install
//! - Matt Pocock Skills: 手动复制（无 CLI）
//! - GStack: npx gstack@latest install
//! - OpenSpec: npx openspec@latest install
//! - BMAD-METHOD: npx bmad-method@latest install
//! - Get Shit Done: npx get-shit-done@latest install

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
        FrameworkRegistry {
            id: "ruflo".to_string(),
            name: "Ruflo".to_string(),
            description: "多 Agent AI 编排平台，100+ 专业 Agent 协同工作，含记忆系统和 MCP 服务器".to_string(),
            repo_url: "https://github.com/ruvnet/ruflo.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string(), "agents".to_string(), "commands".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec!["ruflo@latest".to_string(), "install".to_string()],
            install_env: vec![],
            file_prefix: "ruflo-".to_string(),
        },
        FrameworkRegistry {
            id: "speckit".to_string(),
            name: "Spec Kit".to_string(),
            description: "GitHub 官方规格驱动开发工具包，提供 specify/plan/tasks/implement 等命令".to_string(),
            repo_url: "https://github.com/github/spec-kit.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string(), "commands".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec!["@anthropic-ai/spec-kit@latest".to_string(), "install".to_string()],
            install_env: vec![],
            file_prefix: "speckit-".to_string(),
        },
        FrameworkRegistry {
            id: "mattpocock-skills".to_string(),
            name: "Matt Pocock Skills".to_string(),
            description: "TypeScript/React 专家技能集，含类型推断、状态管理等最佳实践".to_string(),
            repo_url: "https://github.com/mattpocock/skills.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string()],
            install_method: "copy".to_string(),
            install_command: None,
            install_args: vec![],
            install_env: vec![],
            file_prefix: "mp-".to_string(),
        },
        FrameworkRegistry {
            id: "gstack".to_string(),
            name: "GStack".to_string(),
            description: "创业技能集，含产品开发、融资、增长等创业全流程 AI 辅助".to_string(),
            repo_url: "https://github.com/garrytan/gstack.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec!["gstack@latest".to_string(), "install".to_string()],
            install_env: vec![],
            file_prefix: "gstack-".to_string(),
        },
        FrameworkRegistry {
            id: "openspec".to_string(),
            name: "OpenSpec".to_string(),
            description: "AI 驱动的规格说明框架，自动生成项目规格和任务分解".to_string(),
            repo_url: "https://github.com/Fission-AI/OpenSpec.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string(), "commands".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec!["openspec@latest".to_string(), "install".to_string()],
            install_env: vec![],
            file_prefix: "openspec-".to_string(),
        },
        FrameworkRegistry {
            id: "bmad-method".to_string(),
            name: "BMAD-METHOD".to_string(),
            description: "AI 驱动的敏捷开发方法论，12+ 专家 Agent、34+ 工作流，覆盖产品到部署全周期".to_string(),
            repo_url: "https://github.com/bmad-code-org/BMAD-METHOD.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string(), "agents".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec!["bmad-method@latest".to_string(), "install".to_string()],
            install_env: vec![],
            file_prefix: "bmad-".to_string(),
        },
        FrameworkRegistry {
            id: "get-shit-done".to_string(),
            name: "Get Shit Done".to_string(),
            description: "高效执行技能集，专注快速完成任务，减少 AI 犹豫和过度思考".to_string(),
            repo_url: "https://github.com/gsd-build/get-shit-done.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec!["get-shit-done@latest".to_string(), "install".to_string()],
            install_env: vec![],
            file_prefix: "gsd-".to_string(),
        },
    ]
}

/// 根据 ID 查找框架
pub fn find_framework(id: &str) -> Option<FrameworkRegistry> {
    get_all_frameworks().into_iter().find(|f| f.id == id)
}