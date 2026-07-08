use std::path::Path;
use crate::commands::tool_probe::search_paths::{build_tool_search_paths, tool_executable_candidates};
use crate::commands::tool_probe::version::{decode_command_output, extract_version, last_lines, default_flag_for_shell, is_valid_shell};

#[cfg(target_os = "windows")]
use crate::commands::tool_probe::wsl::wsl_distro_for_tool;

pub const NOT_INSTALLED: &str = "not installed";

pub enum ShellProbe {
    Found(String),
    FoundButFailed(String),
    NotFound(String),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInstallation {
    pub path: String,
    pub version: Option<String>,
    pub runnable: bool,
    pub error: Option<String>,
    pub source: String,
    pub is_path_default: bool,
    #[serde(skip)]
    pub real: std::path::PathBuf,
}

pub fn infer_install_source(path: &Path) -> &'static str {
    let s = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if s.contains("/.nvm/") {
        "nvm"
    } else if s.contains("/homebrew/") || s.contains("/cellar/") {
        "homebrew"
    // `.volta` 是 macOS/Linux 默认安装(`~/.volta/bin`),`/volta/` 兜底覆盖
    // Windows 的 `%LOCALAPPDATA%\Volta\bin` / `%VOLTA_HOME%\bin`(无前导点)。
    } else if s.contains("/.volta/") || s.contains("/volta/") {
        "volta"
    } else if s.contains("fnm_multishells") {
        "fnm"
    } else if s.contains("/mise/") {
        "mise"
    } else if s.contains("/.bun/") {
        "bun"
    // pnpm 全局包目录: macOS 一般 `~/.local/share/pnpm`(已 normalize 到 `/pnpm/`)
    // 与 Windows `%LOCALAPPDATA%\pnpm` / `%PNPM_HOME%` 都命中 `/pnpm/`。
    } else if s.contains("/pnpm/") {
        "pnpm"
    } else if s.contains("/scoop/") {
        "scoop"
    } else if s.contains("/library/python")
        || s.contains("/scripts/")
        || s.contains("/site-packages/")
    {
        "pip"
    } else {
        "system"
    }
}

#[cfg(not(target_os = "windows"))]
pub fn first_abs_path_line(raw: &str) -> Option<&str> {
    raw.lines().map(str::trim).find(|l| l.starts_with('/'))
}

#[cfg(not(target_os = "windows"))]
pub fn resolve_path_default(tool: &str) -> Option<std::path::PathBuf> {
    use std::process::Command;
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| is_valid_shell(s))
        .unwrap_or_else(|| "sh".to_string());
    let flag = default_flag_for_shell(&shell);
    let out = Command::new(shell)
        .arg(flag)
        .arg(format!("command -v {tool}"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = decode_command_output(&out.stdout);
    let first = first_abs_path_line(&raw)?;
    std::fs::canonicalize(first).ok()
}

#[cfg(target_os = "windows")]
pub fn resolve_path_default(tool: &str) -> Option<std::path::PathBuf> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    use crate::commands::tool_probe::version::CREATE_NO_WINDOW;

    let out = Command::new("cmd")
        .args(["/C", &format!("where {tool}")])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = decode_command_output(&out.stdout);
    let first = raw.lines().next()?.trim();
    if first.is_empty() {
        return None;
    }
    std::fs::canonicalize(first).ok()
}

pub fn enumerate_tool_installations(tool: &str) -> Vec<ToolInstallation> {
    #[cfg(not(target_os = "windows"))]
    use std::process::Command;

    let search_paths = build_tool_search_paths(tool);
    let current_path = std::env::var_os("PATH")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let path_default = resolve_path_default(tool);

    let mut seen: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();
    let mut installs: Vec<ToolInstallation> = Vec::new();

    for dir in &search_paths {
        #[cfg(target_os = "windows")]
        let new_path = format!("{};{}", dir.display(), current_path);
        #[cfg(not(target_os = "windows"))]
        let new_path = format!("{}:{}", dir.display(), current_path);

        for tool_path in tool_executable_candidates(tool, dir) {
            if !tool_path.exists() {
                continue;
            }
            let real = std::fs::canonicalize(&tool_path).unwrap_or_else(|_| tool_path.clone());
            if !seen.insert(real.clone()) {
                continue;
            }

            #[cfg(target_os = "windows")]
            let output = crate::commands::tool_probe::version::run_windows_tool_version_command(&tool_path, &new_path);
            #[cfg(not(target_os = "windows"))]
            let output = Command::new(&tool_path)
                .arg("--version")
                .env("PATH", &new_path)
                .output();

            let (version, runnable, error) = match output {
                Ok(out) if out.status.success() => {
                    let stdout = decode_command_output(&out.stdout).trim().to_string();
                    let stderr = decode_command_output(&out.stderr).trim().to_string();
                    let raw = if stdout.is_empty() { stderr } else { stdout };
                    (Some(extract_version(&raw)), true, None)
                }
                Ok(out) => {
                    let stderr = decode_command_output(&out.stderr).trim().to_string();
                    let stdout = decode_command_output(&out.stdout).trim().to_string();
                    let detail = if stderr.is_empty() { stdout } else { stderr };
                    let detail = detail.trim();
                    let error = if detail.is_empty() {
                        None
                    } else {
                        Some(last_lines(detail, 4))
                    };
                    (None, false, error)
                }
                Err(e) => (None, false, Some(e.to_string())),
            };

            let is_path_default = path_default.as_ref() == Some(&real);
            let path_str = tool_path.display().to_string();
            let source = infer_install_source(&tool_path);

            installs.push(ToolInstallation {
                path: path_str,
                version,
                runnable,
                error,
                source: source.to_string(),
                is_path_default,
                real: real.clone(),
            });
        }
    }

    installs.sort_by_key(|i| std::cmp::Reverse(i.is_path_default));
    installs
}

pub fn default_install(installs: &[ToolInstallation]) -> Option<&ToolInstallation> {
    installs.iter().find(|i| i.is_path_default).or_else(|| {
        if installs.len() == 1 {
            installs.first()
        } else {
            None
        }
    })
}

pub fn plan_command_for(tool: &str, installs: &[ToolInstallation]) -> (String, bool, bool) {
    #[cfg(target_os = "windows")]
    {
        if wsl_distro_for_tool(tool).is_some() {
            let cmd = crate::commands::tool_lifecycle::wsl_tool_action_shell_command(tool, crate::commands::tool_lifecycle::ToolLifecycleAction::Update)
                .unwrap_or_default();
            return (cmd, false, false);
        }
    }
    match crate::commands::tool_lifecycle::installs_anchored_command(tool, installs) {
        Some(command) => (command, installs.len() >= 2, true),
        None => (crate::commands::tool_lifecycle::static_fallback_command(tool), installs.len() >= 2, false),
    }
}

pub fn is_conflicting(installs: &[ToolInstallation]) -> bool {
    if installs.len() < 2 {
        return false;
    }
    let distinct_versions: std::collections::HashSet<&Option<String>> =
        installs.iter().map(|i| &i.version).collect();
    let runnable_mixed =
        installs.iter().any(|i| i.runnable) && installs.iter().any(|i| !i.runnable);
    distinct_versions.len() > 1 || runnable_mixed
}
