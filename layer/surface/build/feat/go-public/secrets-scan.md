# Scanner Module

> Detect exposed secrets in files. Independent module, CLI unified under `patina secrets`.

**Parent:** [go-public/SPEC.md](./SPEC.md) (0.8.6 milestone)
**Status:** design
**Sessions:** 20260126-111115, 20260126-134036

---

## Design Principle

Scanner is a **pure function**: files in, findings out.

It doesn't know or care about:
- Whether you have a vault
- How you store secrets
- What Patina is

This independence is intentional. The scanner could be used on any project, with or without Patina's vault system.

```
src/secrets/     "Store and use secrets safely"
      │
      │  no dependency
      │
src/scanner/     "Detect exposed secrets in files"
      │
      └── CLI unifies both under `patina secrets`
```

**The test:** Can I delete one without touching the other? Yes. They're not coupled. The code reflects this.

---

## Architecture

```
src/
├── secrets/           # Vault (unchanged)
│   ├── mod.rs
│   ├── vault.rs
│   ├── identity.rs
│   └── ...
│
├── scanner/           # NEW: Independent module
│   ├── mod.rs         # Public API
│   ├── patterns.rs    # Detection patterns
│   └── error.rs       # Typed errors
│
└── commands/
    └── secrets.rs     # CLI composition layer
```

The CLI imports both:
```rust
use patina::secrets;   // Vault operations
use patina::scanner;   // Scan operations

match command {
    Add { .. }   => secrets::add_secret(...),
    Run { .. }   => secrets::run_with_secrets(...),
    Check        => scanner::scan_staged(...),
    Audit        => scanner::scan_tracked(...),
}
```

User sees `patina secrets`. Code is honest about independence.

---

## Public API (mod.rs)

```rust
//! Detect exposed secrets in files.
//!
//! Independent of Patina's vault system. Can scan any files.

mod error;
mod patterns;

pub use error::ScanError;

use std::path::{Path, PathBuf};

/// A detected secret exposure
#[derive(Debug, Clone)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub pattern: &'static str,
    pub matched: String,       // Redacted
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    High,
    Medium,
}

/// Core: scan arbitrary files
///
/// Pure function - works with any file list.
pub fn scan_files(
    paths: impl IntoIterator<Item = impl AsRef<Path>>
) -> Result<Vec<Finding>, ScanError>;

/// Convenience: scan git-tracked files
pub fn scan_tracked(repo_root: &Path) -> Result<Vec<Finding>, ScanError>;

/// Convenience: scan staged files only
pub fn scan_staged(repo_root: &Path) -> Result<Vec<Finding>, ScanError>;
```

**Key:** `scan_files` is the core. Git-aware functions are conveniences built on top.

Composable usage:
```rust
// Scan specific files (no git involved)
scanner::scan_files(&["config.toml", "src/main.rs"])?;

// Scan git-tracked files
scanner::scan_tracked(&repo_root)?;

// Scan only staged files (pre-commit)
scanner::scan_staged(&repo_root)?;
```

---

## Error Handling (error.rs)

Explicit. Matchable. No `anyhow` in public API.

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("not a git repository: {0}")]
    NotAGitRepo(PathBuf),

    #[error("git command failed: {0}")]
    GitFailed(String),

    #[error("failed to read {path}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("regex compilation failed: {0}")]
    RegexError(#[from] regex::Error),
}
```

---

## Patterns (patterns.rs)

Const data. No trait abstraction until needed.

```rust
use crate::scanner::Severity;

pub struct Pattern {
    pub name: &'static str,
    pub regex: &'static str,
    pub severity: Severity,
}

pub static PATTERNS: &[Pattern] = &[
    // === High Severity: Known Formats ===
    Pattern {
        name: "github_token",
        regex: r"(?:gh[oprsu]|github_pat)_[\dA-Za-z_]{36,}",
        severity: Severity::High,
    },
    Pattern {
        name: "gitlab_token",
        regex: r"glpat-[\dA-Za-z_=-]{20,}",
        severity: Severity::High,
    },
    Pattern {
        name: "aws_secret",
        regex: r"(?i)aws.{0,20}['\"][0-9a-zA-Z/+]{40}['\"]",
        severity: Severity::High,
    },
    Pattern {
        name: "openai_key",
        regex: r"sk-[A-Za-z0-9]{48}",
        severity: Severity::High,
    },
    Pattern {
        name: "anthropic_key",
        regex: r"sk-ant-[\dA-Za-z_-]{90,110}",
        severity: Severity::High,
    },
    Pattern {
        name: "age_secret_key",
        regex: r"AGE-SECRET-KEY-1[\dA-Z]{58}",
        severity: Severity::High,
    },
    Pattern {
        name: "stripe_key",
        regex: r"[rs]k_live_[\dA-Za-z]{24,}",
        severity: Severity::High,
    },
    Pattern {
        name: "slack_token",
        regex: r"xox[aboprs]-(?:\d+-)+[\da-z]+",
        severity: Severity::High,
    },
    Pattern {
        name: "private_key_pem",
        regex: r"-{5}BEGIN (?:RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-{5}",
        severity: Severity::High,
    },

    // === Medium Severity: Heuristics ===
    Pattern {
        name: "url_credentials",
        regex: r"[a-z]+://[^:]+:[^@]{8,}@[\w./-]+",
        severity: Severity::Medium,
    },
    Pattern {
        name: "generic_secret",
        regex: r#"(?i)(?:password|secret|token|api_key)\s*[:=]\s*["'][^"']{8,}["']"#,
        severity: Severity::Medium,
    },
];
```

---

## Implementation

Use `RegexSet` for efficient "which patterns matched?" queries.

```rust
use once_cell::sync::Lazy;
use regex::{Regex, RegexSet};

static REGEX_SET: Lazy<RegexSet> = Lazy::new(|| {
    RegexSet::new(patterns::PATTERNS.iter().map(|p| p.regex))
        .expect("invalid pattern regex")
});

// Individual regexes for extracting match details
static REGEXES: Lazy<Vec<Regex>> = Lazy::new(|| {
    patterns::PATTERNS
        .iter()
        .map(|p| Regex::new(p.regex).expect("invalid pattern regex"))
        .collect()
});

pub fn scan_files(
    paths: impl IntoIterator<Item = impl AsRef<Path>>
) -> Result<Vec<Finding>, ScanError> {
    let mut findings = Vec::new();

    for path in paths {
        let path = path.as_ref();

        if should_skip(path) {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => continue, // binary
            Err(e) => return Err(ScanError::ReadError {
                path: path.to_owned(),
                source: e
            }),
        };

        for (line_num, line) in content.lines().enumerate() {
            let matches: Vec<usize> = REGEX_SET.matches(line).iter().collect();

            for idx in matches {
                let pattern = &patterns::PATTERNS[idx];
                if let Some(m) = REGEXES[idx].find(line) {
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

    Ok(findings)
}

pub fn scan_tracked(repo_root: &Path) -> Result<Vec<Finding>, ScanError> {
    let files = git_ls_files(repo_root)?;
    scan_files(files)
}

pub fn scan_staged(repo_root: &Path) -> Result<Vec<Finding>, ScanError> {
    let files = git_staged_files(repo_root)?;
    scan_files(files)
}

fn should_skip(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Lock files have hashes that look like secrets
    name.ends_with(".lock")
}

fn redact(s: &str) -> String {
    if s.len() <= 8 {
        "*".repeat(s.len())
    } else {
        format!("{}...{}", &s[..4], &s[s.len()-4..])
    }
}

fn git_ls_files(repo_root: &Path) -> Result<Vec<PathBuf>, ScanError> {
    let output = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| ScanError::GitFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(ScanError::NotAGitRepo(repo_root.to_owned()));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| repo_root.join(s))
        .collect())
}

fn git_staged_files(repo_root: &Path) -> Result<Vec<PathBuf>, ScanError> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "-z"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| ScanError::GitFailed(e.to_string()))?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| repo_root.join(s))
        .collect())
}
```

---

## CLI Commands

### `patina secrets check`

```bash
$ patina secrets check
Scanning 3 staged files...

Found 1 secret:

  src/config.rs:42:15
    Pattern: anthropic_key
    Severity: HIGH
    Match: sk-a...xyz1

Commit blocked. Remove secret or use `patina secrets add`.
```

Exit codes:
- `0` - Clean
- `1` - Secrets found
- `2` - Error

### `patina secrets audit`

```bash
$ patina secrets audit
Scanning 1510 tracked files...

All clear - no secrets found.
```

### `patina secrets` (status)

Existing status output unchanged. Scanner doesn't affect vault status.

---

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn scan_content(content: &str) -> Vec<Finding> {
        // Helper that scans a string directly
        scan_string(content)
    }

    #[test]
    fn detects_github_token() {
        let findings = scan_content(
            r#"token = "ghp_ABC123def456GHI789jkl012MNO345pqr678""#
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].pattern, "github_token");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn detects_anthropic_key() {
        let findings = scan_content(
            r#"key = "sk-ant-api03-verylongstringhere...""#
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].pattern, "anthropic_key");
    }

    #[test]
    fn ignores_lock_files() {
        assert!(should_skip(Path::new("Cargo.lock")));
        assert!(should_skip(Path::new("package-lock.json")));
    }

    #[test]
    fn redacts_secrets() {
        let redacted = redact("ghp_ABC123def456GHI789jkl012MNO345pqr678");
        assert!(redacted.contains("..."));
        assert!(!redacted.contains("ABC123"));
    }

    #[test]
    fn no_false_positive_on_placeholder() {
        let findings = scan_content(r#"token = "your-token-here""#);
        assert!(findings.is_empty());
    }
}
```

Each pattern gets a test. Integration tests scan fixture directories.

---

## Excluded from Scope

1. **Custom patterns** - Built-in patterns cover 90%+. Add trait abstraction when needed.

2. **Parallel scanning** - Consciously deferred. Add rayon if benchmarks show need.

3. **Git history scanning** - Different problem. Use `git filter-repo` for history.

4. **Live validation** - Not calling APIs to verify secrets are active.

5. **Hook installation** - Manual pre-commit setup for now.

---

## Implementation Order

1. `src/scanner/error.rs` - ScanError enum
2. `src/scanner/patterns.rs` - Pattern definitions
3. `src/scanner/mod.rs` - Core API + implementation
4. `src/lib.rs` - Export scanner module
5. `src/commands/secrets.rs` - Add Check/Audit subcommands
6. Tests for each pattern

---

## Exit Criteria (0.8.6)

- [ ] `src/scanner/` module exists, independent of `src/secrets/`
- [ ] `scanner::scan_files()` works with arbitrary paths
- [ ] `scanner::scan_tracked()` scans git-tracked files
- [ ] `scanner::scan_staged()` scans staged files only
- [ ] `ScanError` provides typed, matchable errors
- [ ] `patina secrets check` scans staged files
- [ ] `patina secrets audit` scans all tracked files
- [ ] Each pattern has a unit test
- [ ] Patina repo passes `patina secrets audit`

---

## References

- [ripsecrets](https://github.com/sirwart/ripsecrets) - Pattern inspiration (MIT)
- [regex crate - RegexSet](https://docs.rs/regex/latest/regex/struct.RegexSet.html)
- Session 20260126-111115 - Tool research
- Session 20260126-134036 - Design discussion
