#!/usr/bin/env rustc

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    println!("\n🚀 Patina Bootstrap");
    println!("Setting up your development environment...\n");
    
    // Parse simple flags
    let minimal = args.contains(&"--minimal".to_string());
    let full = args.contains(&"--full".to_string());
    let dry_run = args.contains(&"--dry-run".to_string());
    
    // Detect system
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    println!("📋 System: {} {}", os, arch);
    
    // Define all available tools
    let all_tools = vec![
        // Essential
        ("rust", "rustc", "essential", "Rust compiler"),
        ("cargo", "cargo", "essential", "Rust package manager"),
        ("git", "git", "essential", "Version control"),
        // Recommended
        ("docker", "docker", "recommended", "Container runtime"),
        ("go", "go", "recommended", "Go language (for Dagger)"),
        ("dagger", "dagger", "recommended", "CI/CD pipelines"),
        // Optional
        ("make", "make", "optional", "Build automation"),
        ("jq", "jq", "optional", "JSON processing"),
    ];
    
    // Filter tools based on mode
    let tools: Vec<_> = if minimal {
        all_tools.into_iter()
            .filter(|(_, _, category, _)| *category == "essential")
            .collect()
    } else if full {
        all_tools
    } else {
        // Interactive selection
        println!("\n🔧 Tool Selection");
        println!("Essential tools will be installed. Choose additional tools:\n");
        
        let mut selected = all_tools.iter()
            .filter(|(_, _, category, _)| *category == "essential")
            .cloned()
            .collect::<Vec<_>>();
            
        print!("Install recommended tools (Docker, Go, Dagger)? [Y/n] ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if input.trim().is_empty() || input.trim().eq_ignore_ascii_case("y") {
            selected.extend(all_tools.iter()
                .filter(|(_, _, category, _)| *category == "recommended")
                .cloned());
        }
        
        selected
    };
    
    println!("\n🔍 Checking installed tools:");
    let mut to_install = Vec::new();
    
    for (name, cmd, category, description) in &tools {
        let installed = Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
            
        if installed {
            println!("   ✓ {} - {}", name, description);
        } else {
            println!("   ✗ {} - {} ({})", name, description, category);
            to_install.push((name, cmd, description));
        }
    }
    
    if to_install.is_empty() {
        println!("\n✅ All tools are installed!");
        return;
    }
    
    if dry_run {
        println!("\n--dry-run specified, would install:");
        for (name, _, _) in &to_install {
            println!("   - {}", name);
        }
        return;
    }
    
    // Confirm installation
    if !full {
        print!("\nInstall missing tools? [Y/n] ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!("Installation cancelled.");
            return;
        }
    }
    
    // Install tools
    for (name, _cmd, description) in to_install {
        println!("\n📦 Installing {} ({})...", name, description);
        
        let success = match *name {
            "docker" => install_docker(os),
            "go" => install_go(os),
            "dagger" => install_dagger(os),
            "make" => install_make(os),
            "jq" => install_jq(os),
            _ => {
                println!("   Don't know how to install {}", name);
                false
            }
        };
        
        if success {
            println!("   ✓ {} installed", name);
        } else {
            println!("   ✗ Failed to install {}", name);
        }
    }
    
    println!("\n🎯 Setup complete!");
    
    // Create PROJECT_DESIGN.toml
    create_project_design();
    
    println!("\nNext steps:");
    println!("1. Restart your shell");
    println!("2. Run: patina --version");
    println!("3. Initialize project: patina init <name> --llm=claude --design=PROJECT_DESIGN.toml");
}

fn install_docker(os: &str) -> bool {
    match os {
        "macos" => {
            Command::new("brew")
                .args(&["install", "--cask", "docker"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        "linux" => {
            Command::new("sh")
                .arg("-c")
                .arg("curl -fsSL https://get.docker.com | sh")
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn install_go(os: &str) -> bool {
    match os {
        "macos" => {
            Command::new("brew")
                .args(&["install", "go"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        _ => {
            println!("   Please install Go manually from https://go.dev");
            false
        }
    }
}

fn install_dagger(os: &str) -> bool {
    match os {
        "macos" => {
            // Try brew first (cleaner)
            if Command::new("brew")
                .args(&["install", "dagger/tap/dagger"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                true
            } else {
                // Fallback to official installer
                println!("   Brew failed, trying official installer...");
                Command::new("sh")
                    .arg("-c")
                    .arg("curl -fsSL https://dl.dagger.io/dagger/install.sh | sh")
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            }
        }
        "linux" => {
            Command::new("sh")
                .arg("-c")
                .arg("curl -fsSL https://dl.dagger.io/dagger/install.sh | sh")
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        _ => {
            println!("   Please install Dagger manually from https://dagger.io");
            false
        }
    }
}

fn install_make(os: &str) -> bool {
    match os {
        "macos" => {
            // Usually comes with Xcode Command Line Tools
            println!("   Make usually comes with Xcode Command Line Tools");
            true
        }
        "linux" => {
            Command::new("sh")
                .arg("-c")
                .arg("command -v apt-get && sudo apt-get install -y build-essential || command -v yum && sudo yum install -y make")
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn install_jq(os: &str) -> bool {
    match os {
        "macos" => {
            Command::new("brew")
                .args(&["install", "jq"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        "linux" => {
            Command::new("sh")
                .arg("-c")
                .arg("command -v apt-get && sudo apt-get install -y jq || command -v yum && sudo yum install -y jq")
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn create_project_design() {
    println!("\n📝 Setting up PROJECT_DESIGN.toml...");
    
    // Check if it already exists
    if Path::new("../PROJECT_DESIGN.toml").exists() {
        print!("PROJECT_DESIGN.toml already exists. Overwrite? [y/N] ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Keeping existing PROJECT_DESIGN.toml");
            return;
        }
    }
    
    // Ask about detail level
    println!("\nHow detailed should the PROJECT_DESIGN.toml be?");
    println!("1. Minimal (just basics)");
    println!("2. Standard (recommended)");
    println!("3. Comprehensive (all sections)");
    print!("\nChoice [2]: ");
    io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    let choice = choice.trim();
    let detail_level = if choice.is_empty() { "2" } else { choice };
    
    // Basic questions (always asked)
    print!("\nProject name [patina]: ");
    io::stdout().flush().unwrap();
    let mut project_name = String::new();
    io::stdin().read_line(&mut project_name).unwrap();
    let project_name = project_name.trim();
    let project_name = if project_name.is_empty() { "patina" } else { project_name };
    
    print!("Project type (tool/service/library/application) [tool]: ");
    io::stdout().flush().unwrap();
    let mut project_type = String::new();
    io::stdin().read_line(&mut project_type).unwrap();
    let project_type = project_type.trim();
    let project_type = if project_type.is_empty() { "tool" } else { project_type };
    
    print!("Project purpose (one line): ");
    io::stdout().flush().unwrap();
    let mut purpose = String::new();
    io::stdin().read_line(&mut purpose).unwrap();
    let purpose = purpose.trim();
    
    // Additional questions based on detail level
    let (problem, solution, users, value) = if detail_level != "1" {
        print!("\nWhat problem does this solve? ");
        io::stdout().flush().unwrap();
        let mut problem = String::new();
        io::stdin().read_line(&mut problem).unwrap();
        
        print!("How does it solve it? ");
        io::stdout().flush().unwrap();
        let mut solution = String::new();
        io::stdin().read_line(&mut solution).unwrap();
        
        print!("Who will use this? [developers]: ");
        io::stdout().flush().unwrap();
        let mut users = String::new();
        io::stdin().read_line(&mut users).unwrap();
        let users = users.trim();
        let users = if users.is_empty() { "developers" } else { users };
        
        print!("Core value proposition: ");
        io::stdout().flush().unwrap();
        let mut value = String::new();
        io::stdin().read_line(&mut value).unwrap();
        
        (problem.trim().to_string(), solution.trim().to_string(), users.to_string(), value.trim().to_string())
    } else {
        ("TODO: What problem does this solve?".to_string(),
         "TODO: How does it solve it?".to_string(),
         "developers".to_string(),
         "TODO: Core value proposition".to_string())
    };
    
    // Detect some project info
    let mut dependencies = Vec::new();
    let mut language = "rust".to_string();
    
    // Check for Cargo.toml to get dependencies
    if let Ok(content) = fs::read_to_string("../Cargo.toml") {
        if content.contains("[dependencies]") {
            // Simple extraction of dependency names
            let in_deps = content.split("[dependencies]").nth(1).unwrap_or("");
            for line in in_deps.lines() {
                if line.contains(" = ") && !line.trim().starts_with('#') {
                    if let Some(dep_name) = line.split(" = ").next() {
                        dependencies.push(dep_name.trim().to_string());
                        if dependencies.len() >= 5 { break; } // Just show first 5
                    }
                }
            }
        }
    } else if Path::new("../package.json").exists() {
        language = "javascript".to_string();
    } else if Path::new("../go.mod").exists() {
        language = "go".to_string();
    }
    
    // Create the TOML based on detail level
    let toml_content = match detail_level {
        "1" => {
            // Minimal version
            format!(r#"[project]
name = "{}"
type = "{}"
purpose = "{}"

[why]
problem = "{}"
solution = "{}"
users = "{}"
value = "{}"

[technical]
language = "{}"
"#, project_name, project_type, purpose, problem, solution, users, value, language)
        },
        "3" => {
            // Comprehensive version
            let deps_str = if dependencies.is_empty() {
                String::new()
            } else {
                format!("\n{}", dependencies.iter()
                    .map(|d| format!("    \"{}\",", d))
                    .collect::<Vec<_>>()
                    .join("\n"))
            };
            
            format!(r#"[project]
name = "{}"
type = "{}"
purpose = "{}"

[why]
problem = "{}"
solution = "{}"
users = "{}"
value = "{}"

[how]
patterns = [
    # TODO: Add architectural patterns (e.g., "MVC", "Event-driven", "Microservices")
]
architecture = "TODO: Describe high-level architecture"
core_abstractions = [
    # TODO: Add key abstractions/concepts
]

[what]
core_features = [
    # TODO: List main features
]
future_features = [
    # TODO: Planned enhancements
]
non_goals = [
    # TODO: What this project won't do
]

[technical]
language = "{}"
dependencies = [{}
]
constraints = [
    # TODO: Technical limitations or requirements
]
deployment = "TODO: How will this be deployed?"

[development]
[development.environment]
required_tools = ["rust", "cargo", "git"]
recommended_tools = ["docker", "dagger"]

[development.commands]
test = "cargo test"
build = "cargo build --release"
run = "cargo run"
lint = "cargo clippy"
format = "cargo fmt"

[development.conventions]
code_style = "rustfmt defaults"
commit_style = "conventional commits"
"#, project_name, project_type, purpose, problem, solution, users, value, language, deps_str)
        },
        _ => {
            // Standard version (default)
            let deps_str = if dependencies.is_empty() {
                String::new()
            } else {
                format!("\n{}", dependencies.iter()
                    .map(|d| format!("    \"{}\",", d))
                    .collect::<Vec<_>>()
                    .join("\n"))
            };
            
            format!(r#"[project]
name = "{}"
type = "{}"
purpose = "{}"

[why]
problem = "{}"
solution = "{}"
users = "{}"
value = "{}"

[how]
patterns = []
architecture = "TODO: High-level architecture"
core_abstractions = []

[what]
core_features = [
    # TODO: Add main features
]
future_features = []
non_goals = []

[technical]
language = "{}"
dependencies = [{}
]
constraints = []

[development]
[development.commands]
test = "cargo test"
build = "cargo build"
run = "cargo run"
"#, project_name, project_type, purpose, problem, solution, users, value, language, deps_str)
        }
    };

    // Write it to parent directory (project root)
    match fs::write("../PROJECT_DESIGN.toml", toml_content) {
        Ok(_) => {
            println!("\n✓ Created PROJECT_DESIGN.toml");
            println!("\nYou can edit this file to add more details about your project.");
            if detail_level != "3" {
                println!("Tip: Look for TODO comments to fill in missing sections.");
            }
        },
        Err(e) => println!("✗ Failed to create PROJECT_DESIGN.toml: {}", e),
    }
}