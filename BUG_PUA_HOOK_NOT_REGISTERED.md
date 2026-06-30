# BUG REPORT: PUA Skill Hook Not Registered in Claude Code

> **⚠️ 历史反复出现的系统性问题 — 已记录 2 次独立根因**
>
> 这个问题不是单一的 bug，而是 PUA skill 安装/注册链路中**多个环节反复断裂**的综合征。每次修复一个点后，下一个断裂点暴露。

---

## 当前问题（本次会话 — 2026-06-23）

### Summary

PUA skill 的 **hooks 完全未注册到 Claude Code**。用户说"你怎么又错了，再试试"等触发词时，没有任何 PUA context 注入。

### 根因

**`~/.claude/hooks/` 目录为空。**

Claude Code 的 hook 机制依赖 `~/.claude/hooks/` 目录。插件安装时，应该将 `hooks/` 下的内容注册（symlink 或复制）到该目录，但这个步骤**没有发生**。

### 证据链

| 检查项 | 结果 | 状态 |
|--------|------|------|
| 插件已安装 | `pua@pua-skills` v3.5.0 | ✅ |
| 插件已启用 | `settings.json` 中 `enabledPlugins` = true | ✅ |
| hooks 源文件存在 | `~/.claude/plugins/cache/pua-skills/pua/3.5.0/hooks/` 有 11 个脚本 | ✅ |
| **`~/.claude/hooks/` 目录** | **空的，没有 `hooks.json`，没有 symlink** | ❌ **根因** |
| `~/.pua/config.json` | 不存在（默认 `always_on: True`） | ✅ |

### 影响

- `UserPromptSubmit` hook（`frustration-trigger.sh`）→ **永不触发**
- `PostToolUse` hook（`failure-detector.sh`）→ **永不触发**
- `SessionStart` hook（`session-restore.sh` + `heartbeat.sh`）→ **永不触发**
- `PreCompact` hook → **永不触发**
- `Stop` hook（`pua-loop-hook.sh` + `stop-feedback.sh`）→ **永不触发**

即：**PUA 的所有自动化行为（压力升级、失败检测、挫败感触发、循环停止）全部失效。**

### 修复方案（未执行）

```bash
# 手动注册 hooks
rm -rf ~/.claude/hooks/*
ln -s ~/.claude/plugins/cache/pua-skills/pua/3.5.0/hooks/* ~/.claude/hooks/
```

---

## 历史问题（第一次 — 2026-06-18）

### Summary

`pua-pua` skill 安装后不被 Claude Code 识别。`/pua-pua` 命令和触发词均无效。

### 根因

**缺少 `.claude-plugin/plugin.json` 注册文件。**

Claude Code 的 skill loader 依赖 `.claude-plugin/plugin.json` 来发现插件。`pua-pua` 目录下没有这个文件，导致 skill 完全不被注册。

### 证据

```
~/.claude/skills/pua-pua/
├── SKILL.md                      ← ✅ Present
├── references/                   ← ✅ Present
└── .claude-plugin/               ← ❌ MISSING - entire directory absent
```

对比正常工作的 skill：
```
~/.claude/skills/wa-web-access/
├── SKILL.md
├── .claude-plugin/              ← ✅ REQUIRED
│   ├── plugin.json               ← ✅ REQUIRED
│   └── marketplace.json
```

### 修复方案（未执行）

```bash
mkdir -p ~/.claude/skills/pua-pua/.claude-plugin
cat > ~/.claude/skills/pua-pua/.claude-plugin/plugin.json << 'EOF'
{
  "name": "pua-pua",
  "description": "PUA/try-harder productivity coaching for Claude Code",
  "version": "2.0.0",
  "license": "MIT",
  "skills": ["./"]
}
EOF
```

---

## 反复出现的模式

| # | 日期 | 问题 | 根因 | 修复状态 |
|---|------|------|------|---------|
| 1 | 2026-06-18 | `/pua-pua` 命令不被识别 | 缺少 `.claude-plugin/plugin.json` | ❌ 未修复 |
| 2 | 2026-06-23 | "又错了"等触发词不触发 PUA | `~/.claude/hooks/` 为空 | ❌ 未修复 |

**共同特征：**
- 都是**安装/注册链路断裂**，而非代码逻辑错误
- 都是**Claude Code 的插件机制与 PUA skill 的注册步骤不匹配**
- 每次修复一个点后，下一个断裂点才暴露
- 用户明确拒绝自动修复，要求文档化

---

## 环境

- **Claude Code Version:** 2.1.177
- **Platform:** macOS (Darwin 25.5.0)
- **PUA Skill Version:** 3.5.0
- **Plugin Install Path:** `~/.claude/plugins/cache/pua-skills/pua/3.5.0`
- **Plugin Source:** GitHub (`tanweai/pua`)

---

## 完整检查清单（供下次排查用）

如果 PUA 再次"不生效"，按此顺序排查：

1. **Plugin 是否安装？**
   ```bash
   cat ~/.claude/plugins/installed_plugins.json
   ```

2. **Plugin 是否启用？**
   ```bash
   cat ~/.claude/settings.json | grep enabledPlugins
   ```

3. **`.claude-plugin/plugin.json` 是否存在？**
   ```bash
   ls ~/.claude/plugins/cache/pua-skills/pua/3.5.0/.claude-plugin/
   ```

4. **`hooks/` 是否注册到 Claude Code？**
   ```bash
   ls -la ~/.claude/hooks/
   ```
   如果为空 → 手动 symlink 或检查安装流程

5. **`~/.pua/config.json` 配置**
   ```bash
   cat ~/.pua/config.json
   ```
   确认 `always_on` 和 `flavor` 设置

6. **手动触发测试**
   说"你怎么又错了，再试试"，检查系统提示中是否有 `<PUA_SKILL_CONTEXT>` 注入

---

## Status

- **Severity:** High（核心功能完全失效）
- **Workaround:** 手动注册 hooks（见上方修复方案）
- **Fix applied:** None（用户明确拒绝）
- **反复次数:** 2 次独立根因，可能还有更多未暴露的断裂点

---

## 新发现（2026-06-23 补充）—— `claude-hud` 框架 hooks 同样未生效

在排查 `fen` 生态时，发现 `claude-hud` 框架也存在 hooks 未注册的问题。

### 根因

框架声明了 `hook_delivery=plugin`，但源码中**缺少 `hooks/hooks.json` 文件**。

### 证据链

| 检查项 | 结果 | 状态 |
|--------|------|------|
| 框架已安装 | `claude-hud` v0.3.0 | ✅ |
| 框架文件存在 | `~/.cc-switch-eco/ecosystems/fen/frameworks/claude-hud/` | ✅ |
| `hooks/hooks.json` | **不存在** | ❌ **根因** |
| `~/.cc-switch-eco/ecosystems/fen/hooks/` | **空的** | ❌ |

### 日志

```
[2026-06-23][11:20:47][WARN] 安装框架 'claude-hud' 失败:
框架「Claude HUD」声明 hook_delivery=plugin，但源码中未找到 hooks/hooks.json
```

### 影响

- `claude-hud` 的 hook 机制完全失效
- 状态栏 HUD 功能可能无法通过 hooks 注入

### 环境

- **生态**: `fen`
- **框架**: `claude-hud` v0.3.0
- **安装路径**: `~/.cc-switch-eco/ecosystems/fen/frameworks/claude-hud/`

---

## 反复出现的模式（更新）

| # | 日期 | 问题 | 根因 | 修复状态 |
|---|------|------|------|---------|
| 1 | 2026-06-18 | `/pua-pua` 命令不被识别 | 缺少 `.claude-plugin/plugin.json` | ❌ 未修复 |
| 2 | 2026-06-23 | "又错了"等触发词不触发 PUA | `~/.claude/hooks/` 为空 | ❌ 未修复 |
| 3 | 2026-06-23 | `claude-hud` 框架 hooks 未生效 | 源码中缺少 `hooks/hooks.json` | ❌ 未修复 |

**共同特征：**
- 都是**安装/注册链路断裂**，而非代码逻辑错误
- 都是**框架/skill 声明了 hook 机制但缺少实际的 hooks.json 文件**
- 每次修复一个点后，下一个断裂点才暴露
- 用户明确拒绝自动修复，要求文档化

---

*First Reported: 2026-06-18*
*Updated: 2026-06-23*
*Reporter: limingkai*
