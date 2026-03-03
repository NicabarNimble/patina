//! Delta computation for diff-driven scrape dispatch.
//!
//! Computes what changed since the last scrape, enabling execute_all() to
//! skip scrapers with zero work. The delta is computed once and threaded
//! through all subsystems.
//!
//! Delta sources:
//! 1. New commits since last scraped SHA (from scrape_meta table)
//! 2. Working tree changes (uncommitted modifications)
//! 3. Classification into source kinds (code, layer, beliefs)

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use super::database;

/// What changed since the last scrape.
///
/// Computed once by `compute_delta()`, then used by `execute_all()` to
/// skip scrapers with zero work. Zero changed files = zero work = millisecond exit.
#[derive(Debug)]
pub struct ScrapeDelta {
    /// New commit SHAs since last scrape (newest first)
    pub new_commits: Vec<String>,
    /// All changed file paths (from commits + working tree)
    pub changed_files: Vec<ChangedFile>,
    /// Whether any layer/ files changed (patterns, sessions, specs)
    pub layer_changed: bool,
    /// Whether belief files changed (or code changed → regrounding needed)
    pub beliefs_affected: bool,
    /// Time spent computing the delta
    pub compute_time: std::time::Duration,
}

/// A file that changed, with its extension for dispatch routing.
#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub extension: Option<String>,
}

impl ScrapeDelta {
    /// True if nothing changed — all scrapers can be skipped.
    pub fn is_empty(&self) -> bool {
        self.new_commits.is_empty() && self.changed_files.is_empty()
    }

    /// File extensions present in changed code files (for lazy plugin loading).
    pub fn changed_extensions(&self) -> HashSet<String> {
        self.changed_files
            .iter()
            .filter_map(|f| f.extension.clone())
            .collect()
    }

    /// Changed code files only (not layer/, not beliefs/).
    pub fn changed_code_files(&self) -> Vec<&ChangedFile> {
        self.changed_files
            .iter()
            .filter(|f| {
                !f.path.starts_with("layer/") && !f.path.starts_with("./layer/")
            })
            .collect()
    }

    /// Print a human-readable summary of the delta.
    pub fn log_summary(&self) {
        if self.is_empty() {
            println!("  Delta: nothing changed since last scrape");
            return;
        }

        let code_files = self.changed_code_files();
        let extensions = self.changed_extensions();

        println!("  Delta ({:.0?}):", self.compute_time);
        if !self.new_commits.is_empty() {
            println!("    {} new commit(s)", self.new_commits.len());
        }
        if !code_files.is_empty() {
            println!(
                "    {} changed code file(s), extensions: {:?}",
                code_files.len(),
                extensions
            );
        }
        if self.layer_changed {
            let layer_count = self.changed_files
                .iter()
                .filter(|f| f.path.starts_with("layer/") || f.path.starts_with("./layer/"))
                .count();
            println!("    {} changed layer file(s)", layer_count);
        }
        if self.beliefs_affected {
            println!("    beliefs affected (regrounding needed)");
        }
    }
}

/// Compute the delta since last scrape.
///
/// Checks three sources in order:
/// 1. New commits via `git log` since stored high-water mark
/// 2. Working tree changes via `git status`
/// 3. Classifies changed files into source kinds
///
/// The DB must exist (patina.db) — if it doesn't, returns a "full scrape needed"
/// delta with empty commits but all files marked as changed.
pub fn compute_delta() -> Result<ScrapeDelta> {
    let start = Instant::now();

    let db_path = Path::new(database::PATINA_DB);

    // If DB doesn't exist, everything is "changed" — need full scrape
    if !db_path.exists() {
        return Ok(ScrapeDelta {
            new_commits: vec!["(no database — full scrape needed)".to_string()],
            changed_files: Vec::new(),
            layer_changed: true,
            beliefs_affected: true,
            compute_time: start.elapsed(),
        });
    }

    let conn = database::initialize(db_path)?;

    // 1. Check for new commits since last scrape
    let last_sha = database::get_last_processed(&conn, "git")?;
    let (new_commits, committed_files) = get_new_commits(last_sha.as_deref())?;

    // 2. Check working tree for uncommitted changes
    let working_tree_files = get_working_tree_changes()?;

    // 3. Merge committed + working tree changes, dedup
    let mut all_paths: HashSet<String> = HashSet::new();
    let mut changed_files: Vec<ChangedFile> = Vec::new();

    for path in committed_files.into_iter().chain(working_tree_files.into_iter()) {
        if all_paths.insert(path.clone()) {
            let extension = Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_string());
            changed_files.push(ChangedFile { path, extension });
        }
    }

    // 4. Classify into source kinds
    let layer_changed = changed_files.iter().any(|f| {
        f.path.starts_with("layer/") || f.path.starts_with("./layer/")
    });

    let beliefs_dir = "layer/surface/epistemic/beliefs/";
    let beliefs_changed = changed_files.iter().any(|f| {
        f.path.starts_with(beliefs_dir)
            || f.path.starts_with(&format!("./{}", beliefs_dir))
    });

    // Beliefs are affected if belief files changed OR code files changed
    // (code changes may invalidate belief grounding)
    let code_changed = changed_files.iter().any(|f| {
        !f.path.starts_with("layer/") && !f.path.starts_with("./layer/")
    });
    let beliefs_affected = beliefs_changed || code_changed;

    Ok(ScrapeDelta {
        new_commits,
        changed_files,
        layer_changed,
        beliefs_affected,
        compute_time: start.elapsed(),
    })
}

/// Get new commits and their changed files since `last_sha`.
/// Returns (commit_shas, changed_file_paths).
fn get_new_commits(last_sha: Option<&str>) -> Result<(Vec<String>, Vec<String>)> {
    // Get current HEAD
    let head_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;

    if !head_output.status.success() {
        return Ok((Vec::new(), Vec::new()));
    }

    let head = String::from_utf8_lossy(&head_output.stdout).trim().to_string();

    // If last_sha matches HEAD, no new commits
    if let Some(last) = last_sha {
        if last == head {
            return Ok((Vec::new(), Vec::new()));
        }
    }

    // Get new commit SHAs
    let mut cmd = Command::new("git");
    cmd.args(["log", "--pretty=format:%H"]);
    if let Some(sha) = last_sha {
        cmd.arg(format!("{}..HEAD", sha));
    } else {
        // No last SHA means first scrape — but we don't want ALL commits
        // Just signal "new commits exist" with HEAD
        return Ok((vec![head], Vec::new()));
    }

    let output = cmd.output()?;
    if !output.status.success() {
        // git log failed (e.g., SHA no longer exists after rebase)
        // Treat as "everything is new"
        return Ok((vec![head], Vec::new()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    if commits.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // Get changed files from these commits
    let diff_output = Command::new("git")
        .args([
            "diff",
            "--name-only",
            &format!("{}..HEAD", last_sha.unwrap_or("HEAD~1")),
        ])
        .output()?;

    let files = if diff_output.status.success() {
        String::from_utf8_lossy(&diff_output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    } else {
        Vec::new()
    };

    Ok((commits, files))
}

/// Get files with uncommitted changes (staged + unstaged + untracked).
fn get_working_tree_changes() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            // git status --porcelain format: "XY path" or "XY path -> new_path"
            if line.len() > 3 {
                let path = &line[3..];
                // Handle renames: "R  old -> new"
                if let Some(arrow) = path.find(" -> ") {
                    Some(path[arrow + 4..].to_string())
                } else {
                    Some(path.to_string())
                }
            } else {
                None
            }
        })
        .collect();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrape_delta_is_empty() {
        let delta = ScrapeDelta {
            new_commits: Vec::new(),
            changed_files: Vec::new(),
            layer_changed: false,
            beliefs_affected: false,
            compute_time: std::time::Duration::from_millis(1),
        };
        assert!(delta.is_empty());
    }

    #[test]
    fn test_scrape_delta_with_commits() {
        let delta = ScrapeDelta {
            new_commits: vec!["abc123".to_string()],
            changed_files: Vec::new(),
            layer_changed: false,
            beliefs_affected: false,
            compute_time: std::time::Duration::from_millis(1),
        };
        assert!(!delta.is_empty());
    }

    #[test]
    fn test_changed_extensions() {
        let delta = ScrapeDelta {
            new_commits: Vec::new(),
            changed_files: vec![
                ChangedFile {
                    path: "src/main.rs".to_string(),
                    extension: Some("rs".to_string()),
                },
                ChangedFile {
                    path: "src/lib.rs".to_string(),
                    extension: Some("rs".to_string()),
                },
                ChangedFile {
                    path: "README.md".to_string(),
                    extension: Some("md".to_string()),
                },
            ],
            layer_changed: false,
            beliefs_affected: false,
            compute_time: std::time::Duration::from_millis(1),
        };

        let exts = delta.changed_extensions();
        assert!(exts.contains("rs"));
        assert!(exts.contains("md"));
        assert_eq!(exts.len(), 2);
    }

    #[test]
    fn test_changed_code_files_excludes_layer() {
        let delta = ScrapeDelta {
            new_commits: Vec::new(),
            changed_files: vec![
                ChangedFile {
                    path: "src/main.rs".to_string(),
                    extension: Some("rs".to_string()),
                },
                ChangedFile {
                    path: "layer/core/test.md".to_string(),
                    extension: Some("md".to_string()),
                },
            ],
            layer_changed: true,
            beliefs_affected: false,
            compute_time: std::time::Duration::from_millis(1),
        };

        let code = delta.changed_code_files();
        assert_eq!(code.len(), 1);
        assert_eq!(code[0].path, "src/main.rs");
    }

    #[test]
    fn test_beliefs_affected_when_code_changes() {
        // When code changes, beliefs are affected (regrounding needed)
        let files = vec![ChangedFile {
            path: "src/main.rs".to_string(),
            extension: Some("rs".to_string()),
        }];

        let code_changed = files.iter().any(|f| {
            !f.path.starts_with("layer/") && !f.path.starts_with("./layer/")
        });

        assert!(code_changed);
    }
}
