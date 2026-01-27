//! Detect exposed secrets in files.
//!
//! Independent of Patina's vault system. Can scan any files.

mod patterns;

use anyhow::{bail, Result};
use regex::{Regex, RegexSet};
use std::path::{Path, PathBuf};

/// A detected secret exposure
#[derive(Debug, Clone)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub pattern: &'static str,
    pub matched: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    High,
    Medium,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
        }
    }
}

/// Core: scan arbitrary files
pub fn scan_files(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Vec<Finding>> {
    let regex_set = RegexSet::new(patterns::PATTERNS.iter().map(|p| p.regex))?;
    let regexes: Vec<Regex> = patterns::PATTERNS
        .iter()
        .map(|p| Regex::new(p.regex))
        .collect::<Result<_, _>>()?;

    let mut findings = Vec::new();

    for path in paths {
        let path = path.as_ref();

        if should_skip(path) {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => continue,
            Err(e) => bail!("failed to read {}: {}", path.display(), e),
        };

        scan_content(path, &content, &regex_set, &regexes, &mut findings);
    }

    Ok(findings)
}

/// Convenience: scan git-tracked files
pub fn scan_tracked(repo_root: &Path) -> Result<Vec<Finding>> {
    let files = git_ls_files(repo_root)?;
    scan_files(files)
}

/// Convenience: scan staged files only
pub fn scan_staged(repo_root: &Path) -> Result<Vec<Finding>> {
    let files = git_staged_files(repo_root)?;
    scan_files(files)
}

/// Scan a string directly (for testing)
pub fn scan_string(content: &str) -> Result<Vec<Finding>> {
    let regex_set = RegexSet::new(patterns::PATTERNS.iter().map(|p| p.regex))?;
    let regexes: Vec<Regex> = patterns::PATTERNS
        .iter()
        .map(|p| Regex::new(p.regex))
        .collect::<Result<_, _>>()?;

    let mut findings = Vec::new();
    scan_content(
        Path::new("<string>"),
        content,
        &regex_set,
        &regexes,
        &mut findings,
    );
    Ok(findings)
}

fn scan_content(
    path: &Path,
    content: &str,
    regex_set: &RegexSet,
    regexes: &[Regex],
    findings: &mut Vec<Finding>,
) {
    for (line_num, line) in content.lines().enumerate() {
        let matches: Vec<usize> = regex_set.matches(line).iter().collect();

        for idx in matches {
            let pattern = &patterns::PATTERNS[idx];
            if let Some(m) = regexes[idx].find(line) {
                findings.push(Finding {
                    path: path.to_owned(),
                    line: line_num + 1,
                    column: m.start() + 1,
                    pattern: pattern.name,
                    matched: redact(m.as_str()),
                    severity: pattern.severity,
                });
            }
        }
    }
}

fn should_skip(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let path_str = path.to_string_lossy();

    // Lock files have hashes that look like secrets
    name.ends_with(".lock")
        || name == "package-lock.json"
        || name == "yarn.lock"
        || name == "pnpm-lock.yaml"
        // Documentation has example patterns
        || name.ends_with(".md")
        // Test directories have test fixtures
        || path_str.contains("/tests/")
        || path_str.contains("/test/")
        // Scanner's own test patterns (circular)
        || path_str.contains("/scanner/")
}

fn redact(s: &str) -> String {
    if s.len() <= 8 {
        "*".repeat(s.len())
    } else {
        format!("{}...{}", &s[..4], &s[s.len() - 4..])
    }
}

fn git_ls_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        bail!("not a git repository: {}", repo_root.display());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| repo_root.join(s))
        .collect())
}

fn git_staged_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "-z"])
        .current_dir(repo_root)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| repo_root.join(s))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_github_token() {
        // Just the token, no "token =" prefix to avoid generic_secret match
        let findings = scan_string(r#"ghp_ABC123def456GHI789jkl012MNO345pqr678"#).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].pattern, "github_token");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn detects_private_key_pem() {
        let findings = scan_string("-----BEGIN RSA PRIVATE KEY-----").unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].pattern, "private_key_pem");
    }

    #[test]
    fn detects_url_credentials() {
        let findings = scan_string("postgres://user:secretpassword@localhost/db").unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].pattern, "url_credentials");
        assert_eq!(findings[0].severity, Severity::Medium);
    }

    #[test]
    fn ignores_lock_files() {
        assert!(should_skip(Path::new("Cargo.lock")));
        assert!(should_skip(Path::new("package-lock.json")));
        assert!(!should_skip(Path::new("config.toml")));
    }

    #[test]
    fn redacts_secrets() {
        let redacted = redact("ghp_ABC123def456GHI789jkl012MNO345pqr678");
        assert!(redacted.contains("..."));
        assert!(redacted.starts_with("ghp_"));
    }

    #[test]
    fn no_false_positive_on_short_value() {
        // Short values (< 8 chars) don't trigger generic_secret
        let findings = scan_string(r#"token = "abc""#).unwrap();
        assert!(findings.is_empty());
    }
}
