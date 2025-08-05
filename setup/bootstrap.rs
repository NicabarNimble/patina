#!/usr/bin/env rustc

use std::env;
use std::io::{self, Write};
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
    
    // Check installed tools
    let tools = vec![
        ("rust", "rustc", true),
        ("cargo", "cargo", true),
        ("git", "git", true),
        ("docker", "docker", !minimal),
        ("go", "go", !minimal),
        ("dagger", "dagger", !minimal),
    ];
    
    println!("\n🔧 Checking tools:");
    let mut to_install = Vec::new();
    
    for (name, cmd, needed) in &tools {
        if !needed {
            continue;
        }
        
        let installed = Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
            
        if installed {
            println!("   ✓ {}", name);
        } else {
            println!("   ✗ {}", name);
            to_install.push((name, cmd));
        }
    }
    
    if to_install.is_empty() {
        println!("\n✅ All tools are installed!");
        return;
    }
    
    if dry_run {
        println!("\n--dry-run specified, would install:");
        for (name, _) in &to_install {
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
    for (name, cmd) in to_install {
        println!("\n📦 Installing {}...", name);
        
        let success = match *name {
            "docker" => install_docker(os),
            "go" => install_go(os),
            "dagger" => install_dagger(),
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
    println!("\nNext steps:");
    println!("1. Restart your shell");
    println!("2. Run: patina --version");
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

fn install_dagger() -> bool {
    Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://dl.dagger.io/dagger/install.sh | sh")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}