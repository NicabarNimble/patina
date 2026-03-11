//! Launch command - Open project in AI adapter
//!
//! The launcher is how you open AI-assisted development sessions.
//! Like `code .` for VS Code, but for AI adapters.
//!
//! # Usage
//!
//! ```bash
//! patina launch              # Open in default adapter
//! patina launch claude       # Open in Claude Code
//! patina launch gemini       # Open in Gemini CLI
//! patina launch . claude     # Explicit current dir + adapter
//! patina launch ~/project    # Different project, default adapter
//! ```

pub(crate) mod internal;

use anyhow::Result;
use std::fmt;
use std::path::Path;

#[cfg_attr(not(test), allow(dead_code))]
/// Why tmux wrapping was disabled
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffReason {
    /// --no-tmux CLI flag
    CliFlag,
    /// PATINA_TMUX=0 environment variable
    EnvVar,
    /// stdout is not a terminal (CI, pipes, scripts)
    NoTty,
    /// Already inside tmux ($TMUX is set)
    InsideTmux,
    /// tmux binary not found in PATH
    NotInPath,
    /// tmux version too old (< 1.9, no -c flag)
    TmuxTooOld,
}

#[cfg_attr(not(test), allow(dead_code))]
/// Whether to wrap the adapter launch in tmux
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxDecision {
    /// Wrap in tmux (default when all checks pass)
    Auto,
    /// Don't wrap, with reason
    Off(OffReason),
}

#[cfg_attr(not(test), allow(dead_code))]
/// Resolve tmux decision from environmental inputs.
///
/// Pure function — all 6 inputs are bools, checked in priority order.
/// Each parameter corresponds to one resolution step from the spec.
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

#[cfg_attr(not(test), allow(dead_code))]
/// Check tmux version by running `tmux -V`.
///
/// Returns (version_ok, version_string).
/// - Success, parseable: "tmux X.Y" → require ≥ 1.9
/// - Success, unparseable: assume ok (don't block on novel formats)
/// - Failure (I/O error, non-zero exit): assume too old (conservative)
pub fn check_tmux_version() -> (bool, String) {
    use std::process::Command;

    let output = match Command::new("tmux").arg("-V").output() {
        Ok(o) if o.status.success() => o,
        _ => return (false, "unknown (tmux -V failed)".to_string()),
    };

    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version_str.is_empty() {
        return (false, "unknown (tmux -V failed)".to_string());
    }

    // Parse "tmux X.Y" or "tmux X.Ya" (e.g., "tmux 1.8a", "tmux 3.4")
    let stripped = version_str.strip_prefix("tmux ").unwrap_or(&version_str);

    // Extract major.minor from start of version string
    let mut parts = stripped.split('.');
    let major = parts.next().and_then(|s| s.parse::<u32>().ok());
    let minor = parts.next().and_then(|s| {
        // Strip trailing non-digit chars like "8a" → "8"
        let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().ok()
    });

    match (major, minor) {
        (Some(maj), Some(min)) => {
            let ok = (maj, min) >= (1, 9);
            (ok, format!("{}.{}", maj, min))
        }
        _ => {
            // Unparseable — assume ok (don't block on novel version strings)
            (true, "unknown".to_string())
        }
    }
}

/// FNV-1a 32-bit hash
fn fnv1a_32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 2_166_136_261; // FNV offset basis
    for &b in bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16_777_619); // FNV prime
    }
    hash
}

#[cfg_attr(not(test), allow(dead_code))]
/// Derive a deterministic tmux session name from a project path.
///
/// Format: `patina_<slug>_<hash>` where:
/// - slug: dir name lowercased, non-alphanumeric → underscore, truncated to 50 chars
/// - hash: 8-char zero-padded hex from FNV-1a 32-bit over raw path bytes
pub fn derive_session_name(project_path: &Path) -> String {
    // Slug from directory name via to_string_lossy (non-UTF-8 → U+FFFD → underscore)
    let dir_name = project_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_string());

    let slug: String = dir_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();

    // Truncate slug to 50 chars
    let slug = if slug.len() > 50 { &slug[..50] } else { &slug };

    // Hash from raw path bytes (non-UTF-8 safe)
    let path_bytes = project_path.as_os_str().as_encoded_bytes();
    let hash = fnv1a_32(path_bytes);

    format!("patina_{}_{:08x}", slug, hash)
}

impl fmt::Display for OffReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OffReason::CliFlag => write!(f, "--no-tmux flag"),
            OffReason::EnvVar => write!(f, "PATINA_TMUX=0"),
            OffReason::NoTty => write!(f, "not a terminal"),
            OffReason::InsideTmux => write!(f, "already inside tmux"),
            OffReason::NotInPath => write!(f, "tmux not found"),
            OffReason::TmuxTooOld => write!(f, "tmux too old"),
        }
    }
}

/// Launch options
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// Path to project (default: current directory)
    pub path: Option<String>,
    /// Adapter to use (default: from config)
    pub adapter: Option<String>,
    /// Start mother in background if not running
    #[allow(dead_code)]
    pub auto_start_mother: bool,
    /// Initialize project if needed (prompt user)
    pub auto_init: bool,
    /// Disable tmux wrapping (--no-tmux)
    pub no_tmux: bool,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            path: None,
            adapter: None,
            auto_start_mother: true,
            auto_init: true,
            no_tmux: false,
        }
    }
}

/// Execute the launch command
pub fn execute(options: LaunchOptions) -> Result<()> {
    internal::launch(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_options() {
        let opts = LaunchOptions::default();
        assert!(opts.path.is_none());
        assert!(opts.adapter.is_none());
        assert!(opts.auto_start_mother);
        assert!(opts.auto_init);
        assert!(!opts.no_tmux);
    }

    // --- resolve_tmux_decision: all 7 branches ---

    #[test]
    fn test_tmux_decision_auto() {
        // All checks pass → Auto
        let d = resolve_tmux_decision(false, false, true, false, true, true);
        assert_eq!(d, TmuxDecision::Auto);
    }

    #[test]
    fn test_tmux_decision_cli_flag() {
        let d = resolve_tmux_decision(true, false, true, false, true, true);
        assert_eq!(d, TmuxDecision::Off(OffReason::CliFlag));
    }

    #[test]
    fn test_tmux_decision_env_var() {
        let d = resolve_tmux_decision(false, true, true, false, true, true);
        assert_eq!(d, TmuxDecision::Off(OffReason::EnvVar));
    }

    #[test]
    fn test_tmux_decision_no_tty() {
        let d = resolve_tmux_decision(false, false, false, false, true, true);
        assert_eq!(d, TmuxDecision::Off(OffReason::NoTty));
    }

    #[test]
    fn test_tmux_decision_inside_tmux() {
        let d = resolve_tmux_decision(false, false, true, true, true, true);
        assert_eq!(d, TmuxDecision::Off(OffReason::InsideTmux));
    }

    #[test]
    fn test_tmux_decision_not_in_path() {
        let d = resolve_tmux_decision(false, false, true, false, false, true);
        assert_eq!(d, TmuxDecision::Off(OffReason::NotInPath));
    }

    #[test]
    fn test_tmux_decision_too_old() {
        let d = resolve_tmux_decision(false, false, true, false, true, false);
        assert_eq!(d, TmuxDecision::Off(OffReason::TmuxTooOld));
    }

    #[test]
    fn test_tmux_decision_priority_cli_over_env() {
        // Both CLI and env disabled — CLI wins (higher priority)
        let d = resolve_tmux_decision(true, true, true, false, true, true);
        assert_eq!(d, TmuxDecision::Off(OffReason::CliFlag));
    }

    #[test]
    fn test_tmux_decision_priority_env_over_tty() {
        let d = resolve_tmux_decision(false, true, false, false, true, true);
        assert_eq!(d, TmuxDecision::Off(OffReason::EnvVar));
    }

    // --- derive_session_name ---

    #[test]
    fn test_session_name_basic() {
        let name = derive_session_name(Path::new("/home/user/Projects/patina"));
        // Should start with patina_ prefix
        assert!(name.starts_with("patina_"));
        // Should contain the slug
        assert!(name.contains("patina_"));
        // Should end with 8-char hex hash
        let parts: Vec<&str> = name.rsplitn(2, '_').collect();
        assert_eq!(parts[0].len(), 8);
        assert!(parts[0].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_session_name_deterministic() {
        let path = Path::new("/home/user/Projects/patina");
        let name1 = derive_session_name(path);
        let name2 = derive_session_name(path);
        assert_eq!(name1, name2);
    }

    #[test]
    fn test_session_name_different_paths_same_dirname() {
        let name1 = derive_session_name(Path::new("/home/alice/patina"));
        let name2 = derive_session_name(Path::new("/home/bob/patina"));
        // Same slug but different hash (different full paths)
        assert_ne!(name1, name2);
        // Both start with patina_patina_
        assert!(name1.starts_with("patina_patina_"));
        assert!(name2.starts_with("patina_patina_"));
    }

    #[test]
    fn test_session_name_special_chars() {
        let name = derive_session_name(Path::new("/home/user/my-cool.project"));
        // Hyphens and dots become underscores
        assert!(name.starts_with("patina_my_cool_project_"));
    }

    #[test]
    fn test_session_name_slug_truncation() {
        // Create a path with a very long directory name (60+ chars)
        let long_name = "a".repeat(60);
        let path = PathBuf::from(format!("/home/user/{}", long_name));
        let name = derive_session_name(&path);

        // Total: "patina_" (7) + slug (≤50) + "_" (1) + hash (8) = ≤66
        assert!(name.len() <= 66, "name too long: {} ({})", name, name.len());

        // Slug portion should be exactly 50 chars
        let without_prefix = name.strip_prefix("patina_").unwrap();
        let slug_end = without_prefix.len() - 9; // "_" + 8 hex chars
        let slug = &without_prefix[..slug_end];
        assert_eq!(slug.len(), 50);
    }

    #[test]
    fn test_session_name_max_length() {
        // Even with max slug, total ≤ 66
        let long_name = "x".repeat(100);
        let path = PathBuf::from(format!("/tmp/{}", long_name));
        let name = derive_session_name(&path);
        assert!(name.len() <= 66);
    }

    // --- fnv1a_32 ---

    #[test]
    fn test_fnv1a_empty() {
        // FNV-1a of empty input is the offset basis
        assert_eq!(fnv1a_32(b""), 2_166_136_261);
    }

    #[test]
    fn test_fnv1a_deterministic() {
        let h1 = fnv1a_32(b"/home/user/patina");
        let h2 = fnv1a_32(b"/home/user/patina");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_fnv1a_different_inputs() {
        let h1 = fnv1a_32(b"/home/alice/patina");
        let h2 = fnv1a_32(b"/home/bob/patina");
        assert_ne!(h1, h2);
    }
}
