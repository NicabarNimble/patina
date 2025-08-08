use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub fn execute(bump: Option<&str>, dry_run: bool) -> Result<()> {
    println!("🚀 Preparing Patina release...");
    println!();

    // Run validation first
    println!("1️⃣ Running validation...");
    super::validate::execute(false)?;
    println!();

    // Run tests
    println!("2️⃣ Running tests...");
    let test_output = Command::new("cargo")
        .args(&["test", "--workspace", "--quiet"])
        .output()
        .context("Failed to run tests")?;

    if !test_output.status.success() {
        anyhow::bail!("Tests failed! Fix them before releasing.");
    }
    println!("   ✓ All tests passed");
    println!();

    // Check formatting
    println!("3️⃣ Checking formatting...");
    let fmt_output = Command::new("cargo")
        .args(&["fmt", "--", "--check"])
        .output()
        .context("Failed to check formatting")?;

    if !fmt_output.status.success() {
        anyhow::bail!("Code not formatted! Run 'cargo fmt' first.");
    }
    println!("   ✓ Code properly formatted");
    println!();

    // Run clippy
    println!("4️⃣ Running clippy...");
    let clippy_output = Command::new("cargo")
        .args(&["clippy", "--workspace", "--", "-D", "warnings"])
        .output()
        .context("Failed to run clippy")?;

    if !clippy_output.status.success() {
        anyhow::bail!("Clippy warnings found! Fix them before releasing.");
    }
    println!("   ✓ No clippy warnings");
    println!();

    // Check for uncommitted changes
    println!("5️⃣ Checking git status...");
    let git_output = Command::new("git")
        .args(&["status", "--porcelain"])
        .output()
        .context("Failed to check git status")?;

    if !git_output.stdout.is_empty() {
        anyhow::bail!("Uncommitted changes found! Commit or stash them first.");
    }
    println!("   ✓ Working tree clean");
    println!();

    // Handle version bump
    if let Some(bump_type) = bump {
        if dry_run {
            println!(
                "6️⃣ Would bump version ({}) - skipping due to --dry-run",
                bump_type
            );
        } else {
            println!("6️⃣ Bumping version ({})...", bump_type);
            bump_version(bump_type)?;
            println!("   ✓ Version bumped");
        }
        println!();
    }

    println!("✅ Ready for release!");
    println!();
    println!("Next steps:");
    if bump.is_none() {
        println!("1. Run with version bump: patina dev release [major|minor|patch]");
    } else if dry_run {
        println!("1. Run without --dry-run to actually bump version");
    } else {
        println!("1. Update CHANGELOG.md with release notes");
        println!(
            "2. Commit version bump: git commit -am \"Release v$(cargo pkgid | cut -d# -f2)\""
        );
        println!("3. Tag release: git tag v$(cargo pkgid | cut -d# -f2)");
        println!("4. Push to GitHub: git push && git push --tags");
        println!("5. Create GitHub release");
        println!("6. Publish to crates.io: cargo publish");
    }

    Ok(())
}

fn bump_version(bump_type: &str) -> Result<()> {
    // Read Cargo.toml
    let cargo_toml_path = "Cargo.toml";
    let content = fs::read_to_string(cargo_toml_path)?;

    // Find current version
    let version_line = content
        .lines()
        .find(|line| line.starts_with("version = "))
        .context("No version found in Cargo.toml")?;

    let current_version = version_line
        .split('"')
        .nth(1)
        .context("Invalid version format")?;

    // Parse version
    let parts: Vec<u32> = current_version
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    if parts.len() != 3 {
        anyhow::bail!("Invalid version format: {}", current_version);
    }

    // Calculate new version
    let (major, minor, patch) = (parts[0], parts[1], parts[2]);
    let new_version = match bump_type {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{}.{}.0", major, minor + 1),
        "patch" => format!("{}.{}.{}", major, minor, patch + 1),
        _ => anyhow::bail!("Invalid bump type: {}", bump_type),
    };

    // Replace version in Cargo.toml
    let new_content = content.replace(
        &format!("version = \"{}\"", current_version),
        &format!("version = \"{}\"", new_version),
    );

    fs::write(cargo_toml_path, new_content)?;

    // Update Cargo.lock
    Command::new("cargo")
        .args(&["update", "--workspace"])
        .output()
        .context("Failed to update Cargo.lock")?;

    println!("   {} → {}", current_version, new_version);

    Ok(())
}
