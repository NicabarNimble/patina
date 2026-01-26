//! Internal implementation for version command
//!
//! All version management logic lives here. The public interface
//! in mod.rs exposes only what's needed.

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================================
// Data Structures
// ============================================================================

/// Version state stored in .patina/version.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionState {
    pub version: VersionInfo,
    #[serde(default)]
    pub metadata: VersionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current: String,
    pub phase: u32,
    pub phase_name: String,
    pub milestone: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionMetadata {
    #[serde(default = "default_history_file")]
    pub history_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_bump: Option<String>,
}

fn default_history_file() -> String {
    "layer/surface/build/feat/go-public/version-history.md".to_string()
}

impl Default for VersionState {
    fn default() -> Self {
        Self {
            version: VersionInfo {
                current: CORE_VERSION.to_string(),
                phase: 1,
                phase_name: "Initial".to_string(),
                milestone: 0,
            },
            metadata: VersionMetadata::default(),
        }
    }
}

// ============================================================================
// Public Functions (called from mod.rs)
// ============================================================================

/// Show version information
pub fn show_version(json_output: bool, components: bool) -> Result<()> {
    if json_output {
        output_json(components)?;
    } else {
        output_human(components)?;
    }
    Ok(())
}

/// Complete current spec milestone and bump version (spec-aware)
pub fn bump_milestone(description_override: Option<&str>, no_tag: bool, dry_run: bool) -> Result<()> {
    let project_path = Path::new(".");

    // 1. Get current milestone from spec index
    let (milestone, spec_path) = get_current_milestone_with_path()
        .ok_or_else(|| anyhow::anyhow!(
            "No current milestone found. Ensure spec has milestones and run 'patina scrape layer'."
        ))?;

    let description = description_override.unwrap_or(&milestone.name);
    let new_version = &milestone.version;

    // 2. Check if versioning is enabled (owned vs fork)
    let versioning_enabled = patina::project::is_versioning_enabled(project_path);

    // 3. Get next pending milestone
    let next_milestone = get_next_pending_milestone(&milestone.spec_id, new_version);

    // 4. Get current Cargo.toml version for comparison
    let old_version = read_cargo_version().unwrap_or_else(|_| "unknown".to_string());

    if dry_run {
        println!("Dry run - would perform these changes:\n");
        println!("Completing: {} v{} - {}", milestone.spec_id, new_version, description);
        if versioning_enabled {
            println!("Cargo.toml: {} -> {}", old_version, new_version);
        } else {
            println!("Cargo.toml: unchanged (fork repo)");
        }
        println!("Spec: mark {} complete, advance to {:?}", new_version, next_milestone);
        println!("\nFiles that would be updated:");
        println!("  - {}", spec_path);
        if versioning_enabled {
            println!("  - Cargo.toml");
        }
        if !no_tag && versioning_enabled {
            println!("  - git tag: v{}", new_version);
        }
        return Ok(());
    }

    // 5. Update spec YAML (mark complete, advance current_milestone)
    update_spec_milestone(&spec_path, new_version, next_milestone.as_deref())?;

    // 6. Update Cargo.toml (only for owned repos)
    if versioning_enabled {
        update_cargo_version(new_version)?;
    }

    // 7. Re-scrape layer to sync index
    println!("Syncing index...");
    if let Err(e) = rescrape_layer() {
        eprintln!("Warning: failed to re-scrape layer: {}", e);
    }

    // 8. Create git tag (only for owned repos)
    if !no_tag && versioning_enabled {
        create_git_tag(new_version, description)?;
    }

    // Output
    println!("\n✓ Milestone complete: {} v{}", milestone.spec_id, new_version);
    println!("  {}", description);
    if versioning_enabled {
        println!("  Cargo.toml: {} -> {}", old_version, new_version);
        if !no_tag {
            println!("  Tagged: v{}", new_version);
        }
    }
    if let Some(next) = &next_milestone {
        println!("  Next: v{}", next);
    } else {
        println!("  No more pending milestones!");
    }

    Ok(())
}

/// Get current milestone with spec file path
fn get_current_milestone_with_path() -> Option<(SpecMilestone, String)> {
    let db_path = Path::new(".patina/local/data/patina.db");
    if !db_path.exists() {
        return None;
    }

    let conn = Connection::open(db_path).ok()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT m.spec_id, m.version, m.name, m.status, p.file_path
            FROM patterns p
            JOIN milestones m ON p.id = m.spec_id AND p.current_milestone = m.version
            WHERE p.current_milestone IS NOT NULL
            LIMIT 1
            "#,
        )
        .ok()?;

    stmt.query_row([], |row| {
        Ok((
            SpecMilestone {
                spec_id: row.get(0)?,
                version: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
            },
            row.get::<_, String>(4)?,
        ))
    })
    .ok()
}

/// Get next pending milestone after completing current one
fn get_next_pending_milestone(spec_id: &str, current_version: &str) -> Option<String> {
    let db_path = Path::new(".patina/local/data/patina.db");
    if !db_path.exists() {
        return None;
    }

    let conn = Connection::open(db_path).ok()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT version FROM milestones
            WHERE spec_id = ?1 AND status = 'pending' AND version > ?2
            ORDER BY version
            LIMIT 1
            "#,
        )
        .ok()?;

    stmt.query_row(rusqlite::params![spec_id, current_version], |row| {
        row.get::<_, String>(0)
    })
    .ok()
}

/// Update spec YAML to mark milestone complete and advance to next
fn update_spec_milestone(spec_path: &str, current_version: &str, next_version: Option<&str>) -> Result<()> {
    let content = fs::read_to_string(spec_path)?;

    // Update the milestone status from in_progress to complete
    let pattern = format!(
        r#"(?m)(- version: "{}"[\s\S]*?status: )in_progress"#,
        regex::escape(current_version)
    );
    let re = regex::Regex::new(&pattern)?;
    let content = re.replace(&content, "${1}complete").to_string();

    // Update current_milestone to next version
    let content = if let Some(next) = next_version {
        // Also mark next milestone as in_progress
        let next_pattern = format!(
            r#"(?m)(- version: "{}"[\s\S]*?status: )pending"#,
            regex::escape(next)
        );
        let next_re = regex::Regex::new(&next_pattern)?;
        let content = next_re.replace(&content, "${1}in_progress").to_string();

        // Update current_milestone pointer
        let cm_re = regex::Regex::new(r#"(?m)^current_milestone: "[^"]+""#)?;
        cm_re.replace(&content, &format!(r#"current_milestone: "{}""#, next)).to_string()
    } else {
        content
    };

    fs::write(spec_path, content)?;
    Ok(())
}

/// Re-run layer scrape to sync index after spec update
fn rescrape_layer() -> Result<()> {
    use std::process::Command as ProcessCommand;

    let output = ProcessCommand::new("patina")
        .args(["scrape", "layer", "--full"])
        .output()
        .context("Failed to run patina scrape layer")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("patina scrape layer failed: {}", stderr);
    }

    Ok(())
}

/// Start a new development phase
pub fn bump_phase(name: &str, no_tag: bool, dry_run: bool) -> Result<()> {
    let mut state = load_or_create_state()?;
    let old_version = state.version.current.clone();

    // Calculate new version: increment phase, reset milestone to 0
    state.version.phase += 1;
    state.version.milestone = 0;
    state.version.phase_name = name.to_string();
    state.version.current = format!("0.{}.0", state.version.phase);

    let new_version = &state.version.current;

    if dry_run {
        println!("Dry run - would perform these changes:\n");
        println!("Version: {} -> {}", old_version, new_version);
        println!("New Phase: {} - {}", state.version.phase, name);
        println!("\nFiles that would be updated:");
        println!("  - .patina/version.toml");
        println!("  - Cargo.toml");
        if !no_tag {
            println!("  - git tag: v{}", new_version);
        }
        return Ok(());
    }

    // Update timestamp
    state.metadata.last_bump = Some(chrono::Utc::now().to_rfc3339());

    // Perform updates
    save_version_state(&state)?;
    update_cargo_version(new_version)?;

    if !no_tag {
        let tag_message = format!("Phase {}: {}", state.version.phase, name);
        create_git_tag(new_version, &tag_message)?;
    }

    // Output
    println!("Phase started: {} -> {}", old_version, new_version);
    println!("Phase {}: {}", state.version.phase, name);
    if !no_tag {
        println!("Tagged: v{}", new_version);
    }

    Ok(())
}

/// Initialize version tracking
pub fn init_version(phase: u32, phase_name: &str, milestone: u32) -> Result<()> {
    let version_path = Path::new(".patina/version.toml");

    if version_path.exists() {
        println!("Version tracking already initialized.");
        println!("Use 'patina version show' to see current state.");
        return Ok(());
    }

    // Read current version from Cargo.toml if present
    let current = read_cargo_version().unwrap_or_else(|_| "0.1.0".to_string());

    let state = VersionState {
        version: VersionInfo {
            current,
            phase,
            phase_name: phase_name.to_string(),
            milestone,
        },
        metadata: VersionMetadata {
            history_file: default_history_file(),
            last_bump: Some(chrono::Utc::now().to_rfc3339()),
        },
    };

    save_version_state(&state)?;

    println!("Version tracking initialized:");
    println!("  Version: {}", state.version.current);
    println!(
        "  Phase: {} ({})",
        state.version.phase, state.version.phase_name
    );
    println!("  Milestone: {}", state.version.milestone);

    Ok(())
}

// ============================================================================
// Output Helpers
// ============================================================================

fn output_json(components: bool) -> Result<()> {
    let state = load_or_create_state()?;

    let mut version_info = json!({
        "patina": CORE_VERSION,
        "phase": state.version.phase,
        "phase_name": state.version.phase_name,
        "milestone": state.version.milestone,
    });

    // Add current spec milestone from index (if available)
    if let Some(milestone) = get_current_spec_milestone() {
        version_info["spec_milestone"] = json!({
            "spec_id": milestone.spec_id,
            "version": milestone.version,
            "name": milestone.name,
            "status": milestone.status,
        });
    }

    if components {
        let components_info = get_component_versions()?;
        version_info["components"] = components_info;
    }

    println!("{}", serde_json::to_string_pretty(&version_info)?);
    Ok(())
}

fn output_human(components: bool) -> Result<()> {
    let state_result = load_version_state();

    // Always show core version
    println!("patina {CORE_VERSION}");

    // Show phase/milestone if version.toml exists
    if let Ok(state) = state_result {
        println!(
            "Phase {}: {} (milestone {})",
            state.version.phase, state.version.phase_name, state.version.milestone
        );
    }

    // Show current spec milestone from index (if available)
    if let Some(milestone) = get_current_spec_milestone() {
        let status_icon = match milestone.status.as_str() {
            "complete" => "✓",
            "in_progress" => "→",
            _ => "○",
        };
        println!(
            "Spec: {} v{} {} {}",
            milestone.spec_id, milestone.version, status_icon, milestone.name
        );
    }

    if components {
        println!("\nComponents:");
        let components_info = get_component_versions()?;

        // Display installed components from version manifest
        if let Some(installed) = components_info.get("installed").and_then(|v| v.as_object()) {
            for (name, info) in installed {
                if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
                    println!("  {name}: {version}");
                }
            }
        }

        // Display git info if available
        if let Some(git) = components_info.get("git").and_then(|v| v.as_object()) {
            if let Some(version) = git.get("version").and_then(|v| v.as_str()) {
                println!("  git: {version}");
                if let Some(commit) = git.get("commit").and_then(|v| v.as_str()) {
                    println!("    commit: {commit}");
                }
                if let Some(branch) = git.get("branch").and_then(|v| v.as_str()) {
                    println!("    branch: {branch}");
                }
            }
        }

        // Display external tools if detected
        if let Some(external) = components_info.get("external").and_then(|v| v.as_object()) {
            for (tool, version) in external {
                if let Some(v) = version.as_str() {
                    println!("  {tool}: {v} (external)");
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// State Management
// ============================================================================

fn load_version_state() -> Result<VersionState> {
    let path = Path::new(".patina/version.toml");
    let content = fs::read_to_string(path)
        .with_context(|| "Version tracking not initialized. Run 'patina version init' first.")?;
    let state: VersionState = toml::from_str(&content)?;
    Ok(state)
}

fn load_or_create_state() -> Result<VersionState> {
    match load_version_state() {
        Ok(state) => Ok(state),
        Err(_) => {
            // Create default state from Cargo.toml
            let current = read_cargo_version().unwrap_or_else(|_| CORE_VERSION.to_string());

            // Parse version to extract phase and milestone
            let parts: Vec<&str> = current.split('.').collect();
            let (phase, milestone) = if parts.len() >= 3 {
                let p = parts[1].parse().unwrap_or(1);
                let m = parts[2].parse().unwrap_or(0);
                (p, m)
            } else {
                (1, 0)
            };

            Ok(VersionState {
                version: VersionInfo {
                    current,
                    phase,
                    phase_name: "Unknown".to_string(),
                    milestone,
                },
                metadata: VersionMetadata::default(),
            })
        }
    }
}

fn save_version_state(state: &VersionState) -> Result<()> {
    let path = Path::new(".patina/version.toml");

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(state)?;
    fs::write(path, content)?;
    Ok(())
}

// ============================================================================
// Cargo.toml Management
// ============================================================================

fn read_cargo_version() -> Result<String> {
    let content = fs::read_to_string("Cargo.toml")?;

    // Simple parsing - find version = "x.y.z" line
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

fn update_cargo_version(new_version: &str) -> Result<()> {
    let path = Path::new("Cargo.toml");
    let content = fs::read_to_string(path)?;

    // Find and replace version line
    // Be careful to only replace the first version line (package version, not dependency versions)
    let mut in_package_section = false;
    let mut version_updated = false;
    let mut new_content = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Track if we're in [package] section
        if trimmed.starts_with('[') {
            in_package_section = trimmed == "[package]";
        }

        // Update version only in [package] section
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

// ============================================================================
// Git Operations
// ============================================================================

fn create_git_tag(version: &str, message: &str) -> Result<()> {
    let tag_name = format!("v{}", version);

    let output = Command::new("git")
        .args(["tag", "-a", &tag_name, "-m", message])
        .output()
        .context("Failed to run git tag")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git tag failed: {}", stderr);
    }

    Ok(())
}

// ============================================================================
// Component Versions (migrated from old version.rs)
// ============================================================================

fn get_git_info() -> Result<serde_json::Value> {
    let version = Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().replace("git version ", ""))
        .unwrap_or_else(|| "unknown".to_string());

    let mut info = json!({ "version": version });

    // Try to get current commit and branch if we're in a git repo
    if let Ok(commit) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if commit.status.success() {
            if let Ok(commit_str) = String::from_utf8(commit.stdout) {
                info["commit"] = json!(commit_str.trim());
            }
        }
    }

    if let Ok(branch) = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        if branch.status.success() {
            if let Ok(branch_str) = String::from_utf8(branch.stdout) {
                info["branch"] = json!(branch_str.trim());
            }
        }
    }

    Ok(info)
}

fn get_dagger_version() -> Result<String> {
    let output = Command::new("dagger").arg("version").output()?;

    if output.status.success() {
        let version_str = String::from_utf8(output.stdout)?;
        Ok(version_str
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .replace("dagger v", ""))
    } else {
        anyhow::bail!("Dagger not found")
    }
}

fn get_component_versions() -> Result<serde_json::Value> {
    let mut components = json!({
        "installed": {},
        "external": {}
    });

    // Try to load version manifest
    let manifest_path = Path::new(".patina/versions.json");
    if manifest_path.exists() {
        let content = fs::read_to_string(manifest_path)?;
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(tools) = manifest.get("components") {
                components["installed"] = tools.clone();
            }
        }
    }

    // Add git info
    if let Ok(git_info) = get_git_info() {
        components["git"] = git_info;
    }

    // Check for external tools
    let mut external = json!({});
    if let Ok(dagger_version) = get_dagger_version() {
        external["dagger"] = json!(dagger_version);
    }

    components["external"] = external;

    Ok(components)
}

// ============================================================================
// Milestone Queries (from scraped index)
// ============================================================================

/// Milestone info from the scraped spec index
#[derive(Debug, Clone)]
pub struct SpecMilestone {
    pub spec_id: String,
    pub version: String,
    pub name: String,
    pub status: String,
}

/// Get current milestone from scraped spec index
///
/// Looks for specs with current_milestone set and returns the matching milestone info.
fn get_current_spec_milestone() -> Option<SpecMilestone> {
    let db_path = Path::new(".patina/local/data/patina.db");
    if !db_path.exists() {
        return None;
    }

    let conn = Connection::open(db_path).ok()?;

    // Find patterns with current_milestone set and join with milestones table
    let mut stmt = conn
        .prepare(
            r#"
            SELECT m.spec_id, m.version, m.name, m.status
            FROM patterns p
            JOIN milestones m ON p.id = m.spec_id AND p.current_milestone = m.version
            WHERE p.current_milestone IS NOT NULL
            LIMIT 1
            "#,
        )
        .ok()?;

    stmt.query_row([], |row| {
        Ok(SpecMilestone {
            spec_id: row.get(0)?,
            version: row.get(1)?,
            name: row.get(2)?,
            status: row.get(3)?,
        })
    })
    .ok()
}

/// Get all milestones for a spec
#[allow(dead_code)]
fn get_spec_milestones(spec_id: &str) -> Vec<SpecMilestone> {
    let db_path = Path::new(".patina/local/data/patina.db");
    if !db_path.exists() {
        return Vec::new();
    }

    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match conn.prepare(
        "SELECT spec_id, version, name, status FROM milestones WHERE spec_id = ?1 ORDER BY version",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stmt.query_map([spec_id], |row| {
        Ok(SpecMilestone {
            spec_id: row.get(0)?,
            version: row.get(1)?,
            name: row.get(2)?,
            status: row.get(3)?,
        })
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}
