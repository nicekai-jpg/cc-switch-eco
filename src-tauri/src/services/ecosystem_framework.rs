//! Agent 框架注册表
//!
//! 定义主流 Claude Code Agent 构建框架的元数据，
//! 用于 Eco 创建时选择安装。
//!
//! 安装策略（遵循各框架官方推荐方式）：
//! - Superpowers: 官方 /plugin install → 将仓库作为 plugin 安装到 plugins/ 目录
//! - GDS: 官方 /plugin install → 将仓库作为 plugin 安装到 plugins/ 目录
//! - agency-agents-zh: 官方 install.sh --tool claude-code → 将 .md 文件安装到 agents/ 目录
//! - Oh_my_OpenClaude: 官方 cp -R 到 plugins/omo/ → 整个仓库作为 plugin

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
    /// 官方推荐安装方式: "plugin" | "script"
    pub install_method: String,
}

/// 框架安装详情（存储在 eco.json 中）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkDetail {
    pub installed_at: i64,
    pub commit_hash: String,
}

/// 获取所有注册的框架
pub fn get_all_frameworks() -> Vec<FrameworkRegistry> {
    vec![
        FrameworkRegistry {
            id: "superpowers".to_string(),
            name: "Superpowers".to_string(),
            description: "Claude Code 插件集合，包含 Swift 开发技能和工具".to_string(),
            repo_url: "https://github.com/ivan-magda/claude-superpowers.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string(), "plugins".to_string()],
            install_method: "plugin".to_string(),
        },
        FrameworkRegistry {
            id: "gds".to_string(),
            name: "GDS Skills".to_string(),
            description: "Godot Development Sprint 技能集，用于游戏开发流程".to_string(),
            repo_url: "https://github.com/iptton-ai/claude-gds-skills.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string()],
            install_method: "plugin".to_string(),
        },
        FrameworkRegistry {
            id: "agency-agents-zh".to_string(),
            name: "Agency Agents 中文版".to_string(),
            description: "211 个即插即用的中文 AI 专家角色，覆盖工程/设计/营销等领域".to_string(),
            repo_url: "https://github.com/jnMetaCode/agency-agents-zh.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["agents".to_string()],
            install_method: "script".to_string(),
        },
        FrameworkRegistry {
            id: "ohmyopenclaude".to_string(),
            name: "Oh My OpenClaude".to_string(),
            description: "多 Agent 编排框架，含 10 个 Agent、10 个命令、9 个 Hook".to_string(),
            repo_url: "https://github.com/mrzhbr/Oh_my_OpenClaude.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "agents".to_string(),
                "commands".to_string(),
                "hooks".to_string(),
                "skills".to_string(),
                "plugins".to_string(),
            ],
            install_method: "plugin".to_string(),
        },
    ]
}

/// 根据 ID 查找框架
pub fn find_framework(id: &str) -> Option<FrameworkRegistry> {
    get_all_frameworks().into_iter().find(|f| f.id == id)
}
