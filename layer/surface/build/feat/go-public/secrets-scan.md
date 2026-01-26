# Secrets Scanning Module

> Native secret detection for Patina - scan tracked files before going public.

**Parent:** [go-public/SPEC.md](./SPEC.md) (0.8.6 milestone)
**Status:** design
**Session:** 20260126-111115

---

## Goal

Implement `patina secrets` commands that:
1. Scan git-tracked files for secrets (pre-commit use case)
2. Respect `.gitignore` natively (no false positives from ignored directories)
3. Integrate with `patina doctor` for pre-public audit
4. Behave differently for owned vs contrib projects

---

## Design Decision: Native vs Dependency

**Evaluated:**
- `ripsecrets` - Rust crate, 905 lines, MIT, pre-commit focused
- `noseyparker` - Rust binary only, 30MB, full history scanning

**Decision: Native implementation**

Rationale:
1. ripsecrets patterns are simple regex (MIT licensed, can copy)
2. Skip statistical randomness detection (complex, marginal benefit)
3. Deep integration with Patina's git infrastructure
4. Return structured `Finding` objects, not terminal output
5. ~200-300 lines vs external dependency

---

## Architecture

```
src/secrets/
├── mod.rs          # Public API: scan(), check()
├── patterns.rs     # Secret detection regex patterns
└── internal.rs     # Scanning implementation
```

### Public Interface (mod.rs)

```rust
//! Secret scanning for Patina projects.
//!
//! "Do X": Detect hardcoded secrets in tracked files.

mod patterns;
mod internal;

pub use internal::{scan, Finding, Severity};

/// A detected secret
#[derive(Debug, Clone)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub pattern_name: &'static str,
    pub matched: String,       // The matched text (redacted in display)
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    High,      // Known secret format (GitHub token, AWS key, etc.)
    Medium,    // Generic password/secret assignment
}

/// Scan git-tracked files for secrets
pub fn scan(repo_root: &Path) -> Result<Vec<Finding>> {
    internal::scan_tracked_files(repo_root)
}

/// Check staged files only (pre-commit use case)
pub fn check_staged(repo_root: &Path) -> Result<Vec<Finding>> {
    internal::scan_staged_files(repo_root)
}
```

### Patterns (patterns.rs)

Adapted from ripsecrets (MIT licensed):

```rust
//! Secret detection patterns.
//!
//! Source: ripsecrets (MIT) - https://github.com/sirwart/ripsecrets

pub struct Pattern {
    pub name: &'static str,
    pub regex: &'static str,
    pub severity: Severity,
}

pub const PATTERNS: &[Pattern] = &[
    // === High Severity: Known Secret Formats ===

    // GitHub tokens
    Pattern {
        name: "github_token",
        regex: r"(?:gh[oprsu]|github_pat)_[\dA-Za-z_]{36}",
        severity: Severity::High,
    },
    // GitLab tokens
    Pattern {
        name: "gitlab_token",
        regex: r"glpat-[\dA-Za-z_=-]{20,22}",
        severity: Severity::High,
    },
    // Stripe keys
    Pattern {
        name: "stripe_key",
        regex: r"[rs]k_live_[\dA-Za-z]{24,247}",
        severity: Severity::High,
    },
    // AWS Secret Access Key
    Pattern {
        name: "aws_secret",
        regex: r"(?i)aws.{0,20}['\"][0-9a-zA-Z/+]{40}['\"]",
        severity: Severity::High,
    },
    // Azure Storage Account Key
    Pattern {
        name: "azure_storage",
        regex: r"AccountKey=[\d+/=A-Za-z]{88}",
        severity: Severity::High,
    },
    // GCP API Key
    Pattern {
        name: "gcp_api_key",
        regex: r"AIzaSy[\dA-Za-z_-]{33}",
        severity: Severity::High,
    },
    // OpenAI API Key
    Pattern {
        name: "openai_key",
        regex: r"sk-[A-Za-z0-9]{48}",
        severity: Severity::High,
    },
    // Anthropic API Key
    Pattern {
        name: "anthropic_key",
        regex: r"sk-ant-[\dA-Za-z_-]{90,110}",
        severity: Severity::High,
    },
    // Age secret key (we use this!)
    Pattern {
        name: "age_secret_key",
        regex: r"AGE-SECRET-KEY-1[\dA-Z]{58}",
        severity: Severity::High,
    },
    // Slack tokens
    Pattern {
        name: "slack_token",
        regex: r"xox[aboprs]-(?:\d+-)+[\da-z]+",
        severity: Severity::High,
    },
    // npm tokens
    Pattern {
        name: "npm_token",
        regex: r"npm_[\dA-Za-z]{36}",
        severity: Severity::High,
    },
    // JWT tokens
    Pattern {
        name: "jwt",
        regex: r"\beyJ[\dA-Za-z=_-]+(?:\.[\dA-Za-z=_-]{3,}){1,4}",
        severity: Severity::High,
    },

    // === Private Keys ===
    Pattern {
        name: "private_key_pem",
        regex: r"-{5}BEGIN (?:RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-{5}",
        severity: Severity::High,
    },
    Pattern {
        name: "putty_private_key",
        regex: r"PuTTY-User-Key-File-2",
        severity: Severity::High,
    },

    // === Medium Severity: URL Credentials ===
    Pattern {
        name: "url_credentials",
        regex: r"[A-Za-z]+://[^:]+:([^@]{8,})@[\dA-Za-z#%&+./:=?_~-]+",
        severity: Severity::Medium,
    },

    // === Medium Severity: Generic Assignments ===
    // password = "..." or secret = "..." etc.
    Pattern {
        name: "generic_secret",
        regex: r#"(?i)(?:password|secret|token|api_key|apikey|auth)\s*[:=]\s*["'][^"']{8,}["']"#,
        severity: Severity::Medium,
    },
];

/// Compile all patterns into a single regex for efficiency
pub fn combined_regex() -> String {
    let mut combined = String::from("(");
    for (i, pattern) in PATTERNS.iter().enumerate() {
        if i > 0 {
            combined.push('|');
        }
        // Named capture group for pattern identification
        combined.push_str(&format!("(?P<{}>{})", pattern.name, pattern.regex));
    }
    combined.push(')');
    combined
}
```

### Implementation (internal.rs)

```rust
//! Internal scanning implementation.

use crate::git;  // Patina's git helpers
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Scan all git-tracked files
pub fn scan_tracked_files(repo_root: &Path) -> Result<Vec<Finding>> {
    let files = get_tracked_files(repo_root)?;
    scan_files(&files, repo_root)
}

/// Scan only staged files (pre-commit)
pub fn scan_staged_files(repo_root: &Path) -> Result<Vec<Finding>> {
    let files = get_staged_files(repo_root)?;
    scan_files(&files, repo_root)
}

/// Get list of tracked files via git ls-files
fn get_tracked_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])  // null-separated for safety
        .current_dir(repo_root)
        .output()?;

    parse_null_separated(&output.stdout, repo_root)
}

/// Get list of staged files via git diff --cached
fn get_staged_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "-z"])
        .current_dir(repo_root)
        .output()?;

    parse_null_separated(&output.stdout, repo_root)
}

fn parse_null_separated(bytes: &[u8], repo_root: &Path) -> Result<Vec<PathBuf>> {
    let s = String::from_utf8_lossy(bytes);
    Ok(s.split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| repo_root.join(s))
        .collect())
}

/// Scan a list of files for secrets
fn scan_files(files: &[PathBuf], repo_root: &Path) -> Result<Vec<Finding>> {
    let regex = Regex::new(&patterns::combined_regex())?;
    let mut findings = Vec::new();

    for file in files {
        // Skip binary files
        if is_binary(file) {
            continue;
        }

        // Skip files we know are safe
        if should_skip(file) {
            continue;
        }

        let content = std::fs::read_to_string(file)?;

        for (line_num, line) in content.lines().enumerate() {
            for cap in regex.captures_iter(line) {
                // Find which pattern matched
                for pattern in patterns::PATTERNS {
                    if let Some(m) = cap.name(pattern.name) {
                        findings.push(Finding {
                            path: file.strip_prefix(repo_root)
                                .unwrap_or(file)
                                .to_path_buf(),
                            line: line_num + 1,
                            column: m.start() + 1,
                            pattern_name: pattern.name,
                            matched: redact(m.as_str()),
                            severity: pattern.severity,
                        });
                    }
                }
            }
        }
    }

    Ok(findings)
}

/// Check if file is binary
fn is_binary(path: &Path) -> bool {
    // Simple heuristic: read first 8KB, look for null bytes
    if let Ok(bytes) = std::fs::read(path) {
        let check_len = bytes.len().min(8192);
        return bytes[..check_len].contains(&0);
    }
    false
}

/// Files to skip (known safe patterns)
fn should_skip(path: &Path) -> bool {
    let name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    // Lock files often have hashes that look like secrets
    if name.ends_with(".lock") || name == "Cargo.lock" {
        return true;
    }

    // Test fixtures may contain example secrets
    if path.to_string_lossy().contains("/test/")
        || path.to_string_lossy().contains("/fixtures/") {
        return true;
    }

    false
}

/// Redact a secret for display
fn redact(s: &str) -> String {
    if s.len() <= 8 {
        return "*".repeat(s.len());
    }
    format!("{}...{}", &s[..4], &s[s.len()-4..])
}
```

---

## CLI Commands

### `patina secrets check`

Pre-commit use case - scan staged files only.

```bash
$ patina secrets check
Scanning staged files for secrets...

Found 1 secret:

  src/config.rs:42:15
  Pattern: anthropic_key
  Match: sk-a...xyz1
  Severity: HIGH

Commit blocked. Remove secrets before committing.
```

Exit codes:
- `0` - No secrets found
- `1` - Secrets found (blocks commit)
- `2` - Error during scan

### `patina secrets audit`

Full audit of tracked files - used before going public.

```bash
$ patina secrets audit
Scanning 1510 tracked files for secrets...

All clear - no secrets found.

# Or with findings:
Found 3 secrets in 2 files:

  src/api.rs:15:20
    Pattern: openai_key
    Severity: HIGH

  config/dev.toml:8:12
    Pattern: generic_secret
    Severity: MEDIUM

  config/dev.toml:12:12
    Pattern: url_credentials
    Severity: MEDIUM

Run `patina secrets audit --help` for remediation options.
```

### `patina secrets patterns`

List available detection patterns.

```bash
$ patina secrets patterns
Secret Detection Patterns (18 total)

HIGH SEVERITY:
  github_token      GitHub personal access token or app token
  gitlab_token      GitLab personal access token
  stripe_key        Stripe live API key
  aws_secret        AWS Secret Access Key
  ...

MEDIUM SEVERITY:
  url_credentials   Credentials in URL (user:pass@host)
  generic_secret    Generic password/secret/token assignment
```

---

## Integration Points

### Pre-commit Hook

```bash
# .git/hooks/pre-commit (installed by patina)
#!/bin/sh
patina secrets check
```

Or via session-start which already manages git state.

### `patina doctor`

Add secrets check to doctor output:

```bash
$ patina doctor
...
Secrets audit:
  Tracked files: 1510
  Staged files: 3
  Status: clean
...
```

### Owned vs Contrib Behavior

| Project Type | `secrets check` | `secrets audit` |
|--------------|-----------------|-----------------|
| Owned/Local | Error + block commit | Error exit code |
| Contrib | Warning only | Warning only |

Contrib projects may have secrets from upstream - not our problem.

---

## Excluded from Scope

1. **Statistical randomness detection** - Complex (200 lines of probability math), marginal benefit. Known patterns catch 90%+ of real secrets.

2. **Git history scanning** - Would need to scan all blobs ever committed. Use `git filter-repo` or BFG for history cleaning if needed.

3. **Live validation** - Not phoning home to APIs to check if secrets are active. Privacy concern.

4. **Custom patterns via config** - Keep it simple for now. Can add later.

---

## Testing Strategy

1. **Test fixtures** with known secrets (in `test/secrets/`)
2. **No false positives** on Cargo.lock, test files, etc.
3. **Real scan** of Patina repo should be clean

---

## Implementation Order

1. `src/secrets/patterns.rs` - Pattern definitions
2. `src/secrets/internal.rs` - Scanning logic
3. `src/secrets/mod.rs` - Public API
4. `src/commands/secrets.rs` - CLI commands
5. Integration with `patina doctor`
6. Pre-commit hook installation

---

## Exit Criteria (for 0.8.6)

- [ ] `patina secrets check` scans staged files
- [ ] `patina secrets audit` scans all tracked files
- [ ] Patterns detect common secret formats
- [ ] Respects .gitignore via `git ls-files`
- [ ] `patina doctor` includes secrets status
- [ ] Patina repo passes `patina secrets audit`

---

## References

- [ripsecrets](https://github.com/sirwart/ripsecrets) - Pattern source (MIT)
- [Nosey Parker](https://github.com/praetorian-inc/noseyparker) - Evaluated but not used
- Session research on secret scanning tools
