//! YOLO Command - Generate devcontainers for autonomous AI development
//!
//! This command scans a repository and generates a complete devcontainer
//! environment optimized for AI assistants to work autonomously.

use anyhow::Result;
use std::path::Path;

mod features;
mod generator;
mod profile;
mod scanner;

use features::{DevContainerFeature, FeatureMapper};
use generator::Generator;
use profile::RepoProfile;
use scanner::Scanner;

/// Main entry point for the yolo command
pub fn execute(
    interactive: bool,
    defaults: bool,
    with: Option<Vec<String>>,
    without: Option<Vec<String>>,
    json: bool,
) -> Result<()> {
    println!("🎯 YOLO Mode: Scanning for autonomous workspace setup...");

    // Get current directory
    let work_dir = std::env::current_dir()?;

    // Step 1: Scan the repository
    let scanner = Scanner::new(&work_dir);
    let mut profile = scanner.scan()?;

    // Apply --with and --without overrides
    if let Some(with_tools) = with {
        for tool in with_tools {
            profile.add_tool_override(&tool);
        }
    }

    if let Some(without_tools) = without {
        for tool in without_tools {
            profile.exclude_tool(&tool);
        }
    }

    // Display detected stack
    if !json {
        display_detection_results(&profile);
    }

    // Step 2: Interactive mode if requested
    if interactive && !defaults {
        profile = run_interactive_mode(profile)?;
    }

    // Step 3: Map to Dev Container Features
    let mapper = FeatureMapper::new();
    let features = mapper.map_profile(&profile)?;

    // Step 4: Generate devcontainer files
    let generator = Generator::new(&work_dir);
    generator.generate(&profile, &features)?;

    // Output results
    if json {
        output_json_results(&profile, &features)?;
    } else {
        display_success_message(&work_dir);
    }

    Ok(())
}

fn display_detection_results(profile: &RepoProfile) {
    // Display languages
    for (lang, info) in &profile.languages {
        if let Some(version) = &info.version {
            println!("  ✓ Found {} ({})", lang, version);
        } else {
            println!("  ✓ Found {}", lang);
        }
    }

    // Display tools
    for (tool, info) in &profile.tools {
        if let Some(version) = &info.version {
            println!("  ✓ Found {} ({})", tool, version);
        } else {
            println!("  ✓ Found {}", tool);
        }
    }

    // Display services
    for service in &profile.services {
        println!("  ✓ Found service: {}", service.name);
    }
}

fn run_interactive_mode(profile: RepoProfile) -> Result<RepoProfile> {
    // TODO: Implement interactive selection
    println!("\n📝 Interactive mode not yet implemented, using defaults...");
    Ok(profile)
}

fn output_json_results(profile: &RepoProfile, features: &[DevContainerFeature]) -> Result<()> {
    // TODO: Implement JSON output
    let result = serde_json::json!({
        "profile": profile,
        "features": features,
        "status": "success"
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn display_success_message(work_dir: &Path) {
    println!("\n🤖 Generating YOLO workspace for AI autonomy...");
    println!("  ✓ Created .devcontainer/devcontainer.json");
    println!("  ✓ Created .devcontainer/Dockerfile");
    println!("  ✓ Created .devcontainer/docker-compose.yml");

    println!("\n💭 AI-Ready Features:");
    println!("  • Permissions bypass for autonomous work");
    println!("  • All detected toolchains installed");
    println!("  • Git worktree isolation configured");

    println!("\n🚀 Launch: docker compose -f .devcontainer/docker-compose.yml up -d");
    println!(
        "   Then: docker exec -it {}-yolo bash",
        work_dir.file_name().unwrap().to_string_lossy()
    );
    println!("   \n⚡ For Claude Code: claude --dangerously-skip-permissions");
}
