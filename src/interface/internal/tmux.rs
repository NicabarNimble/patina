use anyhow::{bail, Result};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffReason {
    CliFlag,
    EnvVar,
    NoTty,
    InsideTmux,
    NotInPath,
    TmuxTooOld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxDecision {
    Auto,
    Off(OffReason),
}

pub fn resolve_tmux_decision(
    cli_no_tmux: bool,
    env_disabled: bool,
    is_tty: bool,
    inside_tmux: bool,
    tmux_in_path: bool,
    tmux_version_ok: bool,
) -> TmuxDecision {
    if cli_no_tmux {
        return TmuxDecision::Off(OffReason::CliFlag);
    }
    if env_disabled {
        return TmuxDecision::Off(OffReason::EnvVar);
    }
    if !is_tty {
        return TmuxDecision::Off(OffReason::NoTty);
    }
    if inside_tmux {
        return TmuxDecision::Off(OffReason::InsideTmux);
    }
    if !tmux_in_path {
        return TmuxDecision::Off(OffReason::NotInPath);
    }
    if !tmux_version_ok {
        return TmuxDecision::Off(OffReason::TmuxTooOld);
    }
    TmuxDecision::Auto
}

pub fn check_tmux_version() -> (bool, String) {
    let output = match Command::new("tmux").arg("-V").output() {
        Ok(output) if output.status.success() => output,
        _ => return (false, "unknown (tmux -V failed)".to_string()),
    };
    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version_str.is_empty() {
        return (false, "unknown (tmux -V failed)".to_string());
    }

    let stripped = version_str.strip_prefix("tmux ").unwrap_or(&version_str);
    let mut parts = stripped.split('.');
    let major = parts.next().and_then(|s| s.parse::<u32>().ok());
    let minor = parts.next().and_then(|s| {
        let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().ok()
    });

    match (major, minor) {
        (Some(maj), Some(min)) => ((maj, min) >= (1, 9), format!("{}.{}", maj, min)),
        _ => (true, "unknown".to_string()),
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

pub fn derive_session_name(project_path: &Path) -> String {
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

pub fn launch_adapter_cli(
    adapter_name: &str,
    project_root: &Path,
    decision: &TmuxDecision,
    session_name: &str,
    extra_env: &[(String, String)],
) -> Result<()> {
    let tmux_log_dir = project_root.join(".patina/local/logs/tmux");
    let tmux_events_log = tmux_log_dir.join("events.log");
    let tmux_verbose = env_truthy("PATINA_TMUX_VERBOSE");

    let _ = fs::create_dir_all(&tmux_log_dir);

    let claude_token = if adapter_name == "claude" {
        try_get_claude_token()
    } else {
        None
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        match decision {
            TmuxDecision::Auto => {
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

                append_tmux_event(
                    &tmux_events_log,
                    "launch_tmux",
                    adapter_name,
                    session_name,
                    Some(&tmux_socket),
                    Some(tmux_verbose),
                    None,
                );

                let mut cmd = Command::new("tmux");
                cmd.arg("-L").arg(&tmux_socket);
                if tmux_verbose {
                    cmd.arg("-vv");
                }
                cmd.args(["new-session", "-A", "-D", "-s", session_name, "-c"]);
                cmd.arg(project_root.as_os_str());
                cmd.arg(adapter_name);
                cmd.current_dir(&tmux_log_dir);
                for (key, value) in extra_env {
                    cmd.env(key, value);
                }
                if let Some(token) = &claude_token {
                    cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
                }
                let err = cmd.exec();
                append_tmux_event(
                    &tmux_events_log,
                    "tmux_exec_failed",
                    adapter_name,
                    session_name,
                    Some(&tmux_socket),
                    Some(tmux_verbose),
                    Some(&err.to_string()),
                );
                eprintln!(
                    "Warning: failed to exec tmux ({}) — launching {} directly",
                    err, adapter_name
                );
            }
            TmuxDecision::Off(_) => {
                append_tmux_event(
                    &tmux_events_log,
                    "launch_direct_tmux_off",
                    adapter_name,
                    session_name,
                    None,
                    Some(tmux_verbose),
                    None,
                );
                println!("\nLaunching {}...\n", adapter_name);
            }
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

fn derive_tmux_socket_name(session_name: &str) -> String {
    format!("{}_sock", session_name)
}

fn env_truthy(key: &str) -> bool {
    matches!(
        env::var(key).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("on")
    )
}

fn append_tmux_event(
    path: &Path,
    event: &str,
    adapter_name: &str,
    session_name: &str,
    socket_name: Option<&str>,
    verbose: Option<bool>,
    error: Option<&str>,
) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    let mut line = format!(
        "ts={} event={} adapter={} session={}",
        ts, event, adapter_name, session_name
    );
    if let Some(socket_name) = socket_name {
        line.push_str(&format!(" socket={}", socket_name));
    }
    if let Some(verbose) = verbose {
        line.push_str(&format!(" verbose={}", verbose));
    }
    if let Some(error) = error {
        line.push_str(&format!(" error={}", sanitize_for_log(error)));
    }
    line.push('\n');

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn sanitize_for_log(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_whitespace() { '_' } else { ch })
        .collect()
}

fn try_get_claude_token() -> Option<String> {
    if env::var("ANTHROPIC_API_KEY").is_ok() {
        eprintln!(
            "patina: ANTHROPIC_API_KEY set — skipping vault token injection (API key takes priority)"
        );
        return None;
    }
    if env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok() {
        eprintln!("patina: CLAUDE_CODE_OAUTH_TOKEN already set — skipping vault token injection");
        return None;
    }
    match crate::secrets::get_global_secret("claude-oauth") {
        Ok(Some(token)) => Some(token),
        Ok(None) => None,
        Err(error) => {
            eprintln!("patina: failed to read claude-oauth from vault — {}", error);
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

    #[test]
    fn legacy_session_name_remains_project_scoped() {
        let path = Path::new("/tmp/patina");
        let legacy = derive_session_name(path);
        let ai = derive_interface_session_name(path, "opencode");

        assert!(!legacy.ends_with("_opencode"));
        assert_ne!(legacy, ai);
    }
}
