# CC Switch Eco 二开说明

> 本项目基于 [CC Switch](https://github.com/farion1231/cc-switch) 进行二次开发，核心新增 **Ecosystem 生态隔离** 功能。

## 项目信息

| 项目 | 信息 |
|---|---|
| **名称** | CC Switch Eco |
| **Fork 来源** | [farion1231/cc-switch](https://github.com/farion1231/cc-switch) |
| **仓库** | [nicekai-jpg/cc-switch-eco](https://github.com/nicekai-jpg/cc-switch-eco) |
| **版本** | 3.17.0 |
| **技术栈** | Tauri 2 + React + TypeScript + Rust |

---

## 核心新增功能：Ecosystem 生态隔离

### 概念

类似 Python 的 `uv` 虚拟环境，为 Claude Code 创建隔离的运行环境。每个 Ecosystem 拥有独立的：

- `skills/` - 技能目录
- `commands/` - 命令目录
- `hooks/` - 钩子目录
- `agents/` - Agent 目录
- `plugins/` - 插件目录
- `rootfiles/` - 根文件（如 CLAUDE.md、settings.json）

通过 symlink 将 Ecosystem 目录映射到 `~/.claude/`，实现环境切换。

### 目录结构

```
~/.cc-switch-eco/
└── ecosystems/
    ├── default/
    │   ├── skills/
    │   ├── commands/
    │   ├── hooks/
    │   ├── agents/
    │   ├── plugins/
    │   ├── rootfiles/
    │   │   ├── CLAUDE.md
    │   │   └── settings.json
    │   └── eco.json          # 元数据
    ├── work/
    │   └── ...
    └── personal/
        └── ...
```

### 切换机制

```bash
# 切换到 work 生态
# 实际操作：删除 ~/.claude/ 下的 symlink，创建指向 ~/.cc-switch-eco/ecosystems/work/ 的新 symlink

~/.claude/skills → ~/.cc-switch-eco/ecosystems/work/skills
~/.claude/commands → ~/.cc-switch-eco/ecosystems/work/commands
...
```

---

## 支持的 Agent 框架

创建 Ecosystem 时可选择预装以下框架：

| 框架 ID | 名称 | 描述 | 安装方式 |
|---|---|---|---|
| `superpowers` | Superpowers 中文版 | 20 个 skills（14 翻译 + 4 中国原创） | `npx superpowers-zh --tool claude-code` |
| `agency-agents-zh` | Agency Agents 中文版 | 211 个中文 AI 专家角色 | `./scripts/install.sh --tool claude-code` |
| `ohmyclaudecode` | Oh My ClaudeCode | Teams-first 多 Agent 编排，10 Agent + 10 命令 + 9 Hook | `npx oh-my-claude-sisyphus@latest setup` |
| `ruflo` | Ruflo | 100+ 专业 Agent，含记忆系统和 MCP 服务器 | `npx ruflo@latest install` |
| `speckit` | Spec Kit | GitHub 官方规格驱动开发工具包 | `npx @anthropic-ai/spec-kit@latest install` |
| `mattpocock-skills` | Matt Pocock Skills | TypeScript/React 专家技能集 | 手动复制 |
| `gstack` | GStack | 创业技能集（产品/融资/增长） | `npx gstack@latest install` |
| `openspec` | OpenSpec | AI 驱动的规格说明框架 | `npx openspec@latest install` |
| `bmad-method` | BMAD-METHOD | 12+ 专家 Agent、34+ 工作流 | `npx bmad-method@latest install` |
| `get-shit-done` | Get Shit Done | 高效执行技能集 | `npx get-shit-done@latest install` |

### 框架隔离配置

每个框架定义了需要隔离的目录和文件：

```rust
pub struct FrameworkRegistry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub repo_url: String,
    pub install_method: String,        // "npx" | "script" | "copy"
    pub install_command: Option<String>,
    pub install_args: Vec<String>,
    pub isolated_dirs: Vec<String>,    // 额外隔离目录
    pub isolated_files: Vec<String>,   // 隔离的根文件
    pub file_prefix: String,           // 文件前缀（避免冲突）
}
```

例如 Oh My ClaudeCode：
- `isolated_dirs: ["hud"]` - 隔离 hud 目录
- `isolated_files: ["CLAUDE.md", "settings.json"]` - 隔离配置文件
- `file_prefix: "omc-"` - 文件加前缀

---

## 代码改动清单

### 新增文件

| 文件 | 说明 |
|---|---|
| `src-tauri/src/services/ecosystem/mod.rs` | Ecosystem 公共接口 + CRUD + switch（~170 行） |
| `src-tauri/src/services/ecosystem/fragment.rs` | Fragment 路径/列表/重建/合并/用户偏好（~590 行） |
| `src-tauri/src/services/ecosystem/symlink.rs` | Symlink 创建/删除/切换（~170 行） |
| `src-tauri/src/services/ecosystem/framework_ops.rs` | 框架安装/卸载/更新（~470 行） |
| `src-tauri/src/services/ecosystem/migration.rs` | 旧版 Eco 迁移（~50 行） |
| `src-tauri/src/services/ecosystem/fs_utils.rs` | 文件系统工具函数（~50 行） |
| `src-tauri/src/services/ecosystem_framework.rs` | 框架注册表（~240 行） |
| `src-tauri/src/commands/ecosystem.rs` | Tauri 命令定义 |
| `src-tauri/src/database/dao/ecosystems.rs` | Ecosystem DAO |
| `src-tauri/src/database/schema.rs` | 新增 ecosystems 表 |
| `src/components/ecosystem/EcosystemPanel.tsx` | 主面板 + 创建表单（~180 行） |
| `src/components/ecosystem/EcoCard.tsx` | Eco 卡片 + 框架管理（~200 行） |
| `src/components/ecosystem/FrameworkPicker.tsx` | 框架选择器（~60 行） |
| `src/components/ecosystem/ConflictWarning.tsx` | 冲突警告组件（~30 行） |
| `src/hooks/useEcosystem.ts` | React Hook（按职责分组） |
| `src/lib/api/ecosystem.ts` | API 封装 |

### 数据库变更

```sql
CREATE TABLE ecosystems (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    is_current BOOLEAN DEFAULT 0,
    created_at INTEGER
);
```

### API 接口

| 命令 | 说明 |
|---|---|
| `create_ecosystem` | 创建生态 |
| `switch_ecosystem` | 切换生态 |
| `delete_ecosystem` | 删除生态 |
| `list_ecosystems` | 列出所有生态 |
| `get_current_ecosystem` | 获取当前生态 |
| `list_frameworks` | 列出可用框架 |
| `install_framework_to_ecosystem` | 安装框架到生态 |
| `uninstall_framework_from_ecosystem` | 从生态卸载框架 |
| `update_framework_in_ecosystem` | 更新生态中的框架 |
| `get_ecosystem_frameworks` | 获取生态已安装框架 |
| `save_user_preferences` | 保存用户偏好到 user-fragment |
| `remove_user_preference` | 从 user-fragment 移除指定 key |

---

## 多框架根文件合并机制

当一个 Eco 安装多个框架时，多个框架可能共享同一个根文件（如 `settings.json`、`CLAUDE.md`）。为避免配置覆盖和数据丢失，采用 **Per-framework Fragment + 按需重建** 方案。

### 问题背景

| 根文件 | 冲突框架 |
|---|---|
| `CLAUDE.md` | superpowers, ohmyclaudecode, ruflo, speckit |
| `settings.json` | ohmyclaudecode, ruflo, gstack, get-shit-done |
| `mcp.json` | ruflo |

### Fragment 文件命名

每个框架的 JSON 配置存储为独立的 fragment 文件，合并后的根文件由所有 fragment 按安装顺序重建生成：

```
rootfiles/settings.json              ← 合并后的最终文件（symlink 目标）
rootfiles/settings.omc-fragment.json ← OMC 的 fragment
rootfiles/settings.ruflo-fragment.json ← Ruflo 的 fragment
rootfiles/settings.gstack-fragment.json ← GStack 的 fragment
```

命名规则：`<basename>.<prefix>fragment.json`

### 合并规则

- **CLAUDE.md**：追加合并，用 `<!-- prefix -->` 标记分隔（不变）
- **JSON 文件**：深合并
  - 对象：递归合并，相同 key 递归深入
  - 数组：并集去重拼接（两个框架都写 `permissions.allow` 时，合并为包含所有项的数组，重复项只保留一份）
  - 标量：后安装的框架优先（安装顺序从 `eco.json` 的 `frameworks` 数组读取）
- **其他文件**：直接覆盖

### 用户偏好优先机制

合并顺序：框架 fragment（按安装顺序）→ **user-fragment（始终最后）**

```
rootfiles/settings.json                    ← 合并后的最终文件（symlink 目标）
rootfiles/settings.omc-fragment.json       ← OMC 的 fragment
rootfiles/settings.ruflo-fragment.json     ← Ruflo 的 fragment
rootfiles/settings.user-fragment.json      ← 用户偏好（始终优先）
```

- **用户偏好始终优先**：`user-fragment.json` 在所有框架 fragment 之后合并，标量冲突时用户值覆盖框架值，且不记录为冲突
- **自动快照**：切换 Eco 前，自动将当前合并文件保存为 `user-fragment`，确保用户手动修改的配置不丢失
- **手动保存**：前端提供"保存偏好"按钮，用户可主动将当前 settings.json 同步到 user-fragment
- **恢复默认**：`remove_user_preference` 可移除 user-fragment 中的指定 key，下次重建时该 key 恢复为框架默认值

### 标量冲突日志

当两个框架对同一个标量 key 写入不同值时（如 `defaultMode`、`effort`），后安装的框架覆盖先安装的，同时：

1. **Rust 日志**：输出 `log::warn!` 记录冲突详情（格式：`key_path: old_value → new_value (被 prefix 覆盖)`）
2. **eco.json**：将冲突信息写入 `mergeConflicts` 字段，供用户查阅

```json
{
  "mergeConflicts": {
    "settings.json": [
      "defaultMode: bypassPermissions → normal (被 ruflo- 覆盖)",
      "effort: max → medium (被 ruflo- 覆盖)"
    ]
  }
}
```

3. **前端警告**：创建 Eco 时显示冲突提示，说明数组去重拼接规则和标量后写优先规则

### 安装/卸载/更新流程

| 操作 | Fragment 处理 |
|---|---|
| 安装框架 | 保存 fragment → 重建所有 JSON 根文件 |
| 卸载框架 | 删除 fragment → 重建所有 JSON 根文件 |
| 更新框架 | 覆盖 fragment → 重建所有 JSON 根文件 |

### 旧版 Eco 兼容

切换到旧版创建的 Eco 时，如果 `rootfiles/` 中有 JSON 文件但没有 fragment 文件，自动将现有文件迁移为 `user-fragment.json`，确保用户偏好优先于框架配置。

### 前端冲突警告

创建 Eco 选择多个框架时，如果检测到根文件冲突，显示黄色警告：

```
检测到根文件冲突
settings.json: Oh My ClaudeCode + Ruflo
这些文件将自动合并：数组去重拼接，标量冲突时后安装的框架优先。
用户偏好始终优先于框架配置，可随时保存当前配置为偏好。
```

---

## 品牌变更

| 原始 | 变更后 |
|---|---|
| CC Switch | CC Switch Eco |
| `.cc-switch` | `.cc-switch-eco` |
| `cc_switch_lib` | `cc_switch_eco_lib` |

---

## 构建与发布

### 移除的功能

- Tauri 自动更新器（移除 pubkey，禁用 updater artifacts）
- 原有签名流程

### Release Workflow

- 新增 `workflow_dispatch` 触发器
- Tauri 签名改为可选
- Updater artifacts 改为可选

---

## 开发指南

### 本地开发

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 构建
pnpm tauri build
```

### 添加新框架

编辑 `src-tauri/src/services/ecosystem_framework.rs`：

```rust
FrameworkRegistry {
    id: "new-framework".to_string(),
    name: "New Framework".to_string(),
    description: "描述".to_string(),
    repo_url: "https://github.com/...".to_string(),
    repo_branch: "main".to_string(),
    provided_dirs: vec!["skills".to_string()],
    install_method: "npx".to_string(),
    install_command: Some("npx".to_string()),
    install_args: vec!["new-framework".to_string(), "install".to_string()],
    install_env: vec![],
    isolated_dirs: vec![],
    isolated_files: vec!["CLAUDE.md".to_string()],
    file_prefix: "nf-".to_string(),
}
```

---

## 与上游同步

```bash
# 添加上游远程
git remote add upstream https://github.com/farion1231/cc-switch.git

# 获取上游更新
git fetch upstream

# 合并到当前分支
git merge upstream/main
```

---

## License

继承上游 MIT License。
