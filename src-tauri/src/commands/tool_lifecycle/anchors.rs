use std::path::Path;
use crate::commands::tool_probe::enumerate::{default_install, ToolInstallation, infer_install_source};
use super::command_builder::{LifecycleCommandShell, ToolLifecycleAction};

pub const CLAUDE_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://claude.ai/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
pub const OPENCODE_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://opencode.ai/install -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
pub const HERMES_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
pub const HERMES_UPDATE_UNIX: &str =
    "hermes update || bash -c 'tmp=$(mktemp) && curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";

pub fn npm_package_for(tool: &str) -> Option<&'static str> {
    match tool {
        "claude" => Some("@anthropic-ai/claude-code"),
        "codex" => Some("@openai/codex"),
        "gemini" => Some("@google/gemini-cli"),
        "opencode" => Some("opencode-ai"),
        "openclaw" => Some("openclaw"),
        _ => None,
    }
}

pub fn parent_dir(p: &str) -> String {
    match p.rfind('\\').max(p.rfind('/')) {
        Some(i) if i > 0 => p[..i].to_string(),
        _ => String::new(),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn brew_formula_from_path(real: &str) -> Option<String> {
    let mut segs = real.split('/');
    while let Some(seg) = segs.next() {
        if seg.eq_ignore_ascii_case("Cellar") {
            return segs.next().filter(|s| !s.is_empty()).map(|s| s.to_string());
        }
    }
    None
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(not(target_os = "windows"))]
pub fn quote_path_if_spaced(p: &str) -> String {
    if p.contains(' ') {
        shell_single_quote(p)
    } else {
        p.to_string()
    }
}

#[cfg(target_os = "windows")]
pub fn win_quote_path_for_batch(p: &str) -> String {
    let escaped = if p.contains('%') {
        p.replace('%', "%%%%")
    } else {
        p.to_string()
    };
    let needs_quote = p
        .chars()
        .any(|c| matches!(c, ' ' | '&' | '(' | ')' | '^' | ';' | '<' | '>' | '|' | ','));
    if needs_quote {
        format!("\"{}\"", escaped.replace('"', "\\\""))
    } else {
        escaped
    }
}

#[cfg(target_os = "windows")]
pub fn sibling_bin_with_ext(
    bin_path: &str,
    exe_basename: &str,
    ext_candidates: &[&str],
) -> Option<String> {
    let dir = parent_dir(bin_path);
    if dir.is_empty() {
        return None;
    }
    let dir = std::path::PathBuf::from(dir);
    for ext in ext_candidates {
        let candidate = dir.join(format!("{exe_basename}.{ext}"));
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn sibling_bin(bin_path: &str, exe: &str) -> Option<String> {
    let dir = parent_dir(bin_path);
    if dir.is_empty() {
        None
    } else {
        Some(format!("{dir}/{exe}"))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn anchored_official_update_command(tool: &str, bin_path: &str) -> Option<String> {
    official_update_args(tool).map(|args| format!("{} {args}", quote_path_if_spaced(bin_path)))
}

#[cfg(target_os = "windows")]
pub fn anchored_official_update_command(tool: &str, bin_path: &str) -> Option<String> {
    official_update_args(tool).map(|args| format!("{} {args}", win_quote_path_for_batch(bin_path)))
}

pub fn prefers_official_update(tool: &str, shell: LifecycleCommandShell) -> bool {
    match shell {
        LifecycleCommandShell::Posix => {
            matches!(tool, "claude" | "opencode" | "openclaw")
        }
        LifecycleCommandShell::WindowsBatch => {
            matches!(
                tool,
                "claude" | "openclaw"
            )
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn codex_repair_command(bin_path: &str, real: &str) -> Option<String> {
    if brew_formula_from_path(real).is_some() {
        return None;
    }
    if !matches!(
        infer_install_source(Path::new(bin_path)),
        "nvm" | "fnm" | "mise" | "homebrew"
    ) {
        return None;
    }
    let npm = sibling_bin(bin_path, "npm")?;
    let npm = quote_path_if_spaced(&npm);
    let pkg = "@openai/codex";
    Some(format!(
        "{npm} uninstall -g {pkg} || true; {npm} i -g {pkg}@latest"
    ))
}

#[cfg(target_os = "windows")]
pub fn codex_repair_command(_bin_path: &str, _real: &str) -> Option<String> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn package_manager_anchored_command_from_paths(
    tool: &str,
    bin_path: &str,
    real_target: &str,
) -> Option<String> {
    if let Some(formula) = brew_formula_from_path(real_target) {
        let brew = sibling_bin(bin_path, "brew")?;
        return Some(format!("{} upgrade {formula}", quote_path_if_spaced(&brew)));
    }
    let pkg = npm_package_for(tool)?;
    match infer_install_source(Path::new(bin_path)) {
        "volta" => {
            let volta = sibling_bin(bin_path, "volta")?;
            return Some(format!("{} install {pkg}", quote_path_if_spaced(&volta)));
        }
        "bun" => {
            let bun = sibling_bin(bin_path, "bun")?;
            return Some(format!(
                "{} add -g {pkg}@latest",
                quote_path_if_spaced(&bun)
            ));
        }
        "nvm" | "fnm" | "mise" | "homebrew" => {}
        _ => return None,
    }
    let npm = sibling_bin(bin_path, "npm")?;
    Some(format!("{} i -g {pkg}@latest", quote_path_if_spaced(&npm)))
}

#[cfg(target_os = "windows")]
pub fn package_manager_anchored_command_from_paths(tool: &str, bin_path: &str) -> Option<String> {
    let pkg = npm_package_for(tool)?;

    match infer_install_source(Path::new(bin_path)) {
        "volta" => {
            let volta = sibling_bin_with_ext(bin_path, "volta", &["exe", "cmd"])?;
            Some(format!(
                "{} install {pkg}",
                win_quote_path_for_batch(&volta)
            ))
        }
        "pnpm" => {
            let pnpm = sibling_bin_with_ext(bin_path, "pnpm", &["cmd", "exe"])?;
            Some(format!(
                "{} add -g {pkg}@latest",
                win_quote_path_for_batch(&pnpm)
            ))
        }
        _ => {
            let npm = sibling_bin_with_ext(bin_path, "npm", &["cmd", "exe"])?;
            Some(format!(
                "{} i -g {pkg}@latest",
                win_quote_path_for_batch(&npm)
            ))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn anchored_command_from_paths(tool: &str, bin_path: &str, real_target: &str) -> Option<String> {
    let real_lower = real_target.to_ascii_lowercase();

    if tool == "hermes" {
        return anchored_official_update_command(tool, bin_path);
    }
    if tool == "claude"
        && (real_lower.contains("/.local/share/claude/")
            || real_lower.contains("/claude/versions/"))
    {
        return anchored_official_update_command(tool, bin_path);
    }
    let package_command = package_manager_anchored_command_from_paths(tool, bin_path, real_target);
    if brew_formula_from_path(real_target).is_some() {
        return package_command;
    }
    if prefers_official_update(tool, LifecycleCommandShell::Posix) {
        let update = anchored_official_update_command(tool, bin_path)?;
        return Some(match package_command {
            Some(fallback) => chain_update_commands(update, fallback, LifecycleCommandShell::Posix),
            None => update,
        });
    }
    package_command
}

#[cfg(target_os = "windows")]
pub fn anchored_command_from_paths(tool: &str, bin_path: &str, _real_target: &str) -> Option<String> {
    if tool == "hermes" {
        return anchored_official_update_command(tool, bin_path);
    }
    let package_command = package_manager_anchored_command_from_paths(tool, bin_path);
    if prefers_official_update(tool, LifecycleCommandShell::WindowsBatch) {
        let update = anchored_official_update_command(tool, bin_path)?;
        return Some(match package_command {
            Some(fallback) => {
                chain_update_commands(update, fallback, LifecycleCommandShell::WindowsBatch)
            }
            None => update,
        });
    }
    package_command
}

pub fn installs_anchored_command(tool: &str, installs: &[ToolInstallation]) -> Option<String> {
    let inst = default_install(installs)?;
    let real = inst.real.to_string_lossy();
    if tool == "codex" && !inst.runnable {
        if let Some(cmd) = codex_repair_command(&inst.path, &real) {
            return Some(cmd);
        }
    }
    anchored_command_from_paths(tool, &inst.path, &real)
}

pub fn static_fallback_command_for(tool: &str, action: ToolLifecycleAction) -> String {
    super::command_builder::tool_action_shell_command(tool, action).unwrap_or_default()
}

pub fn static_fallback_command(tool: &str) -> String {
    static_fallback_command_for(tool, ToolLifecycleAction::Update)
}

pub fn installer_with_npm_fallback(installer: &str, tool: &str) -> String {
    match npm_install_command_for(tool) {
        Some(npm) => chain_update_commands(
            installer.to_string(),
            npm.to_string(),
            LifecycleCommandShell::Posix,
        ),
        None => installer.to_string(),
    }
}

pub fn posix_install_command_for(tool: &str) -> String {
    match tool {
        "claude" => installer_with_npm_fallback(CLAUDE_INSTALL_UNIX, tool),
        "opencode" => installer_with_npm_fallback(OPENCODE_INSTALL_UNIX, tool),
        "hermes" => HERMES_INSTALL_UNIX.to_string(),
        _ => static_fallback_command_for(tool, ToolLifecycleAction::Install),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn install_command_for(tool: &str) -> String {
    posix_install_command_for(tool)
}

pub fn npm_install_command_for(tool: &str) -> Option<&'static str> {
    match tool {
        "claude" => Some("npm i -g @anthropic-ai/claude-code@latest"),
        "codex" => Some("npm i -g @openai/codex@latest"),
        "gemini" => Some("npm i -g @google/gemini-cli@latest"),
        "opencode" => Some("npm i -g opencode-ai@latest"),
        "openclaw" => Some("npm i -g openclaw@latest"),
        _ => None,
    }
}

pub fn official_update_args(tool: &str) -> Option<&'static str> {
    match tool {
        "claude" | "codex" | "hermes" => Some("update"),
        "openclaw" => Some("update --yes"),
        "opencode" => Some("upgrade"),
        _ => None,
    }
}

pub fn bare_official_update_command(tool: &str) -> Option<String> {
    official_update_args(tool).map(|args| format!("{tool} {args}"))
}

pub fn chain_update_commands(
    primary: String,
    fallback: String,
    shell: LifecycleCommandShell,
) -> String {
    if fallback.trim().is_empty() {
        return primary;
    }
    match shell {
        LifecycleCommandShell::Posix => format!("{primary} || {fallback}"),
        LifecycleCommandShell::WindowsBatch => format!("{primary} || call {fallback}"),
    }
}
