//! Internal implementation for version command
//!
//! After version-consolidation: show + hotfix only.
//! All version bump logic lives in patina::release.

use anyhow::Result;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

use patina::release::{BumpType, ReleaseStrategy};

const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// Emergency patch bump without spec ceremony.
///
/// Uses ReleaseStrategy::preflight/execute with BumpType::Patch.
/// Same safeguards as spec-driven releases. Cargo strategy only.
pub fn hotfix(description: &str) -> Result<()> {
    let strategy = ReleaseStrategy::from_project(Path::new("."));

    if strategy != ReleaseStrategy::Cargo {
        anyhow::bail!(
            "Hotfix is only available for Cargo strategy projects.\n\
             Your project uses {:?} strategy.\n\
             Manage your emergency patches manually.",
            strategy
        );
    }

    let prepared = strategy.preflight(BumpType::Patch, "Cargo.toml")?;
    prepared.execute(description, "Cargo.toml", None)?;

    println!("\n  Consider creating a spec for traceability:");
    println!("    patina spec complete <id>");

    println!("\n  Rebuild to use new version:");
    println!("    cargo build --release && cargo install --path .");

    Ok(())
}

// ============================================================================
// Output Helpers
// ============================================================================

fn output_json(components: bool) -> Result<()> {
    let mut version_info = json!({
        "version": CORE_VERSION,
        "strategy": format!("{:?}", ReleaseStrategy::from_project(Path::new("."))),
    });

    // Show ready specs instead of milestones
    if let Ok(ready) = get_ready_spec_ids() {
        if !ready.is_empty() {
            version_info["ready"] = json!(ready);
        }
    }

    if components {
        let components_info = get_component_versions()?;
        version_info["components"] = components_info;
    }

    println!("{}", serde_json::to_string_pretty(&version_info)?);
    Ok(())
}

fn output_human(components: bool) -> Result<()> {
    println!("patina {CORE_VERSION}");

    // Show strategy
    let strategy = ReleaseStrategy::from_project(Path::new("."));
    match strategy {
        ReleaseStrategy::Cargo => {} // Default, no noise
        ReleaseStrategy::External => println!("  Strategy: external (advisory)"),
        ReleaseStrategy::None => println!("  Strategy: none (spec-only)"),
    }

    // Show ready specs instead of milestones
    match get_ready_spec_ids() {
        Ok(ready) if !ready.is_empty() => {
            println!("Ready: {}", ready.join(", "));
        }
        Ok(_) => {} // No ready specs, no noise
        Err(_) => {
            eprintln!("  (no index - run 'patina scrape layer')");
        }
    }

    if components {
        println!("\nComponents:");
        let components_info = get_component_versions()?;

        if let Some(installed) = components_info.get("installed").and_then(|v| v.as_object()) {
            for (name, info) in installed {
                if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
                    println!("  {name}: {version}");
                }
            }
        }

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
// Ready Spec Query
// ============================================================================

/// Get IDs of specs ready to work on (simple query for version show)
fn get_ready_spec_ids() -> Result<Vec<String>> {
    let db_path = Path::new(".patina/local/data/patina.db");
    if !db_path.exists() {
        anyhow::bail!("No index");
    }

    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        r#"
        SELECT p.id
        FROM patterns p
        WHERE p.file_path LIKE 'layer/surface/build/%'
          AND p.status IN ('ready', 'active')
          AND NOT EXISTS (
            SELECT 1 FROM spec_deps d
            JOIN patterns blocker ON d.depends_on = blocker.id
            WHERE d.spec_id = p.id
              AND blocker.status NOT IN ('complete', 'done')
          )
        ORDER BY p.id
        "#,
    )?;

    let ids: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ids)
}

// ============================================================================
// Component Versions
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

    let manifest_path = Path::new(".patina/versions.json");
    if manifest_path.exists() {
        let content = fs::read_to_string(manifest_path)?;
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(tools) = manifest.get("components") {
                components["installed"] = tools.clone();
            }
        }
    }

    if let Ok(git_info) = get_git_info() {
        components["git"] = git_info;
    }

    let mut external = json!({});
    if let Ok(dagger_version) = get_dagger_version() {
        external["dagger"] = json!(dagger_version);
    }

    components["external"] = external;

    Ok(components)
}

#[cfg(test)]
mod tests {
    use patina::spec::{parse_spec_file, serialize_spec_file, Sessions};

    #[test]
    fn test_spec_frontmatter_parse_roundtrip() {
        let yaml = r#"---
type: feat
id: v1-release
status: in_progress
created: 2026-01-27
updated: 2026-01-29
sessions:
  origin: 20260127-085434
  work: [20260129-074742]
related:
  - spec/go-public
  - spec-epistemic-layer
milestones:
  - version: "0.9.1"
    name: Version & spec system alignment
    status: in_progress
  - version: "0.9.2"
    name: Epistemic E4
    status: pending
current_milestone: "0.9.1"
---

# feat: v1.0 Release

Body content here.
"#;

        let (frontmatter, body) = parse_spec_file(yaml).expect("should parse");

        assert_eq!(frontmatter.id, "v1-release");
        assert_eq!(frontmatter.r#type, "feat");
        assert_eq!(frontmatter.milestones.len(), 2);
        assert_eq!(frontmatter.milestones[0].version, "0.9.1");
        assert_eq!(frontmatter.milestones[0].status, "in_progress");
        assert_eq!(frontmatter.current_milestone, Some("0.9.1".to_string()));
        assert!(body.contains("# feat: v1.0 Release"));

        let output = serialize_spec_file(&frontmatter, &body).expect("should serialize");

        let (fm2, body2) = parse_spec_file(&output).expect("should parse again");
        assert_eq!(fm2.id, frontmatter.id);
        assert_eq!(fm2.milestones.len(), frontmatter.milestones.len());
        assert_eq!(body2.trim(), body.trim());
    }

    #[test]
    fn test_sessions_list_format() {
        let yaml = r#"---
type: refactor
id: test-spec
status: in_progress
sessions: [20260108-200725, 20260109-063849]
---

# Test
"#;

        let (frontmatter, _) = parse_spec_file(yaml).expect("should parse list format");
        match frontmatter.sessions {
            Some(Sessions::List(list)) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], "20260108-200725");
            }
            _ => panic!("Expected Sessions::List"),
        }
    }

    #[test]
    fn test_sessions_structured_format() {
        let yaml = r#"---
type: feat
id: test-spec
status: in_progress
sessions:
  origin: 20260127-085434
  work: [20260129-074742]
---

# Test
"#;

        let (frontmatter, _) = parse_spec_file(yaml).expect("should parse structured format");
        match frontmatter.sessions {
            Some(Sessions::Structured { origin, work, .. }) => {
                assert_eq!(origin, Some("20260127-085434".to_string()));
                assert_eq!(work.len(), 1);
            }
            _ => panic!("Expected Sessions::Structured"),
        }
    }
}
