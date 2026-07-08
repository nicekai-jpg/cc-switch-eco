use std::collections::HashMap;
use regex::Regex;
use once_cell::sync::Lazy;
use crate::commands::tool_probe::enumerate::ShellProbe;
use crate::commands::tool_probe::enumerate::NOT_INSTALLED;

#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn decode_command_output(bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        decode_windows_command_output(bytes)
    }

    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(target_os = "windows")]
pub fn decode_windows_command_output(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }

    use windows_sys::Win32::Globalization::{GetACP, GetOEMCP, MultiByteToWideChar};

    fn decode_codepage(bytes: &[u8], codepage: u32) -> Option<String> {
        if codepage == 0 {
            return None;
        }

        let input_len = i32::try_from(bytes.len()).ok()?;
        unsafe {
            let wide_len = MultiByteToWideChar(
                codepage,
                0,
                bytes.as_ptr(),
                input_len,
                std::ptr::null_mut(),
                0,
            );
            if wide_len <= 0 {
                return None;
            }

            let mut wide = vec![0u16; wide_len as usize];
            let written = MultiByteToWideChar(
                codepage,
                0,
                bytes.as_ptr(),
                input_len,
                wide.as_mut_ptr(),
                wide_len,
            );
            if written <= 0 {
                return None;
            }

            Some(String::from_utf16_lossy(&wide[..written as usize]))
        }
    }

    let oem_cp = unsafe { GetOEMCP() };
    if let Some(decoded) = decode_codepage(bytes, oem_cp) {
        return decoded;
    }

    let ansi_cp = unsafe { GetACP() };
    if ansi_cp != oem_cp {
        if let Some(decoded) = decode_codepage(bytes, ansi_cp) {
            return decoded;
        }
    }

    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(not(target_os = "windows"))]
pub fn decode_windows_command_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

pub fn is_valid_shell(shell: &str) -> bool {
    if shell.is_empty() || shell.len() > 100 {
        return false;
    }
    shell.chars().all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '-' || c == '.')
}

pub fn is_valid_shell_flag(flag: &str) -> bool {
    matches!(flag, "-c" | "-lc" | "-lic")
}

pub fn default_flag_for_shell(shell: &str) -> &'static str {
    match shell.rsplit('/').next().unwrap_or(shell) {
        "dash" | "sh" => "-c",
        "fish" => "-lc",
        _ => "-lic",
    }
}

pub fn extract_version(raw: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)(?:version\s+)?v?(\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?)(?:\s|$)").unwrap()
    });

    let cleaned = raw
        .lines()
        .map(|line| line.trim())
        .find(|line| !line.is_empty() && !line.starts_with("npm warn") && !line.starts_with("warning"))
        .unwrap_or("");

    if let Some(caps) = RE.captures(cleaned) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
        }
    }

    let words: Vec<&str> = cleaned.split_whitespace().collect();
    for word in words {
        let cleaned_word = word.trim_start_matches('v').trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
        if cleaned_word.split('.').count() >= 2 && cleaned_word.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return cleaned_word.to_string();
        }
    }

    cleaned.to_string()
}

pub fn npm_prerelease_tags(tool: &str) -> &'static [&'static str] {
    match tool {
        "claude" => &["next"],
        _ => &[],
    }
}

pub fn parse_semver(v: &str) -> Option<([u64; 3], Vec<String>)> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^v?(\d+)\.(\d+)\.(\d+)(?:-([a-zA-Z0-9.]+))?$").unwrap()
    });
    let caps = RE.captures(v)?;
    let major = caps.get(1)?.as_str().parse().ok()?;
    let minor = caps.get(2)?.as_str().parse().ok()?;
    let patch = caps.get(3)?.as_str().parse().ok()?;
    let prerelease = caps
        .get(4)
        .map(|m| m.as_str().split('.').map(|s| s.to_string()).collect())
        .unwrap_or_default();
    Some(([major, minor, patch], prerelease))
}

pub fn compare_semver(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let (nums_a, pre_a) = parse_semver(a)?;
    let (nums_b, pre_b) = parse_semver(b)?;
    if nums_a != nums_b {
        return Some(nums_a.cmp(&nums_b));
    }
    match (pre_a.is_empty(), pre_b.is_empty()) {
        (true, true) => Some(std::cmp::Ordering::Equal),
        (true, false) => Some(std::cmp::Ordering::Greater),
        (false, true) => Some(std::cmp::Ordering::Less),
        (false, false) => Some(pre_a.cmp(&pre_b)),
    }
}

pub fn pick_latest_version(versions: &[String]) -> Option<String> {
    let mut valid: Vec<&str> = versions.iter().map(|s| s.as_str()).collect();
    valid.sort_by(|a, b| compare_semver(a, b).unwrap_or(std::cmp::Ordering::Equal));
    valid.last().map(|s| s.to_string())
}

pub async fn fetch_npm_dist_tags(
    client: &reqwest::Client,
    package: &str,
) -> Option<HashMap<String, String>> {
    let url = format!("https://registry.npmjs.org/-/package/{package}/dist-tags");
    client
        .get(&url)
        .header("User-Agent", "cc-switch-eco")
        .send()
        .await
        .ok()?
        .json::<HashMap<String, String>>()
        .await
        .ok()
}

pub async fn fetch_npm_latest_for_tool(
    client: &reqwest::Client,
    tool: &str,
    package: &str,
) -> Option<String> {
    let tags = fetch_npm_dist_tags(client, package).await?;
    let mut candidates = Vec::new();
    if let Some(latest) = tags.get("latest") {
        candidates.push(latest.clone());
    }
    for tag in npm_prerelease_tags(tool) {
        if let Some(v) = tags.get(*tag) {
            candidates.push(v.clone());
        }
    }
    pick_latest_version(&candidates)
}

pub async fn fetch_github_latest_version(client: &reqwest::Client, repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = client
        .get(&url)
        .header("User-Agent", "cc-switch-eco")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
    }
    let r = resp.json::<Release>().await.ok()?;
    Some(extract_version(&r.tag_name))
}

pub async fn fetch_pypi_latest_version(client: &reqwest::Client, package: &str) -> Option<String> {
    let url = format!("https://pypi.org/pypi/{package}/json");
    let resp = client
        .get(&url)
        .header("User-Agent", "cc-switch-eco")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    #[derive(serde::Deserialize)]
    struct Info {
        version: String,
    }
    #[derive(serde::Deserialize)]
    struct PyPiResponse {
        info: Info,
    }
    let r = resp.json::<PyPiResponse>().await.ok()?;
    Some(extract_version(&r.info.version))
}

pub fn try_get_version(tool: &str) -> ShellProbe {
    use std::process::Command;

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd")
            .args(["/C", &format!("{tool} --version")])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        match output {
            Ok(out) => {
                let stdout = decode_command_output(&out.stdout).trim().to_string();
                let stderr = decode_command_output(&out.stderr).trim().to_string();
                if out.status.success() {
                    let raw = if stdout.is_empty() { &stderr } else { &stdout };
                    if raw.is_empty() {
                        ShellProbe::NotFound(NOT_INSTALLED.to_string())
                    } else {
                        ShellProbe::Found(extract_version(raw))
                    }
                } else {
                    let err = if stderr.is_empty() { stdout } else { stderr };
                    let not_found = err.is_empty()
                        || out.status.code() == Some(1)
                        || err.contains("is not recognized as an internal or external command");
                    if not_found {
                        ShellProbe::NotFound(NOT_INSTALLED.to_string())
                    } else {
                        ShellProbe::FoundButFailed(last_lines(err.trim(), 4))
                    }
                }
            }
            Err(e) => ShellProbe::NotFound(format!("exec failed: {e}")),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| is_valid_shell(s))
            .unwrap_or_else(|| "sh".to_string());
        let flag = default_flag_for_shell(&shell);
        let output = Command::new(shell)
            .arg(flag)
            .arg(format!("{tool} --version"))
            .output();
        match output {
            Ok(out) => {
                let stdout = decode_command_output(&out.stdout).trim().to_string();
                let stderr = decode_command_output(&out.stderr).trim().to_string();
                if out.status.success() {
                    let raw = if stdout.is_empty() { &stderr } else { &stdout };
                    if raw.is_empty() {
                        ShellProbe::NotFound(NOT_INSTALLED.to_string())
                    } else {
                        ShellProbe::Found(extract_version(raw))
                    }
                } else {
                    let err = if stderr.is_empty() { stdout } else { stderr };
                    let not_found = err.is_empty()
                        || out.status.code() == Some(127)
                        || err.contains("command not found")
                        || err.contains("not found");
                    if not_found {
                        ShellProbe::NotFound(NOT_INSTALLED.to_string())
                    } else {
                        ShellProbe::FoundButFailed(last_lines(err.trim(), 4))
                    }
                }
            }
            Err(e) => ShellProbe::NotFound(format!("exec failed: {e}")),
        }
    }
}

#[cfg(target_os = "windows")]
pub fn run_windows_tool_version_command(tool_path: &Path, path_env: &str) -> std::io::Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new(tool_path)
        .arg("--version")
        .env("PATH", path_env)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

pub async fn get_single_tool_version_impl(
    tool: &str,
    force_wsl_shell: Option<&str>,
    force_wsl_shell_flag: Option<&str>,
) -> crate::commands::tool_probe::ToolVersion {
    let client = reqwest::Client::new();
    let (env_type, wsl_distro) = crate::commands::tool_probe::tool_env_type_and_wsl_distro(tool);

    let local_probe = if env_type == "wsl" {
        let distro = wsl_distro.as_deref().unwrap_or("");
        crate::commands::tool_probe::wsl::try_get_version_wsl(tool, distro, force_wsl_shell, force_wsl_shell_flag)
    } else {
        try_get_version(tool)
    };

    let (version, error, installed_but_broken) = match local_probe {
        ShellProbe::Found(v) => (Some(v), None, false),
        ShellProbe::FoundButFailed(err) => (None, Some(err), true),
        ShellProbe::NotFound(err) => (None, Some(err), false),
    };

    let latest_version = match tool {
        "claude" => fetch_npm_latest_for_tool(&client, tool, "@anthropic-ai/claude-code").await,
        "codex" => fetch_npm_latest_for_tool(&client, tool, "@openai/codex").await,
        "gemini" => fetch_npm_latest_for_tool(&client, tool, "@google/gemini-cli").await,
        "opencode" => fetch_npm_latest_for_tool(&client, tool, "opencode-ai").await,
        "openclaw" => fetch_npm_latest_for_tool(&client, tool, "openclaw").await,
        "hermes" => fetch_pypi_latest_version(&client, "hermes-ai").await,
        _ => None,
    };

    crate::commands::tool_probe::ToolVersion {
        name: tool.to_string(),
        version,
        latest_version,
        error,
        installed_but_broken,
        env_type,
        wsl_distro,
    }
}
