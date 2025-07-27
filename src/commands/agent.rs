use anyhow::{Context, Result};
use std::env;
use std::path::Path;
use std::process::Command;

pub fn execute(command: Option<String>) -> Result<()> {
    // Check if we have a Dagger pipeline
    if !Path::new("pipelines/main.go").exists() {
        anyhow::bail!("No Dagger pipeline found. Run 'patina init' with --dev=dagger first.");
    }
    
    let subcommand = command.as_deref().unwrap_or("help");
    
    match subcommand {
        "test" => {
            // Patina decides what to test
            println!("🧪 Running tests in container...");
            run_in_dagger(&["test"])?;
        }
        
        "exec" => {
            // Allow arbitrary commands in container
            println!("🔧 Executing in container...");
            let args: Vec<String> = env::args().skip(3).collect();
            if args.is_empty() {
                anyhow::bail!("Usage: patina agent exec <command> [args...]");
            }
            let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_in_dagger_exec(&args_refs)?;
        }
        
        "improve" => {
            // Patina orchestrates improvement workflow
            println!("🤖 Running improvement workflow...");
            
            // Get component from args
            let component = env::args().nth(3)
                .ok_or_else(|| anyhow::anyhow!("Usage: patina agent improve <component>"))?;
            
            // Patina decides the steps:
            println!("1. Testing {} before changes...", component);
            run_in_dagger_exec(&["cargo", "test", "--package", &component])?;
            
            println!("2. Analyzing {} for improvements...", component);
            // Here Patina would coordinate with AI
            
            println!("3. Testing {} after changes...", component);
            run_in_dagger_exec(&["cargo", "test", "--package", &component])?;
            
            println!("✅ Improvement workflow complete");
        }
        
        "version" => {
            // Patina handles version logic
            println!("📦 Version management...");
            
            let component = env::args().nth(3)
                .ok_or_else(|| anyhow::anyhow!("Usage: patina agent version <component>"))?;
            
            // Patina reads current version
            let manifest = patina::version::VersionManifest::load(Path::new("."))?;
            let current = manifest.get_component_version(&component)
                .unwrap_or("0.0.0");
            
            println!("Current {} version: {}", component, current);
            
            // Patina decides new version
            // Then tells Dagger to run update commands
        }
        
        "explore" => {
            // Patina orchestrates exploration workflow
            println!("🔬 Starting exploration workflow...");
            
            let feature = env::args().nth(3)
                .ok_or_else(|| anyhow::anyhow!("Usage: patina agent explore <feature-name>"))?;
            
            // Start a basic container
            println!("Starting container...");
            std::thread::spawn(|| {
                let _ = run_in_dagger(&["container"]);
            });
            
            // Give container time to start
            std::thread::sleep(std::time::Duration::from_secs(2));
            
            // Now orchestrate the exploration setup
            println!("Setting up exploration environment...");
            
            // Configure git
            run_in_dagger_exec(&["git", "config", "--global", "user.email", "explorer@patina.dev"])?;
            run_in_dagger_exec(&["git", "config", "--global", "user.name", "Patina Explorer"])?;
            
            // Create feature branch
            println!("Creating feature branch: explore/{}...", feature);
            run_in_dagger_exec(&["git", "checkout", "-b", &format!("explore/{}", feature)])?;
            
            // Build to ensure everything works
            println!("Building project in container...");
            run_in_dagger_exec(&["cargo", "build"])?;
            
            println!("
✅ Exploration environment ready!");
            println!("   Feature: {}", feature);
            println!("   Branch: explore/{}", feature);
            println!("
💡 You can now:");
            println!("   - Edit files (they're mounted from host)");
            println!("   - Run: patina agent exec cargo test");
            println!("   - Run: patina agent exec cargo build");
            println!("   - Run: patina agent exec ./target/debug/patina --version");
        }
        
        "help" | _ => {
            println!("Available agent commands:");
            println!("  explore <name>    - Start exploration environment");
            println!("  test              - Run tests in container");
            println!("  exec <cmd>        - Execute command in container");
            println!("  improve <comp>    - Run improvement workflow");
            println!("  version <comp>    - Manage component versions");
        }
    }
    
    Ok(())
}

// Simple wrapper to run commands in Dagger
fn run_in_dagger(args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("go");
    cmd.current_dir("pipelines")
        .arg("run")
        .arg(".")
        .args(args);
    
    let status = cmd.status()
        .context("Failed to run Dagger pipeline")?;
    
    if !status.success() {
        anyhow::bail!("Dagger command failed");
    }
    
    Ok(())
}

// Run exec commands in Dagger
fn run_in_dagger_exec(exec_args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("go");
    cmd.current_dir("pipelines")
        .arg("run")
        .arg(".")
        .arg("exec")
        .args(exec_args);
    
    let status = cmd.status()
        .context("Failed to run Dagger pipeline")?;
    
    if !status.success() {
        anyhow::bail!("Dagger exec failed");
    }
    
    Ok(())
}