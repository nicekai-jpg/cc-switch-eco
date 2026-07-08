use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem::fragment;

/// 将 settings.json 中 hooks 命令路径从临时 `.claude/hooks/` 改写为 `~/.claude/hooks/`
pub fn rewrite_installer_hook_paths_in_claude_settings(
    eco_claude_dir: &Path,
    eco_dir: &Path,
) -> Result<(), AppError> {
    let settings_path = eco_claude_dir.join("settings.json");
    if !settings_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&settings_path).map_err(|e| AppError::io(&settings_path, e))?;
    let mut settings: serde_json::Value =
        fragment::parse_json(&content, "解析 settings.json 失败")?;

    let old_hooks_dir = path_prefix_for_rewrite(&eco_dir.join(".claude").join("hooks"));
    let new_hooks_dir = path_prefix_for_rewrite(&crate::config::get_claude_config_dir().join("hooks"));

    if let Some(hooks) = settings.get_mut("hooks") {
        rewrite_hook_paths_in_json(hooks, &old_hooks_dir, &new_hooks_dir);
    }

    fs::write(&settings_path, fragment::write_json(&settings)?)
        .map_err(|e| AppError::io(&settings_path, e))?;

    log::info!(
        "已重写 settings hooks 路径: {} → {}",
        old_hooks_dir,
        new_hooks_dir
    );
    Ok(())
}

fn path_prefix_for_rewrite(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches('/').to_string()
}

fn rewrite_hook_paths_in_json(
    value: &mut serde_json::Value,
    old_prefix: &str,
    new_prefix: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "command" {
                    if let Some(command) = child.as_str() {
                        *child = serde_json::Value::String(rewrite_hook_command_path(
                            command, old_prefix, new_prefix,
                        ));
                    }
                } else {
                    rewrite_hook_paths_in_json(child, old_prefix, new_prefix);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                rewrite_hook_paths_in_json(item, old_prefix, new_prefix);
            }
        }
        _ => {}
    }
}

pub fn rewrite_hook_command_path(command: &str, old_prefix: &str, new_prefix: &str) -> String {
    if old_prefix.is_empty() || !command.contains(old_prefix) {
        return command.to_string();
    }
    command.replace(old_prefix, new_prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_hook_command_path() {
        let old = "/tmp/eco/.claude/hooks";
        let new = "/Users/me/.claude/hooks";
        let cmd = r#""/opt/homebrew/bin/node" "/tmp/eco/.claude/hooks/gsd-check-update.js""#;
        let rewritten = rewrite_hook_command_path(cmd, old, new);
        assert!(rewritten.contains("/Users/me/.claude/hooks/gsd-check-update.js"));
        assert!(!rewritten.contains("/tmp/eco/.claude/hooks"));
    }

    #[test]
    fn test_rewrite_installer_hook_paths_in_settings() {
        let dir = tempfile::tempdir().unwrap();
        let eco_dir = dir.path();
        let eco_claude = eco_dir.join(".claude");
        fs::create_dir_all(&eco_claude).unwrap();

        let old_hooks = eco_claude.join("hooks");
        let settings = serde_json::json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!("bash \"{}/gsd-session-state.sh\"", old_hooks.display())
                    }]
                }]
            }
        });
        fs::write(
            eco_claude.join("settings.json"),
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .unwrap();

        rewrite_installer_hook_paths_in_claude_settings(&eco_claude, eco_dir).unwrap();

        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(eco_claude.join("settings.json")).unwrap())
                .unwrap();
        let cmd = updated["hooks"]["SessionStart"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        let expected_prefix = crate::config::get_claude_config_dir().join("hooks");
        assert!(
            cmd.contains(&expected_prefix.to_string_lossy().to_string()),
            "expected {cmd} to contain {}",
            expected_prefix.display()
        );
    }
}
