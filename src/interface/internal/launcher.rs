use anyhow::{bail, Result};
use std::env;
use std::io::{self, IsTerminal as _, Write};
use std::path::Path;
use std::process::Command;

use crate::interface::TmuxLaunchMode;

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TmuxDecision {
    UseTmux,
    Direct(DirectReason),
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectReason {
    DisabledByFlag,
    DisabledByEnv,
    MissingSessionName,
    MissingTty,
    TmuxMissing,
    TmuxTooOld,
}

pub fn launch_adapter_cli(
    adapter_name: &str,
    project_root: &Path,
    tmux_mode: TmuxLaunchMode,
    tmux_session_name: Option<&str>,
    extra_env: &[(String, String)],
) -> Result<()> {
    let claude_token = if adapter_name == "claude" {
        try_get_claude_token()
    } else {
        None
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        if should_launch_tmux(tmux_mode, tmux_session_name) {
            let session_name = tmux_session_name.unwrap_or_default();
            let tmux_socket = derive_tmux_socket_name(session_name);

            eprintln!(
                "Launching {} in tmux session: {}",
                adapter_name, session_name
            );
            eprintln!(
                "  Reconnect: tmux -L {} attach -t {}",
                tmux_socket, session_name
            );
            io::stderr().flush().ok();

            let mut cmd = Command::new("tmux");
            cmd.arg("-L").arg(&tmux_socket);
            cmd.args(["new-session", "-A", "-D", "-s", session_name, "-c"]);
            cmd.arg(project_root.as_os_str());
            cmd.arg(adapter_name);
            cmd.current_dir(project_root);
            cmd.env_remove("TMUX");
            for (key, value) in extra_env {
                cmd.env(key, value);
            }
            if let Some(token) = &claude_token {
                cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
            }
            let err = cmd.exec();
            eprintln!(
                "Warning: failed to exec tmux ({}) — launching {} directly",
                err, adapter_name
            );
        }

        let mut cmd = Command::new(adapter_name);
        cmd.current_dir(project_root);
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        if let Some(token) = &claude_token {
            cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
        }
        let err = cmd.exec();
        bail!("Failed to exec {}: {}", adapter_name, err);
    }

    #[cfg(not(unix))]
    {
        use anyhow::Context as _;

        let mut cmd = Command::new(adapter_name);
        cmd.current_dir(project_root);
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        if let Some(token) = &claude_token {
            cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
        }
        let status = cmd
            .status()
            .with_context(|| format!("Failed to run {}", adapter_name))?;
        if !status.success() {
            bail!("{} exited with status: {}", adapter_name, status);
        }
        Ok(())
    }
}

fn fnv1a_32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 2_166_136_261;
    for &byte in bytes {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn derive_session_name(project_path: &Path) -> String {
    let dir_name = project_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_string());
    let slug: String = dir_name
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    let slug = if slug.len() > 50 { &slug[..50] } else { &slug };
    let hash = fnv1a_32(project_path.as_os_str().as_encoded_bytes());
    format!("patina_{}_{:08x}", slug, hash)
}

pub fn derive_interface_session_name(project_path: &Path, adapter_name: &str) -> String {
    let lane: String = adapter_name
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    format!("{}_{}", derive_session_name(project_path), lane)
}

#[cfg(unix)]
fn should_launch_tmux(tmux_mode: TmuxLaunchMode, tmux_session_name: Option<&str>) -> bool {
    matches!(
        resolve_tmux_decision(
            tmux_mode,
            tmux_session_name,
            env_truthy("PATINA_NO_TMUX"),
            stdio_has_tty(),
            which::which("tmux").is_ok(),
            check_tmux_version_ok(),
        ),
        TmuxDecision::UseTmux
    )
}

#[cfg(unix)]
fn resolve_tmux_decision(
    tmux_mode: TmuxLaunchMode,
    tmux_session_name: Option<&str>,
    env_disabled: bool,
    is_tty: bool,
    tmux_in_path: bool,
    tmux_version_ok: bool,
) -> TmuxDecision {
    if tmux_mode == TmuxLaunchMode::Off {
        return TmuxDecision::Direct(DirectReason::DisabledByFlag);
    }
    if tmux_session_name.is_none() {
        return TmuxDecision::Direct(DirectReason::MissingSessionName);
    }
    if tmux_mode == TmuxLaunchMode::Auto && env_disabled {
        return TmuxDecision::Direct(DirectReason::DisabledByEnv);
    }
    if tmux_mode == TmuxLaunchMode::Auto && !is_tty {
        return TmuxDecision::Direct(DirectReason::MissingTty);
    }
    if !tmux_in_path {
        return TmuxDecision::Direct(DirectReason::TmuxMissing);
    }
    if !tmux_version_ok {
        return TmuxDecision::Direct(DirectReason::TmuxTooOld);
    }
    TmuxDecision::UseTmux
}

#[cfg(unix)]
fn stdio_has_tty() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

#[cfg(unix)]
fn check_tmux_version_ok() -> bool {
    let output = match Command::new("tmux").arg("-V").output() {
        Ok(output) if output.status.success() => output,
        _ => return false,
    };
    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version_str.is_empty() {
        return false;
    }

    let stripped = version_str.strip_prefix("tmux ").unwrap_or(&version_str);
    let mut parts = stripped.split('.');
    let major = parts.next().and_then(|s| s.parse::<u32>().ok());
    let minor = parts.next().and_then(|s| {
        let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().ok()
    });

    match (major, minor) {
        (Some(maj), Some(min)) => (maj, min) >= (1, 9),
        _ => true,
    }
}

#[cfg(unix)]
fn derive_tmux_socket_name(session_name: &str) -> String {
    format!("{}_sock", session_name)
}

#[cfg(unix)]
fn env_truthy(key: &str) -> bool {
    matches!(
        env::var(key).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("on")
    )
}

fn try_get_claude_token() -> Option<String> {
    if env::var("ANTHROPIC_API_KEY").is_ok() {
        eprintln!(
            "patina: ANTHROPIC_API_KEY set - skipping vault token injection (API key takes priority)"
        );
        return None;
    }
    if env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok() {
        eprintln!("patina: CLAUDE_CODE_OAUTH_TOKEN already set - skipping vault token injection");
        return None;
    }
    match crate::mother::get_global_secret("claude-oauth") {
        Ok(Some(token)) => Some(token),
        Ok(None) => None,
        Err(error) => {
            eprintln!("patina: failed to read claude-oauth from vault - {}", error);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn interface_lane_names_are_stable_per_adapter() {
        let path = Path::new("/tmp/patina");
        let first = derive_interface_session_name(path, "opencode");
        let second = derive_interface_session_name(path, "opencode");
        assert_eq!(first, second);
    }

    #[test]
    fn interface_lane_names_do_not_collide_across_interfaces() {
        let path = Path::new("/tmp/patina");
        let opencode = derive_interface_session_name(path, "opencode");
        let gemini = derive_interface_session_name(path, "gemini");
        assert_ne!(opencode, gemini);
        assert!(opencode.ends_with("_opencode"));
        assert!(gemini.ends_with("_gemini"));
    }

    #[cfg(unix)]
    #[test]
    fn tmux_decision_auto_requires_tty_path_and_version() {
        assert_eq!(
            resolve_tmux_decision(TmuxLaunchMode::Auto, Some("lane"), false, true, true, true,),
            TmuxDecision::UseTmux
        );
        assert_eq!(
            resolve_tmux_decision(TmuxLaunchMode::Auto, Some("lane"), false, false, true, true,),
            TmuxDecision::Direct(DirectReason::MissingTty)
        );
        assert_eq!(
            resolve_tmux_decision(TmuxLaunchMode::Auto, Some("lane"), false, true, false, true,),
            TmuxDecision::Direct(DirectReason::TmuxMissing)
        );
    }

    #[cfg(unix)]
    #[test]
    fn tmux_decision_force_overrides_auto_only_constraints() {
        assert_eq!(
            resolve_tmux_decision(TmuxLaunchMode::Force, Some("lane"), true, false, true, true,),
            TmuxDecision::UseTmux
        );
        assert_eq!(
            resolve_tmux_decision(
                TmuxLaunchMode::Force,
                Some("lane"),
                false,
                true,
                false,
                true,
            ),
            TmuxDecision::Direct(DirectReason::TmuxMissing)
        );
    }
}
