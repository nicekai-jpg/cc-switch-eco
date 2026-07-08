#![cfg(target_os = "linux")]

use std::path::Path;
use crate::commands::tool_probe::version::decode_command_output;
use super::shell::{
    get_user_shell, build_exec_line, build_final_shell_cd_command,
    build_provider_command_line,
};

pub fn launch_linux_terminal(config_file: &std::path::Path, cwd: Option<&Path>) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let preferred = crate::settings::get_preferred_terminal();

    let shell = get_user_shell();
    let exec_line = build_exec_line(&shell, cwd);
    let final_cd_command = build_final_shell_cd_command(&shell, cwd);

    // Default terminal list with their arguments
    let default_terminals = [
        ("gnome-terminal", vec!["--"]),
        ("konsole", vec!["-e"]),
        ("xfce4-terminal", vec!["-e"]),
        ("mate-terminal", vec!["--"]),
        ("lxterminal", vec!["-e"]),
        ("alacritty", vec!["-e"]),
        ("kitty", vec!["-e"]),
        ("ghostty", vec!["-e"]),
    ];

    // Create temp script file
    let temp_dir = std::env::temp_dir();
    let script_file = temp_dir.join(format!("cc_switch_launcher_{}.sh", std::process::id()));
    let config_path = config_file.to_string_lossy();
    let provider_command = build_provider_command_line(&shell, &config_path, cwd);

    let script_content = format!(
        r#"#!/usr/bin/env sh
trap 'rm -f "{config_path}" "{script_file}"' EXIT
echo "Using provider-specific claude config:"
echo "{config_path}"
{provider_command}
{final_cd_command}
{exec_line}
"#,
        config_path = config_path,
        script_file = script_file.display(),
        provider_command = provider_command,
        final_cd_command = final_cd_command,
        exec_line = exec_line,
    );

    std::fs::write(&script_file, &script_content).map_err(|e| format!("写入启动脚本失败: {e}"))?;

    std::fs::set_permissions(&script_file, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("设置脚本权限失败: {e}"))?;

    // Build terminal list: preferred terminal first (if specified), then defaults
    let terminals_to_try: Vec<(&str, Vec<&str>)> = if let Some(ref pref) = preferred {
        let pref_args = default_terminals
            .iter()
            .find(|(name, _)| *name == pref.as_str())
            .map(|(_, args)| args.to_vec())
            .unwrap_or_else(|| vec!["-e"]); // Default args for unknown terminals

        let mut list = vec![(pref.as_str(), pref_args)];
        for (name, args) in &default_terminals {
            if *name != pref.as_str() {
                list.push((*name, args.to_vec()));
            }
        }
        list
    } else {
        default_terminals
            .iter()
            .map(|(name, args)| (*name, args.to_vec()))
            .collect()
    };

    let mut last_error = String::from("未找到可用的终端");

    for (terminal, args) in terminals_to_try {
        let terminal_exists = std::path::Path::new(&format!("/usr/bin/{}", terminal)).exists()
            || std::path::Path::new(&format!("/bin/{}", terminal)).exists()
            || std::path::Path::new(&format!("/usr/local/bin/{}", terminal)).exists()
            || which_command(terminal);

        if terminal_exists {
            let result = Command::new(terminal)
                .args(&args)
                .arg("sh")
                .arg(script_file.to_string_lossy().as_ref())
                .spawn();

            match result {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = format!("执行 {} 失败: {}", terminal, e);
                }
            }
        }
    }

    let _ = std::fs::remove_file(&script_file);
    let _ = std::fs::remove_file(config_file);
    Err(last_error)
}

pub fn which_command(cmd: &str) -> bool {
    use std::process::Command;
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
