use anyhow::{Context, Result};
use std::fs;

pub fn execute(component: &str, bump_type: &str, dry_run: bool) -> Result<()> {
    println!("📦 Bumping {} version ({})...", component, bump_type);

    match component {
        "patina" => bump_patina_version(bump_type, dry_run)?,
        "claude-adapter" => bump_component_version("claude-adapter", bump_type, dry_run)?,
        "gemini-adapter" => bump_component_version("gemini-adapter", bump_type, dry_run)?,
        "openai-adapter" => bump_component_version("openai-adapter", bump_type, dry_run)?,
        "docker-templates" => bump_component_version("docker-templates", bump_type, dry_run)?,
        _ => anyhow::bail!("Unknown component: {}", component),
    }

    if !dry_run {
        println!();
        println!("✅ Version bumped successfully!");
        println!();
        println!("Next steps:");
        println!("1. Update CHANGELOG.md");
        println!("2. Run tests: cargo test");
        println!("3. Commit: git commit -am \"Bump {} version\"", component);
    }

    Ok(())
}

fn bump_patina_version(bump_type: &str, dry_run: bool) -> Result<()> {
    // This is the same logic from release.rs
    let cargo_toml_path = "Cargo.toml";
    let content = fs::read_to_string(cargo_toml_path)?;

    let version_line = content
        .lines()
        .find(|line| line.starts_with("version = "))
        .context("No version found in Cargo.toml")?;

    let current_version = version_line
        .split('"')
        .nth(1)
        .context("Invalid version format")?;

    let new_version = calculate_new_version(current_version, bump_type)?;

    println!("   Current: {}", current_version);
    println!("   New:     {}", new_version);

    if !dry_run {
        let new_content = content.replace(
            &format!("version = \"{}\"", current_version),
            &format!("version = \"{}\"", new_version),
        );
        fs::write(cargo_toml_path, new_content)?;
    }

    Ok(())
}

fn bump_component_version(component: &str, bump_type: &str, dry_run: bool) -> Result<()> {
    // Components are tracked in version manifests
    let manifest_path = ".patina/version_manifest.json";

    if !Path::new(manifest_path).exists() {
        // If no manifest, create one
        println!("   No version manifest found");
        return Ok(());
    }

    let content = fs::read_to_string(manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&content)?;

    let components = manifest
        .get_mut("components")
        .and_then(|c| c.as_object_mut())
        .context("Invalid manifest format")?;

    let current_version = components
        .get(component)
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0");

    let new_version = calculate_new_version(current_version, bump_type)?;

    println!("   Current: {}", current_version);
    println!("   New:     {}", new_version);

    if !dry_run {
        components[component] = serde_json::Value::String(new_version.clone());
        let new_content = serde_json::to_string_pretty(&manifest)?;
        fs::write(manifest_path, new_content)?;

        // Also update version constants in code
        update_version_in_code(component, &new_version)?;
    }

    Ok(())
}

fn calculate_new_version(current: &str, bump_type: &str) -> Result<String> {
    let parts: Vec<u32> = current.split('.').map(|s| s.parse().unwrap_or(0)).collect();

    if parts.len() != 3 {
        anyhow::bail!("Invalid version format: {}", current);
    }

    let (major, minor, patch) = (parts[0], parts[1], parts[2]);

    Ok(match bump_type {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{}.{}.0", major, minor + 1),
        "patch" => format!("{}.{}.{}", major, minor, patch + 1),
        _ => anyhow::bail!("Invalid bump type: {}", bump_type),
    })
}

fn update_version_in_code(component: &str, new_version: &str) -> Result<()> {
    // Update version constants in the appropriate files
    match component {
        "claude-adapter" => {
            let path = "src/adapters/claude.rs";
            if let Ok(content) = fs::read_to_string(path) {
                let new_content = content.replace(
                    "const CLAUDE_ADAPTER_VERSION: &str = ",
                    &format!(
                        "const CLAUDE_ADAPTER_VERSION: &str = \"{}\"; // ",
                        new_version
                    ),
                );
                if content != new_content {
                    fs::write(path, new_content)?;
                }
            }
        }
        // Add other components as needed
        _ => {}
    }

    Ok(())
}

use std::path::Path;
