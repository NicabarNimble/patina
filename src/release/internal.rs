//! Internal implementation for release module
//!
//! Contains strategy detection, safeguard checks, and release execution
//! for each ReleaseStrategy variant.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use super::{BumpType, PreparedRelease, ReleaseStrategy};

// ============================================================================
// Strategy Detection
// ============================================================================

/// Detect release strategy from project config and files.
///
/// Resolution chain:
/// 1. Explicit config wins: .patina/config.toml [versioning] strategy
/// 2. Auto-detect from project files
pub(crate) fn detect_strategy(project_path: &Path) -> ReleaseStrategy {
    // 1. Check explicit config
    if let Some(strategy) = read_config_strategy(project_path) {
        return strategy;
    }

    // 2. Auto-detect from project files
    let cargo_toml = project_path.join("Cargo.toml");
    if cargo_toml.exists() {
        // Check if this is a fork (upstream.owned = false)
        if !crate::project::is_versioning_enabled(project_path) {
            return ReleaseStrategy::None;
        }
        return ReleaseStrategy::Cargo;
    }

    // Other language files → External (advisory)
    let external_markers = ["package.json", "pyproject.toml", "setup.py", "go.mod"];
    for marker in &external_markers {
        if project_path.join(marker).exists() {
            return ReleaseStrategy::External;
        }
    }

    // Nothing detected → None
    ReleaseStrategy::None
}

/// Read explicit strategy from .patina/config.toml [versioning] section
fn read_config_strategy(project_path: &Path) -> Option<ReleaseStrategy> {
    let config_path = project_path.join(".patina/config.toml");
    let content = fs::read_to_string(config_path).ok()?;

    // Parse as toml::Value to check for [versioning] section
    let table: toml::Value = toml::from_str(&content).ok()?;
    let strategy_str = table.get("versioning")?.get("strategy")?.as_str()?;

    match strategy_str {
        "cargo" => Some(ReleaseStrategy::Cargo),
        "external" => Some(ReleaseStrategy::External),
        "none" => Some(ReleaseStrategy::None),
        _ => None, // Unknown strategy → fall through to auto-detect
    }
}

// ============================================================================
// Preflight
// ============================================================================

/// Run preflight checks and produce a PreparedRelease token.
///
/// `spec_path` is the file being released — expected to be dirty (just had
/// its status updated). Excluded from the clean-tree check.
pub(crate) fn preflight(
    strategy: ReleaseStrategy,
    bump: BumpType,
    spec_path: &str,
) -> Result<PreparedRelease> {
    match strategy {
        ReleaseStrategy::Cargo => preflight_cargo(bump, spec_path),
        ReleaseStrategy::External => preflight_external(bump),
        ReleaseStrategy::None => Ok(PreparedRelease {
            strategy,
            bump,
            old_version: None,
            new_version: None,
        }),
    }
}

/// Cargo preflight: read version, compute next, run safeguards.
fn preflight_cargo(bump: BumpType, spec_path: &str) -> Result<PreparedRelease> {
    let old_version = read_cargo_version()?;
    let new_version = compute_next_version(&old_version, bump)?;

    // Run safeguard checks (spec_path excluded from dirty-tree check)
    run_safeguard_checks(&new_version, spec_path)?;

    Ok(PreparedRelease {
        strategy: ReleaseStrategy::Cargo,
        bump,
        old_version: Some(old_version),
        new_version: Some(new_version),
    })
}

/// External preflight: verify a version file exists so the reminder is actionable.
fn preflight_external(bump: BumpType) -> Result<PreparedRelease> {
    // Check that at least one version file exists
    let project_path = Path::new(".");
    let version_files = ["package.json", "pyproject.toml", "setup.py", "go.mod"];
    let found = version_files.iter().any(|f| project_path.join(f).exists());

    if !found {
        anyhow::bail!(
            "External strategy selected but no version file found.\n\
             Expected one of: {}\n\
             Override strategy in .patina/config.toml if needed.",
            version_files.join(", ")
        );
    }

    Ok(PreparedRelease {
        strategy: ReleaseStrategy::External,
        bump,
        old_version: None,
        new_version: None,
    })
}

// ============================================================================
// Safeguard Checks (migrated from version/internal.rs)
// ============================================================================

/// Blocking safeguard checks before any Cargo release.
///
/// `spec_path` is excluded from the dirty-tree check — we just updated
/// its status and will include it in the release commit.
///
/// Checks (all blocking):
/// 1. Clean working tree (excluding spec_path)
/// 2. Not behind remote
/// 3. Not diverged from remote
/// 4. Target tag doesn't already exist
/// 5. Index exists
fn run_safeguard_checks(new_version: &str, spec_path: &str) -> Result<()> {
    use crate::git;

    // 1. Dirty tree check (excluding untracked files and the spec file we're releasing)
    let dirty = git::status_porcelain()?;
    let unexpected: Vec<&str> = dirty
        .lines()
        .filter(|line| {
            // status_porcelain format: "XY path" or "XY path -> path"
            // Skip untracked files (??) — they won't be in the release commit
            if line.starts_with("??") {
                return false;
            }
            let path = line
                .get(3..)
                .unwrap_or("")
                .split(" -> ")
                .next()
                .unwrap_or("");
            !path.is_empty() && path != spec_path
        })
        .collect();

    if !unexpected.is_empty() {
        anyhow::bail!(
            "Working tree has uncommitted changes ({} files)\n\n\
             Commit your changes first:\n\
               git add -A && git commit -m \"your message\"\n\n\
             Or stash them:\n\
               git stash",
            unexpected.len()
        );
    }

    // 2. Behind remote check
    if git::has_upstream()? {
        let behind = git::commits_behind_upstream()?;
        if behind > 0 {
            anyhow::bail!(
                "Branch is {} commits behind remote\n\n\
                 Pull changes first:\n\
                   git pull --rebase",
                behind
            );
        }
    }

    // 3. Diverged check
    if git::is_diverged()? {
        anyhow::bail!(
            "Branch has diverged from remote (both ahead and behind)\n\n\
             Resolve the divergence first:\n\
               git pull --rebase"
        );
    }

    // 4. Tag exists check
    let tag_name = format!("v{}", new_version);
    if git::tag_exists(&tag_name)? {
        anyhow::bail!(
            "Tag '{}' already exists\n\n\
             This version has already been released.",
            tag_name
        );
    }

    // 5. Index stale check
    check_index_freshness()?;

    // Non-blocking warning: not on dev branch (from config, fallback "work")
    let branch = git::current_branch()?;
    let dev_branch = std::env::current_dir()
        .ok()
        .and_then(|p| crate::project::load(&p).ok())
        .map(|c| c.project.branch)
        .unwrap_or_else(|| "work".to_string());
    if branch != dev_branch {
        eprintln!(
            "  Warning: Not on '{}' branch (currently on '{}')",
            dev_branch, branch
        );
    }

    Ok(())
}

/// Check if the spec index exists.
///
/// We verify the database exists so that release commands have spec awareness.
/// Fine-grained staleness checks are left to `patina scrape` — checking every
/// spec file on every release is wasteful when the scrape timestamp is visible.
fn check_index_freshness() -> Result<()> {
    let db_path = Path::new(".patina/local/data/patina.db");
    if !db_path.exists() {
        anyhow::bail!(
            "No index found\n\n\
             Run 'patina scrape layer' first to build the index."
        );
    }

    Ok(())
}

// ============================================================================
// Release Execution
// ============================================================================

/// Execute a prepared release.
///
/// When `archive_dir` is provided, the spec directory is removed and folded
/// into the release commit (Cargo strategy only).
pub(crate) fn execute_release(
    prepared: PreparedRelease,
    title: &str,
    spec_path: &str,
    archive_dir: Option<&str>,
) -> Result<()> {
    match prepared.strategy {
        ReleaseStrategy::Cargo => execute_cargo(prepared, title, spec_path, archive_dir),
        ReleaseStrategy::External => execute_external(prepared, title),
        ReleaseStrategy::None => Ok(()), // Silent no-op
    }
}

/// Cargo release: update Cargo.toml, commit, tag.
///
/// When `archive_dir` is provided, the spec directory is `git rm -r`'d
/// and included in the release commit. The caller is responsible for
/// creating the `spec/<id>` tag before calling this (so it points to
/// the commit that still has the spec).
fn execute_cargo(
    prepared: PreparedRelease,
    title: &str,
    spec_path: &str,
    archive_dir: Option<&str>,
) -> Result<()> {
    let new_version = prepared
        .new_version
        .as_deref()
        .expect("Cargo preflight always sets new_version");
    let old_version = prepared
        .old_version
        .as_deref()
        .expect("Cargo preflight always sets old_version");

    // 1. Update Cargo.toml
    update_cargo_version(new_version)?;

    // 2. Stage files
    if let Some(dir) = archive_dir {
        // Archive mode: force-remove spec directory (stages deletion), then add Cargo.toml.
        // -f needed because the spec file was just modified (status update) on disk.
        let output = Command::new("git")
            .args(["rm", "-rf", dir])
            .output()
            .context("Failed to remove spec directory")?;
        if !output.status.success() {
            anyhow::bail!("git rm failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        let output = Command::new("git")
            .args(["add", "Cargo.toml"])
            .output()
            .context("Failed to stage Cargo.toml")?;
        if !output.status.success() {
            anyhow::bail!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    } else {
        // Normal mode: stage Cargo.toml + spec file
        let output = Command::new("git")
            .args(["add", "Cargo.toml", spec_path])
            .output()
            .context("Failed to stage files")?;
        if !output.status.success() {
            anyhow::bail!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    // 3. Commit
    let commit_msg = format!("release: v{} — {}", new_version, title);
    let output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("Failed to commit")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("nothing to commit") {
            anyhow::bail!("git commit failed: {}", stderr);
        }
    }

    // 4. Create version tag
    let tag_name = format!("v{}", new_version);
    crate::git::create_tag(&tag_name, title)?;

    println!(
        "\n  Spec type '{}' → {} bump",
        prepared.bump.label(),
        prepared.bump.label()
    );
    println!("  Version: {} → {}", old_version, new_version);
    println!("  Tagged: v{}", new_version);

    Ok(())
}

/// External release: print recommendation, don't touch files.
fn execute_external(prepared: PreparedRelease, title: &str) -> Result<()> {
    let bump_label = prepared.bump.label();
    println!();
    println!("  Spec type → {} bump", bump_label);
    println!("  Version management: external (not managed by patina)");

    match prepared.bump {
        BumpType::Major => {
            println!("  Action needed: bump to next major version and tag manually");
        }
        _ => {
            println!(
                "  Action needed: bump your {} version and tag manually",
                bump_label
            );
        }
    }

    println!("  Spec: {}", title);

    Ok(())
}

// ============================================================================
// Version Helpers
// ============================================================================

/// Read current version from Cargo.toml
fn read_cargo_version() -> Result<String> {
    let content = fs::read_to_string("Cargo.toml")?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") && trimmed.contains('=') {
            if let Some(version) = trimmed.split('=').nth(1) {
                let version = version.trim().trim_matches('"').trim_matches('\'');
                return Ok(version.to_string());
            }
        }
    }
    anyhow::bail!("Could not find version in Cargo.toml")
}

/// Compute next version from current version and bump type
fn compute_next_version(current: &str, bump: BumpType) -> Result<String> {
    let parts: Vec<u32> = current
        .split('.')
        .map(|s| s.parse::<u32>().context("Invalid version component"))
        .collect::<Result<Vec<_>>>()?;

    if parts.len() != 3 {
        anyhow::bail!("Expected semver format (x.y.z), got '{}'", current);
    }

    Ok(match bump {
        BumpType::Patch => format!("{}.{}.{}", parts[0], parts[1], parts[2] + 1),
        BumpType::Minor => format!("{}.{}.0", parts[0], parts[1] + 1),
        BumpType::Major => format!("{}.0.0", parts[0] + 1),
    })
}

/// Update version in Cargo.toml's [package] section
fn update_cargo_version(new_version: &str) -> Result<()> {
    let path = Path::new("Cargo.toml");
    let content = fs::read_to_string(path)?;

    let mut in_package_section = false;
    let mut version_updated = false;
    let mut new_content = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package_section = trimmed == "[package]";
        }
        if in_package_section && !version_updated && trimmed.starts_with("version") {
            new_content.push_str(&format!("version = \"{}\"\n", new_version));
            version_updated = true;
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    if !version_updated {
        anyhow::bail!("Could not find version field in [package] section of Cargo.toml");
    }

    fs::write(path, new_content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_next_version_patch() {
        assert_eq!(
            compute_next_version("0.15.3", BumpType::Patch).unwrap(),
            "0.15.4"
        );
    }

    #[test]
    fn test_compute_next_version_minor() {
        assert_eq!(
            compute_next_version("0.15.3", BumpType::Minor).unwrap(),
            "0.16.0"
        );
    }

    #[test]
    fn test_compute_next_version_major() {
        assert_eq!(
            compute_next_version("0.15.3", BumpType::Major).unwrap(),
            "1.0.0"
        );
    }

    #[test]
    fn test_bump_type_from_spec_type() {
        assert_eq!(BumpType::from_spec_type("feat"), Some(BumpType::Minor));
        assert_eq!(BumpType::from_spec_type("fix"), Some(BumpType::Patch));
        assert_eq!(BumpType::from_spec_type("refactor"), Some(BumpType::Patch));
        assert_eq!(BumpType::from_spec_type("explore"), None);
        assert_eq!(BumpType::from_spec_type("unknown"), None);
    }
}
