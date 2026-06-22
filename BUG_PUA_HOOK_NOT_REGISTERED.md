# BUG REPORT: PUA Skill Hook Not Registered in Claude Code

## Summary

The `pua-pua` skill installed at `~/.claude/skills/pua-pua/` is **not being recognized or loaded** by Claude Code (v2.1.177). Neither explicit invocation (`/pua-pua`) nor implicit triggers (frustration keywords like "再试试", "为什么还不行") activate the skill.

## Environment

- **Claude Code Version:** 2.1.177
- **Platform:** macOS (Darwin 25.5.0)
- **Skill Location:** `~/.claude/skills/pua-pua/`
- **Skill Source:** Installed manually (not via marketplace)

## Root Cause

The `pua-pua` skill is **missing the `.claude-plugin/plugin.json` registration file** that Claude Code requires to discover and load skills.

### Comparison with Working Skill (wa-web-access)

**Working skill structure (`wa-web-access`):**
```
~/.claude/skills/wa-web-access/
├── SKILL.md
├── .claude-plugin/              ← ✅ REQUIRED
│   ├── plugin.json               ← ✅ REQUIRED - registration metadata
│   └── marketplace.json
└── ...
```

**Broken skill structure (`pua-pua`):**
```
~/.claude/skills/pua-pua/
├── SKILL.md                      ← ✅ Present
├── references/                   ← ✅ Present
└── .claude-plugin/               ← ❌ MISSING - entire directory absent
```

### Key Finding

The `wa-web-access` skill contains `.claude-plugin/plugin.json` with:
```json
{
  "name": "web-access",
  "description": "Complete web browsing and automation skill...",
  "version": "2.5.2",
  "skills": ["./"]
}
```

The `pua-pua` skill **has no equivalent file**, so Claude Code's skill loader never registers it.

## Impact

1. **Explicit invocation fails:** `/pua-pua` command is not recognized
2. **Implicit triggers fail:** SKILL.md defines trigger keywords ("再试试", "为什么还不行", "换个方法", etc.) but these never activate the skill because it's not registered
3. **PUA behavior never activates:** No pressure escalation, no flavor switching, no methodology routing

## Evidence

1. Session context shows `pua-pua` in available skills list but it never activates
2. User input `/pua-pua` or trigger phrases → no skill activation
3. File system inspection confirms missing `.claude-plugin/` directory:
   ```bash
   $ ls ~/.claude/skills/pua-pua/.claude-plugin 2>&1
   ls: /Users/limingkai/.claude/skills/pua-pua/.claude-plugin: No such file or directory
   ```

## Potential Fix (NOT APPLIED)

Create the missing registration file:

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

**Note:** Claude Code may require a session restart after adding the plugin.json to recognize the skill.

## Additional Observations

1. **Name mismatch risk:** The skill directory is named `pua-pua` but the SKILL.md frontmatter declares `name: pua`. This discrepancy may cause issues even after registration.

2. **Hook dependency:** The SKILL.md references `SessionStart hook` injection (`[PUA Always-On]`, `Current Flavor`) but there is no evidence these hooks are configured in `~/.claude/settings.json`. Even with plugin.json, the auto-load behavior may require additional configuration.

3. **Settings.json lacks skill configuration:** Current `settings.json` has no `skills` section or hook definitions:
   ```json
   {
     "defaultMode": "bypassPermissions",
     "env": { ... },
     // No "hooks", "skills", or "sessionStart" config
   }
   ```

## Status

- **Severity:** Medium (skill completely non-functional)
- **Workaround:** None available without modifying skill installation
- **Fix applied:** None (user explicitly declined fix)

---
*Reported: 2026-06-18*
*Reporter: limingkai*
