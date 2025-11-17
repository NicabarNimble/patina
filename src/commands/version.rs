use anyhow::Result;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn execute(json: bool, components: bool) -> Result<()> {
    if json {
        output_json(components)?;
    } else {
        output_human(components)?;
    }
    Ok(())
}

fn output_json(components: bool) -> Result<()> {
    let mut version_info = json!({
        "patina": CORE_VERSION,
    });

    if components {
        let components_info = get_component_versions()?;
        version_info["components"] = components_info;
    }

    println!("{}", serde_json::to_string_pretty(&version_info)?);
    Ok(())
}

fn output_human(components: bool) -> Result<()> {
    println!("patina {CORE_VERSION}");

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
        // Extract just the version number from "dagger vX.Y.Z"
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

    // Could add more external tools here (docker, etc)
    components["external"] = external;

    Ok(components)
}
