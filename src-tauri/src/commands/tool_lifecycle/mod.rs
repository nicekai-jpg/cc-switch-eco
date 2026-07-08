use std::collections::HashMap;
use std::str::FromStr;
use crate::commands::tool_probe::WslShellPreferenceInput;

pub mod anchors;
pub mod command_builder;
pub mod exec;

// Re-export key functions so they are easily accessible.
#[allow(unused_imports)]
pub use command_builder::{normalize_requested_tools, build_tool_lifecycle_command, ToolLifecycleAction, tool_display_name};
#[allow(unused_imports)]
pub use anchors::{installs_anchored_command, static_fallback_command, anchored_command_from_paths};

#[cfg(target_os = "windows")]
pub use command_builder::wsl_tool_action_shell_command;

#[tauri::command]
pub async fn run_tool_lifecycle_action(
    tools: Vec<String>,
    action: String,
    wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>>,
) -> Result<(), String> {
    let action = ToolLifecycleAction::from_str(&action)?;
    let requested = normalize_requested_tools(&tools);
    if requested.is_empty() {
        return Err("No supported tools selected".to_string());
    }

    let label = match action {
        ToolLifecycleAction::Install => "tool_install",
        ToolLifecycleAction::Update => "tool_update",
    };

    tokio::task::spawn_blocking(move || {
        let command_line =
            build_tool_lifecycle_command(&requested, action, wsl_shell_by_tool.as_ref())?;
        exec::run_tool_lifecycle_silently(&command_line, label)
    })
    .await
    .map_err(|e| format!("tool lifecycle task join error: {e}"))?
}
