use crate::commands::tool_probe::version::decode_command_output;

#[cfg(not(target_os = "windows"))]
pub fn run_tool_lifecycle_silently(command_line: &str, _label: &str) -> Result<(), String> {
    use std::process::Command;
    // command_line 是 bash 风格脚本（含 `set -e` 与多行命令）；强制用 bash 执行，
    // 避免用户默认 shell 为 fish/zsh 时 `set -e` 等语义不一致。
    let output = Command::new("bash")
        .arg("-c")
        .arg(command_line)
        .output()
        .map_err(|e| format!("spawn bash error: {e}"))?;

    finish_lifecycle_output(&output)
}

#[cfg(target_os = "windows")]
pub fn run_tool_lifecycle_silently(command_line: &str, label: &str) -> Result<(), String> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    use crate::commands::tool_probe::version::CREATE_NO_WINDOW;

    let temp_dir = std::env::temp_dir();
    let bat_file = temp_dir.join(format!("cc_switch_lifecycle_{}_{}.bat", label, std::process::id()));

    // Windows lifecycle batch runs in CREATE_NO_WINDOW process.
    // Ensure exit codes propagate to indicate success/failure correctly.
    let bat_content = format!(
        "@echo off\r\n{command_line}\r\nif %errorlevel% neq 0 (exit /b %errorlevel%)\r\n"
    );
    std::fs::write(&bat_file, &bat_content).map_err(|e| format!("write temp bat error: {e}"))?;

    let output = Command::new("cmd")
        .args(["/C", &bat_file.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = std::fs::remove_file(&bat_file);

    let output = output.map_err(|e| format!("spawn cmd.exe error: {e}"))?;
    finish_lifecycle_output(&output)
}

fn finish_lifecycle_output(output: &std::process::Output) -> Result<(), String> {
    use crate::commands::tool_probe::version::last_lines;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = decode_command_output(&output.stderr).trim().to_string();
        let stdout = decode_command_output(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        let detail = detail.trim();
        if detail.is_empty() {
            Err("Command exited with non-zero code, but output was empty".to_string())
        } else {
            Err(last_lines(detail, 8))
        }
    }
}
