use crate::services::ecosystem_framework;
use crate::services::ecosystem::plugin_install::{
    should_use_claude_plugin_cli, validate_hook_delivery
};
use crate::services::ecosystem::hook_ops::{remove_stale_hooks_recursive, inject_plugin_hooks_to_settings};

#[test]
fn test_pua_should_use_claude_plugin_cli() {
    let pua = ecosystem_framework::find_framework("pua").expect("pua exists");
    assert!(should_use_claude_plugin_cli(&pua));
}

#[test]
fn test_web_access_should_use_claude_plugin_cli() {
    let wa = ecosystem_framework::find_framework("web-access").expect("web-access exists");
    assert!(should_use_claude_plugin_cli(&wa));
}

#[test]
fn test_gsd_should_not_use_claude_plugin_cli() {
    let gsd = ecosystem_framework::find_framework("get-shit-done").expect("gsd exists");
    assert!(!should_use_claude_plugin_cli(&gsd));
}

#[test]
fn test_claude_hud_should_not_use_claude_plugin_cli() {
    let hud = ecosystem_framework::find_framework("claude-hud").expect("claude-hud exists");
    assert!(!should_use_claude_plugin_cli(&hud));
}

#[test]
fn test_remove_stale_plugin_hooks_removes_old_version() {
    let mut hooks = serde_json::json!({
        "SessionStart": [
            {
                "matcher": "*",
                "hooks": [
                    {
                        "type": "command",
                        "command": "node \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.0\"/scripts/run.cjs \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.0\"/scripts/session-start.mjs",
                        "timeout": 5
                    },
                    {
                        "type": "command",
                        "command": "node \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2\"/scripts/run.cjs \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2\"/scripts/session-start.mjs",
                        "timeout": 5
                    }
                ]
            }
        ],
        "PreToolUse": [
            {
                "matcher": "*",
                "hooks": [
                    {
                        "type": "command",
                        "command": "node \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.0\"/scripts/run.cjs \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.0\"/scripts/pre-tool-enforcer.mjs",
                        "timeout": 3
                    },
                    {
                        "type": "command",
                        "command": "node \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2\"/scripts/run.cjs \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2\"/scripts/pre-tool-enforcer.mjs",
                        "timeout": 3
                    }
                ]
            }
        ]
    });

    let version_parent = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode";
    let current_marker = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2/";
    let current_marker_no_slash = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2";
    remove_stale_hooks_recursive(&mut hooks, version_parent, current_marker, current_marker_no_slash);

    let session_hooks = hooks["SessionStart"][0]["hooks"].as_array().unwrap();
    assert_eq!(session_hooks.len(), 1, "应只剩 1 个 SessionStart hook");
    let cmd = session_hooks[0]["command"].as_str().unwrap();
    assert!(cmd.contains("4.15.2"), "保留的 hook 应是 4.15.2 版本");
    assert!(!cmd.contains("4.15.0"), "4.15.0 版本 hook 应被移除");

    let pretool_hooks = hooks["PreToolUse"][0]["hooks"].as_array().unwrap();
    assert_eq!(pretool_hooks.len(), 1, "应只剩 1 个 PreToolUse hook");
    let cmd2 = pretool_hooks[0]["command"].as_str().unwrap();
    assert!(cmd2.contains("4.15.2"), "保留的 PreToolUse hook 应是 4.15.2 版本");
}

#[test]
fn test_remove_stale_plugin_hooks_preserves_other_plugins() {
    let mut hooks = serde_json::json!({
        "SessionStart": [
            {
                "matcher": "*",
                "hooks": [
                    {
                        "type": "command",
                        "command": "node \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.0\"/scripts/run.cjs",
                        "timeout": 5
                    },
                    {
                        "type": "command",
                        "command": "node \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2\"/scripts/run.cjs",
                        "timeout": 5
                    },
                    {
                        "type": "command",
                        "command": "bash \"/Users/me/.claude/plugins/cache/pua-skills/pua/3.5.0/hooks/session-restore.sh\"",
                        "timeout": 5
                    }
                ]
            }
        ]
    });

    let version_parent = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode";
    let current_marker = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2/";
    let current_marker_no_slash = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2";
    remove_stale_hooks_recursive(&mut hooks, version_parent, current_marker, current_marker_no_slash);

    let session_hooks = hooks["SessionStart"][0]["hooks"].as_array().unwrap();
    assert_eq!(session_hooks.len(), 2, "OMC 旧版移除，PUA 保留");

    let pua_hook = session_hooks.iter().find(|h| {
        h["command"].as_str().map(|c| c.contains("pua-skills")).unwrap_or(false)
    });
    assert!(pua_hook.is_some(), "PUA hook 应被保留");

    let omc_new = session_hooks.iter().find(|h| {
        h["command"].as_str().map(|c| c.contains("4.15.2")).unwrap_or(false)
    });
    assert!(omc_new.is_some(), "OMC 4.15.2 hook 应被保留");
}

#[test]
fn test_remove_stale_plugin_hooks_cleans_empty_groups() {
    let mut hooks = serde_json::json!({
        "SessionStart": [
            {
                "matcher": "init",
                "hooks": [
                    {
                        "type": "command",
                        "command": "node \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.0\"/scripts/setup-init.mjs",
                        "timeout": 30
                    }
                ]
            },
            {
                "matcher": "*",
                "hooks": [
                    {
                        "type": "command",
                        "command": "node \"/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2\"/scripts/session-start.mjs",
                        "timeout": 5
                    }
                ]
            }
        ]
    });

    let version_parent = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode";
    let current_marker = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2/";
    let current_marker_no_slash = "/Users/me/.claude/plugins/cache/omc/oh-my-claudecode/4.15.2";
    remove_stale_hooks_recursive(&mut hooks, version_parent, current_marker, current_marker_no_slash);

    let session_arr = hooks["SessionStart"].as_array().unwrap();
    assert_eq!(session_arr.len(), 1, "空的 init group 应被移除");
    assert_eq!(
        session_arr[0]["matcher"].as_str(),
        Some("*"),
        "保留的应是 * matcher group"
    );
}

#[test]
fn test_validate_hook_delivery_passes_without_hooks_json() {
    let dir = tempfile::tempdir().unwrap();
    let fw_dir = dir.path();
    let hud = ecosystem_framework::find_framework("claude-hud").expect("claude-hud exists");
    let result = validate_hook_delivery(&hud, fw_dir);
    assert!(result.is_ok(), "hook_delivery=plugin 但无 hooks.json 应通过，got: {result:?}");
}

#[test]
fn test_inject_plugin_hooks_noop_when_no_hooks_json() {
    let dir = tempfile::tempdir().unwrap();
    let eco_dir = dir.path();
    let hud = ecosystem_framework::find_framework("claude-hud").expect("claude-hud exists");
    let result = inject_plugin_hooks_to_settings(eco_dir, &hud);
    assert!(result.is_ok(), "无 hooks.json 时 inject 应返回 Ok(())");
}
