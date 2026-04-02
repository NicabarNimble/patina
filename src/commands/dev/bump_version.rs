use anyhow::{Context, Result};

use crate::release::internal::{compute_next_version, read_cargo_version, update_cargo_version};
use crate::release::BumpType;

pub fn execute(component: &str, bump_type: &str, dry_run: bool) -> Result<()> {
    println!("📦 Bumping {} version ({})...", component, bump_type);

    match component {
        "patina" => bump_patina_version(bump_type, dry_run)?,
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
    let bump = parse_bump_type(bump_type)?;
    let current_version = read_cargo_version()?;
    let new_version = compute_next_version(&current_version, bump)?;

    println!("   Current: {}", current_version);
    println!("   New:     {}", new_version);

    if !dry_run {
        update_cargo_version(&new_version)?;
    }

    Ok(())
}

fn parse_bump_type(bump_type: &str) -> Result<BumpType> {
    match bump_type {
        "major" => Ok(BumpType::Major),
        "minor" => Ok(BumpType::Minor),
        "patch" => Ok(BumpType::Patch),
        _ => anyhow::bail!("Invalid bump type: {}", bump_type),
    }
}
