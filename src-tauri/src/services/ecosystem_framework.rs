//! Agent 框架注册表
//!
//! 定义主流 Claude Code Agent 构建框架的元数据，
//! 用于 Eco 创建时选择安装。
//!
//! 安装策略（遵循各框架官方推荐方式）：
//! - Superpowers 中文版: claude plugin install superpowers-zh@superpowers-zh
//! - Oh My ClaudeCode: claude plugin install oh-my-claudecode@omc
//! - Ruflo: claude plugin install claude-flow@ruflo
//! - Spec Kit: uv tool install specify-cli --from git+... && specify init . --integration claude
//! - Matt Pocock Skills: npx skills@latest add mattpocock/skills -y -a claude-code --copy
//! - GStack: git clone + ./setup（官方推荐方式）
//! - OpenSpec: npx @fission-ai/openspec@latest init --tools claude --force
//! - BMAD-METHOD: npx bmad-method install --yes --modules bmm --tools claude-code
//! - Get Shit Done: npx @opengsd/gsd-core@latest --yes
//! - PUA: claude plugin install pua@pua-skills（HOME 重定向 → eco/plugins，失败回退手动注册）
//! - Web Access: claude plugin install web-access@web-access（HOME 重定向 → eco/plugins）
//! - Claude HUD: plugin（commands + .claude-plugin，安装后自动配置 statusLine）

use crate::services::ecosystem::dir_strategy::DirLayout;
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
    /// 安装方式: "npx" | "script" | "copy" | "plugin" | "uv"
    pub install_method: String,
    /// 官方安装命令（npx 方式为可执行文件名，script 方式为脚本相对路径）
    pub install_command: Option<String>,
    /// 安装命令参数（支持 {fw_dir} {eco_dir} {real_home} 模板变量）
    pub install_args: Vec<String>,
    /// 额外环境变量（key=变量名, value=模版值，支持同上模版变量）
    pub install_env: Vec<(String, String)>,
    /// 框架需要隔离的子目录（相对于 ~/.claude/，如 "helpers"、"hud"）
    pub isolated_dirs: Vec<String>,
    /// 框架需要隔离的根文件（相对于 ~/.claude/，如 "CLAUDE.md"、"settings.json"）
    pub isolated_files: Vec<String>,
    /// 安装文件前缀（用于移动到 Eco 目录时加前缀，如 "superpowers-"）
    pub file_prefix: String,
    /// 源目录内容组织方式
    pub dir_layout: DirLayout,
    /// 源文件名是否已含 file_prefix（如 GSD 的 gsd-advisor-researcher.md）
    pub files_prefixed: bool,
    /// 非标准目录映射（源目录名 → eco 目标路径模板，支持 {id} 变量）
    /// 如 (".claude-plugin", "plugins/{id}")
    pub dir_mappings: Vec<(String, String)>,
    /// 插件 marketplace 名称（仅 plugin 类型框架需要）
    /// Claude Code 的 installed_plugins.json key 格式为 pluginName@marketplaceName
    /// 如 warp 的 marketplace 为 "claude-code-warp"，key 为 "warp@claude-code-warp"
    pub marketplace_name: Option<String>,
    /// Hook 交付方式: "none" | "settings" | "plugin"
    /// - none: 无 hook
    /// - settings: hooks 合并进 settings.json fragment（命令不能含 ${CLAUDE_PLUGIN_ROOT}）
    /// - plugin: 必须通过 Claude Code plugin 注册（hooks 脚本引用 ${CLAUDE_PLUGIN_ROOT}）
    pub hook_delivery: String,
    /// Claude Code 插件名（默认与 id 相同；如 ruflo 官方插件名为 claude-flow）
    pub plugin_name: Option<String>,
}

/// 获取所有注册的框架
pub fn get_all_frameworks() -> Vec<FrameworkRegistry> {
    vec![
        FrameworkRegistry {
            id: "superpowers".to_string(),
            name: "Superpowers 中文版".to_string(),
            description:
                "AI 编程超能力中文增强版，20 个 skills（14 翻译 + 4 中国原创 + 2 上游保留）"
                    .to_string(),
            repo_url: "https://github.com/jnMetaCode/superpowers-zh.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "skills".to_string(),
                "hooks".to_string(),
                ".claude-plugin".to_string(),
            ],
            install_method: "plugin".to_string(),
            install_command: None,
            install_args: vec![],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec!["CLAUDE.md".to_string()],
            file_prefix: "superpowers-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![(
                ".claude-plugin".to_string(),
                "plugins/{id}/.claude-plugin".to_string(),
            )],
            marketplace_name: Some("superpowers-zh".to_string()),
            hook_delivery: "plugin".to_string(),
            plugin_name: Some("superpowers-zh".to_string()),
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
            install_env: vec![(
                "CLAUDE_AGENTS_DIR".to_string(),
                "{eco_dir}/agents".to_string(),
            )],
            isolated_dirs: vec![],
            isolated_files: vec![],
            file_prefix: "agency-".to_string(),
            dir_layout: DirLayout::Recursive,
            files_prefixed: false,
            dir_mappings: vec![],
            marketplace_name: None,
            hook_delivery: "none".to_string(),
            plugin_name: None,
        },
        FrameworkRegistry {
            id: "ohmyclaudecode".to_string(),
            name: "Oh My ClaudeCode".to_string(),
            description: "Teams-first 多 Agent 编排框架，含 10 个 Agent、10 个命令、9 个 Hook"
                .to_string(),
            repo_url: "https://github.com/Yeachan-Heo/oh-my-claudecode.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "agents".to_string(),
                "commands".to_string(),
                "hooks".to_string(),
                "skills".to_string(),
                ".claude-plugin".to_string(),
            ],
            install_method: "plugin".to_string(),
            install_command: None,
            install_args: vec![],
            install_env: vec![],
            isolated_dirs: vec!["hud".to_string()],
            isolated_files: vec!["CLAUDE.md".to_string(), "settings.json".to_string()],
            file_prefix: "omc-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![(
                ".claude-plugin".to_string(),
                "plugins/{id}/.claude-plugin".to_string(),
            )],
            marketplace_name: Some("omc".to_string()),
            hook_delivery: "plugin".to_string(),
            plugin_name: Some("oh-my-claudecode".to_string()),
        },
        FrameworkRegistry {
            id: "ruflo".to_string(),
            name: "Ruflo".to_string(),
            description: "多 Agent AI 编排平台，100+ 专业 Agent 协同工作，含记忆系统和 MCP 服务器"
                .to_string(),
            repo_url: "https://github.com/ruvnet/ruflo.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "skills".to_string(),
                "agents".to_string(),
                "commands".to_string(),
                ".claude-plugin".to_string(),
            ],
            install_method: "plugin".to_string(),
            install_command: None,
            install_args: vec![],
            install_env: vec![],
            isolated_dirs: vec!["helpers".to_string()],
            isolated_files: vec![
                "CLAUDE.md".to_string(),
                "settings.json".to_string(),
                "mcp.json".to_string(),
            ],
            file_prefix: "ruflo-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![(
                ".claude-plugin".to_string(),
                "plugins/{id}/.claude-plugin".to_string(),
            )],
            marketplace_name: Some("ruflo".to_string()),
            hook_delivery: "plugin".to_string(),
            plugin_name: Some("claude-flow".to_string()),
        },
        FrameworkRegistry {
            id: "speckit".to_string(),
            name: "Spec Kit".to_string(),
            description: "GitHub 官方规格驱动开发工具包，提供 specify/plan/tasks/implement 等命令"
                .to_string(),
            repo_url: "https://github.com/github/spec-kit.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string(), "commands".to_string()],
            install_method: "uv".to_string(),
            install_command: Some("uv".to_string()),
            install_args: vec![
                "tool".to_string(),
                "install".to_string(),
                "specify-cli".to_string(),
                "--from".to_string(),
                "git+https://github.com/github/spec-kit.git".to_string(),
            ],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec!["CLAUDE.md".to_string()],
            file_prefix: "speckit-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![],
            marketplace_name: None,
            hook_delivery: "none".to_string(),
            plugin_name: None,
        },
        FrameworkRegistry {
            id: "mattpocock-skills".to_string(),
            name: "Matt Pocock Skills".to_string(),
            description: "TypeScript/React 专家技能集，含类型推断、状态管理等最佳实践".to_string(),
            repo_url: "https://github.com/mattpocock/skills.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec![
                "skills@latest".to_string(),
                "add".to_string(),
                "mattpocock/skills".to_string(),
                "-y".to_string(),
                "-a".to_string(),
                "claude-code".to_string(),
                "--copy".to_string(),
            ],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec![],
            file_prefix: "mp-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![],
            marketplace_name: None,
            hook_delivery: "none".to_string(),
            plugin_name: None,
        },
        FrameworkRegistry {
            id: "gstack".to_string(),
            name: "GStack".to_string(),
            description: "创业技能集，含产品开发、融资、增长等创业全流程 AI 辅助".to_string(),
            repo_url: "https://github.com/garrytan/gstack.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string()],
            install_method: "script".to_string(),
            install_command: Some("setup".to_string()),
            install_args: vec![],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec!["settings.json".to_string()],
            file_prefix: "gstack-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![],
            marketplace_name: None,
            hook_delivery: "none".to_string(),
            plugin_name: None,
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
            install_args: vec![
                "@fission-ai/openspec@latest".to_string(),
                "init".to_string(),
                "--tools".to_string(),
                "claude".to_string(),
                "--force".to_string(),
            ],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec![],
            file_prefix: "openspec-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![],
            marketplace_name: None,
            hook_delivery: "none".to_string(),
            plugin_name: None,
        },
        FrameworkRegistry {
            id: "bmad-method".to_string(),
            name: "BMAD-METHOD".to_string(),
            description:
                "AI 驱动的敏捷开发方法论，12+ 专家 Agent、34+ 工作流，覆盖产品到部署全周期"
                    .to_string(),
            repo_url: "https://github.com/bmad-code-org/BMAD-METHOD.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec!["skills".to_string(), "agents".to_string()],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec![
                "bmad-method".to_string(),
                "install".to_string(),
                "--yes".to_string(),
                "--modules".to_string(),
                "bmm".to_string(),
                "--tools".to_string(),
                "claude-code".to_string(),
            ],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec![],
            file_prefix: "bmad-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![],
            marketplace_name: None,
            hook_delivery: "none".to_string(),
            plugin_name: None,
        },
        FrameworkRegistry {
            id: "get-shit-done".to_string(),
            name: "Get Shit Done".to_string(),
            description: "高效执行技能集，专注快速完成任务，减少 AI 犹豫和过度思考".to_string(),
            repo_url: "https://github.com/open-gsd/gsd-core.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "commands".to_string(),
                "agents".to_string(),
                "hooks".to_string(),
                "get-shit-done".to_string(),
            ],
            install_method: "npx".to_string(),
            install_command: Some("npx".to_string()),
            install_args: vec![
                "@opengsd/gsd-core@latest".to_string(),
                "--yes".to_string(),
            ],
            install_env: vec![],
            isolated_dirs: vec!["get-shit-done".to_string()],
            isolated_files: vec!["settings.json".to_string()],
            file_prefix: "gsd-".to_string(),
            dir_layout: DirLayout::Nested,
            files_prefixed: true,
            dir_mappings: vec![],
            marketplace_name: None,
            hook_delivery: "settings".to_string(),
            plugin_name: None,
        },
        FrameworkRegistry {
            id: "pua".to_string(),
            name: "PUA".to_string(),
            description: "高能动性 Agent 技能，用职场 PUA 话术驱动 AI 穷尽一切方案，含 14 种风格和 L0-L4 压力体系".to_string(),
            repo_url: "https://github.com/tanweai/pua.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "skills".to_string(),
                "agents".to_string(),
                "commands".to_string(),
                "hooks".to_string(),
                ".claude-plugin".to_string(),
            ],
            install_method: "plugin".to_string(),
            install_command: None,
            install_args: vec![],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec![],
            file_prefix: "pua-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![(
                ".claude-plugin".to_string(),
                "plugins/{id}/.claude-plugin".to_string(),
            )],
            marketplace_name: Some("pua-skills".to_string()),
            hook_delivery: "plugin".to_string(),
            plugin_name: Some("pua".to_string()),
        },
        FrameworkRegistry {
            id: "web-access".to_string(),
            name: "Web Access".to_string(),
            description: "完整联网能力 Skill：三层通道调度 + CDP 浏览器自动化 + 站点经验积累，支持 Chrome/Edge".to_string(),
            repo_url: "https://github.com/eze-is/web-access.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "skills".to_string(),
                ".claude-plugin".to_string(),
            ],
            install_method: "plugin".to_string(),
            install_command: None,
            install_args: vec![],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec![],
            file_prefix: "wa-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![(
                ".claude-plugin".to_string(),
                "plugins/{id}".to_string(),
            )],
            marketplace_name: Some("web-access".to_string()),
            hook_delivery: "none".to_string(),
            plugin_name: Some("web-access".to_string()),
        },
        FrameworkRegistry {
            id: "claude-hud".to_string(),
            name: "Claude HUD".to_string(),
            description: "终端 HUD 插件，实时显示当前模型、上下文用量、活跃工具和 Agent 进度（安装后自动配置 statusLine）".to_string(),
            repo_url: "https://github.com/jarrodwatts/claude-hud.git".to_string(),
            repo_branch: "main".to_string(),
            provided_dirs: vec![
                "commands".to_string(),
                ".claude-plugin".to_string(),
                "src".to_string(),
            ],
            install_method: "plugin".to_string(),
            install_command: None,
            install_args: vec![],
            install_env: vec![],
            isolated_dirs: vec![],
            isolated_files: vec![],
            file_prefix: "hud-".to_string(),
            dir_layout: DirLayout::Flat,
            files_prefixed: false,
            dir_mappings: vec![
                // .claude-plugin/ 整体复制到 plugins/{id}/.claude-plugin/
                // 这样 Claude Code 在 installPath 下能找到 .claude-plugin/plugin.json
                (
                    ".claude-plugin".to_string(),
                    "plugins/{id}/.claude-plugin".to_string(),
                ),
                // commands/ 复制到 plugins/{id}/.claude-plugin/commands/
                // 因为 plugin.json 中的路径（如 ./commands/setup.md）相对于 .claude-plugin/
                (
                    "commands".to_string(),
                    "plugins/{id}/.claude-plugin/commands".to_string(),
                ),
            ],
            marketplace_name: Some("claude-hud".to_string()),
            hook_delivery: "plugin".to_string(),
            plugin_name: Some("claude-hud".to_string()),
        },
    ]
}

/// 根据 ID 查找框架
pub fn find_framework(id: &str) -> Option<FrameworkRegistry> {
    get_all_frameworks().into_iter().find(|f| f.id == id)
}

/// Claude Code 插件名（默认 framework id）
pub fn framework_plugin_name(framework: &FrameworkRegistry) -> &str {
    framework
        .plugin_name
        .as_deref()
        .unwrap_or(framework.id.as_str())
}

/// installed_plugins.json / enabledPlugins 的 key：{pluginName}@{marketplace}
pub fn framework_plugin_key(framework: &FrameworkRegistry) -> Option<String> {
    framework
        .marketplace_name
        .as_ref()
        .map(|m| format!("{}@{m}", framework_plugin_name(framework)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pua_uses_plugin_hook_delivery() {
        let pua = find_framework("pua").expect("pua framework should exist");
        assert_eq!(pua.install_method, "plugin");
        assert_eq!(pua.hook_delivery, "plugin");
        assert_eq!(pua.marketplace_name.as_deref(), Some("pua-skills"));
        assert!(
            pua.provided_dirs.contains(&"hooks".to_string()),
            "PUA should ship hooks directory"
        );
    }

    #[test]
    fn test_framework_hook_delivery_consistency() {
        for fw in get_all_frameworks() {
            if fw.hook_delivery == "plugin" {
                assert_eq!(
                    fw.install_method, "plugin",
                    "{}: hook_delivery=plugin 必须配合 install_method=plugin",
                    fw.id
                );
                assert!(
                    fw.marketplace_name.is_some(),
                    "{}: hook_delivery=plugin 必须配置 marketplace_name",
                    fw.id
                );
            }
        }
    }

    #[test]
    fn test_plugin_frameworks_with_hooks_use_plugin_delivery() {
        let plugin_hook_frameworks = [
            ("superpowers", "superpowers-zh"),
            ("ohmyclaudecode", "oh-my-claudecode"),
            ("ruflo", "claude-flow"),
            ("pua", "pua"),
        ];
        for (id, plugin_name) in plugin_hook_frameworks {
            let fw = find_framework(id).expect(id);
            assert_eq!(fw.hook_delivery, "plugin", "{id} should use plugin hooks");
            assert_eq!(
                framework_plugin_name(&fw),
                plugin_name,
                "{id} plugin name mismatch"
            );
        }
    }
}
