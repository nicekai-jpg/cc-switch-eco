use std::collections::HashMap;
use std::str::FromStr;
use crate::commands::tool_probe::{VALID_TOOLS, WslShellPreferenceInput};
use crate::commands::tool_probe::enumerate::enumerate_tool_installations;
use super::anchors::{
    installs_anchored_command, static_fallback_command,
    prefers_official_update, bare_official_update_command, chain_update_commands,
    npm_install_command_for, install_command_for,
    HERMES_INSTALL_UNIX, HERMES_UPDATE_UNIX,
};

#[cfg(target_os = "windows")]
use crate::commands::tool_probe::version::{default_flag_for_shell, is_valid_shell, is_valid_shell_flag};
#[cfg(target_os = "windows")]
use super::anchors::{posix_install_command_for, static_fallback_command_for};

#[cfg(target_os = "windows")]
use crate::commands::tool_probe::wsl::{wsl_distro_for_tool, is_valid_wsl_distro_name};

pub fn normalize_requested_tools(tools: &[String]) -> Vec<&'static str> {
    let set: std::collections::HashSet<&str> = tools.iter().map(|s| s.as_str()).collect();
    VALID_TOOLS
        .iter()
        .copied()
        .filter(|tool| set.contains(tool))
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub enum ToolLifecycleAction {
    Install,
    Update,
}

impl FromStr for ToolLifecycleAction {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "install" => Ok(Self::Install),
            "update" => Ok(Self::Update),
            _ => Err(format!("Unsupported tool action: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LifecycleCommandShell {
    Posix,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    WindowsBatch,
}

pub fn build_tool_lifecycle_command(
    tools: &[&str],
    action: ToolLifecycleAction,
    wsl_shell_by_tool: Option<&HashMap<String, WslShellPreferenceInput>>,
) -> Result<String, String> {
    let mut lines = Vec::new();

    #[cfg(not(target_os = "windows"))]
    {
        lines.push("set -e".to_string());
        lines.push("set -o pipefail".to_string());
    }

    #[cfg(target_os = "windows")]
    lines.push("@echo off".to_string());

    for tool in tools {
        let label = tool_display_name(tool);
        lines.push(format!("echo ========== {label} =========="));

        let pref = wsl_shell_by_tool.and_then(|m| m.get(*tool));
        let line = build_tool_action_line(
            tool,
            action,
            pref.and_then(|p| p.wsl_shell.as_deref()),
            pref.and_then(|p| p.wsl_shell_flag.as_deref()),
        )?;
        lines.push(line);

        #[cfg(target_os = "windows")]
        lines.push("if errorlevel 1 exit /b %errorlevel%".to_string());

        #[cfg(not(target_os = "windows"))]
        lines.push(String::new());
    }

    Ok(lines.join(if cfg!(target_os = "windows") {
        "\r\n"
    } else {
        "\n"
    }))
}

pub fn tool_display_name(tool: &str) -> &'static str {
    match tool {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        "opencode" => "OpenCode",
        "openclaw" => "OpenClaw",
        "hermes" => "Hermes",
        _ => "Unknown",
    }
}

#[cfg(target_os = "windows")]
const HERMES_INSTALL_WINDOWS_SCRIPT: &str =
    "irm https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.ps1 | iex";

#[cfg(target_os = "windows")]
fn powershell_encoded_command(script: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    STANDARD.encode(bytes)
}

#[cfg(target_os = "windows")]
pub fn hermes_install_windows_command() -> String {
    format!(
        "powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand {}",
        powershell_encoded_command(HERMES_INSTALL_WINDOWS_SCRIPT)
    )
}

#[cfg(target_os = "windows")]
pub fn hermes_update_windows_command() -> String {
    format!("hermes update || {}", hermes_install_windows_command())
}

pub fn tool_action_shell_command_for_shell(
    tool: &str,
    action: ToolLifecycleAction,
    shell: LifecycleCommandShell,
) -> Option<String> {
    if tool == "hermes" {
        return Some(
            match (action, shell) {
                (ToolLifecycleAction::Install, LifecycleCommandShell::Posix) => HERMES_INSTALL_UNIX,
                (ToolLifecycleAction::Update, LifecycleCommandShell::Posix) => HERMES_UPDATE_UNIX,
                #[cfg(target_os = "windows")]
                (ToolLifecycleAction::Install, LifecycleCommandShell::WindowsBatch) => {
                    return Some(hermes_install_windows_command());
                }
                #[cfg(target_os = "windows")]
                (ToolLifecycleAction::Update, LifecycleCommandShell::WindowsBatch) => {
                    return Some(hermes_update_windows_command());
                }
                #[cfg(not(target_os = "windows"))]
                (_, LifecycleCommandShell::WindowsBatch) => return None,
            }
            .to_string(),
        );
    }

    let install = npm_install_command_for(tool)?;
    match action {
        ToolLifecycleAction::Install => Some(install.to_string()),
        ToolLifecycleAction::Update => match prefers_official_update(tool, shell)
            .then(|| bare_official_update_command(tool))
            .flatten()
        {
            Some(update) => Some(chain_update_commands(update, install.to_string(), shell)),
            None => Some(install.to_string()),
        },
    }
}

pub fn tool_action_shell_command(tool: &str, action: ToolLifecycleAction) -> Option<String> {
    #[cfg(target_os = "windows")]
    let shell = LifecycleCommandShell::WindowsBatch;
    #[cfg(not(target_os = "windows"))]
    let shell = LifecycleCommandShell::Posix;

    tool_action_shell_command_for_shell(tool, action, shell)
}

#[cfg(target_os = "windows")]
pub fn wsl_tool_action_shell_command(tool: &str, action: ToolLifecycleAction) -> Option<String> {
    match action {
        ToolLifecycleAction::Install => {
            let command = posix_install_command_for(tool);
            if command.is_empty() {
                None
            } else {
                Some(command)
            }
        }
        ToolLifecycleAction::Update => {
            tool_action_shell_command_for_shell(tool, action, LifecycleCommandShell::Posix)
        }
    }
}

pub fn build_tool_action_line(
    tool: &str,
    action: ToolLifecycleAction,
    wsl_shell: Option<&str>,
    wsl_shell_flag: Option<&str>,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(distro) = wsl_distro_for_tool(tool) {
            let command = wsl_tool_action_shell_command(tool, action)
                .ok_or_else(|| format!("Unsupported tool action target: {tool}"))?;
            return build_wsl_tool_action_line(&distro, &command, wsl_shell, wsl_shell_flag);
        }
        let command = match action {
            ToolLifecycleAction::Update => {
                let installs = enumerate_tool_installations(tool);
                installs_anchored_command(tool, &installs)
                    .unwrap_or_else(|| static_fallback_command(tool))
            }
            ToolLifecycleAction::Install => {
                static_fallback_command_for(tool, ToolLifecycleAction::Install)
            }
        };
        if command.is_empty() {
            return Err(format!("Unsupported tool action target: {tool}"));
        }
        return Ok(format!("call {command}"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (wsl_shell, wsl_shell_flag);
        let command = match action {
            ToolLifecycleAction::Update => {
                let installs = enumerate_tool_installations(tool);
                installs_anchored_command(tool, &installs)
                    .unwrap_or_else(|| static_fallback_command(tool))
            }
            ToolLifecycleAction::Install => install_command_for(tool),
        };
        if command.is_empty() {
            return Err(format!("Unsupported tool action target: {tool}"));
        }
        Ok(command)
    }
}

#[cfg(target_os = "windows")]
fn build_wsl_tool_action_line(
    distro: &str,
    command: &str,
    force_shell: Option<&str>,
    force_shell_flag: Option<&str>,
) -> Result<String, String> {
    if !is_valid_wsl_distro_name(distro) {
        return Err(format!("Invalid WSL distro name: {distro}"));
    }

    let shell = force_shell
        .map(|s| s.rsplit('/').next().unwrap_or(s))
        .unwrap_or("sh");
    if !is_valid_shell(shell) {
        return Err(format!("Invalid WSL shell: {shell}"));
    }

    let flag = if let Some(flag) = force_shell_flag {
        if !is_valid_shell_flag(flag) {
            return Err(format!("Invalid WSL shell flag: {flag}"));
        }
        flag
    } else {
        default_flag_for_shell(shell)
    };

    Ok(format!(
        "wsl.exe -d {distro} -- {shell} {flag} {}",
        windows_cmd_double_quote_arg(command)
    ))
}

#[cfg(target_os = "windows")]
fn win_double_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

#[cfg(target_os = "windows")]
fn windows_cmd_double_quote_arg(value: &str) -> String {
    win_double_quote(value)
}
