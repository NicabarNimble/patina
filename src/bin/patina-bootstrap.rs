#!/usr/bin/env rust-script
//! A comprehensive system bootstrap tool for Patina development
//! 
//! ```cargo
//! [dependencies]
//! anyhow = "1"
//! clap = { version = "4", features = ["derive"] }
//! reqwest = { version = "0.12", features = ["blocking"] }
//! indicatif = "0.17"
//! colored = "2"
//! which = "7"
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use which::which;

#[derive(Parser)]
#[command(name = "patina-bootstrap")]
#[command(about = "Bootstrap your system for Patina development", long_about = None)]
struct Cli {
    /// Skip confirmation prompts
    #[arg(short = 'y', long)]
    yes: bool,

    /// Only check what would be installed
    #[arg(long)]
    dry_run: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Skip optional tools
    #[arg(long)]
    minimal: bool,

    /// Also create a PROJECT_DESIGN.toml
    #[arg(long)]
    with_design: bool,
}

#[derive(Debug)]
struct SystemInfo {
    os: String,
    arch: String,
    distro: Option<String>,
    has_brew: bool,
    has_apt: bool,
    has_yum: bool,
    has_dnf: bool,
    shell: String,
    shell_rc: PathBuf,
}

#[derive(Debug)]
struct Tool {
    name: &'static str,
    check_cmd: &'static str,
    required: bool,
    installed: bool,
    version: Option<String>,
}

impl Tool {
    fn new(name: &'static str, check_cmd: &'static str, required: bool) -> Self {
        let installed = which(check_cmd).is_ok();
        let version = if installed {
            get_version(check_cmd)
        } else {
            None
        };
        
        Self {
            name,
            check_cmd,
            required,
            installed,
            version,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    println!("{}", "🚀 Patina Bootstrap System".bold().blue());
    println!("{}", "Setting up your development environment...\n".dimmed());
    
    // Detect system info
    let system = detect_system()?;
    display_system_info(&system);
    
    // Check existing tools
    let mut tools = check_installed_tools(&cli)?;
    display_tool_status(&tools);
    
    // Show what needs to be installed
    let to_install: Vec<&Tool> = tools.iter()
        .filter(|t| !t.installed && (t.required || !cli.minimal))
        .collect();
    
    if to_install.is_empty() {
        println!("\n{}", "✅ All tools are already installed!".green().bold());
        
        if cli.with_design {
            create_design_toml()?;
        }
        
        display_next_steps();
        return Ok(());
    }
    
    // Confirm installation
    if !cli.yes && !cli.dry_run {
        println!("\n{}", "📦 Tools to install:".bold());
        for tool in &to_install {
            println!("   - {} {}", 
                tool.name, 
                if tool.required { "(required)" } else { "(optional)" }.dimmed()
            );
        }
        
        print!("\nProceed with installation? [Y/n] ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!("Installation cancelled.");
            return Ok(());
        }
    }
    
    if cli.dry_run {
        println!("\n{}", "DRY RUN - No changes will be made".yellow().bold());
        return Ok(());
    }
    
    // Install tools
    for tool in to_install {
        install_tool(tool, &system, &cli)?;
    }
    
    // Re-check tools
    tools = check_installed_tools(&cli)?;
    
    // Update shell configuration
    update_shell_config(&system)?;
    
    // Create design TOML if requested
    if cli.with_design {
        create_design_toml()?;
    }
    
    // Display summary
    display_summary(&tools);
    display_next_steps();
    
    Ok(())
}

fn detect_system() -> Result<SystemInfo> {
    let os = env::consts::OS.to_string();
    let arch = env::consts::ARCH.to_string();
    
    let distro = if os == "linux" {
        detect_linux_distro()
    } else {
        None
    };
    
    let has_brew = which("brew").is_ok();
    let has_apt = which("apt-get").is_ok();
    let has_yum = which("yum").is_ok();
    let has_dnf = which("dnf").is_ok();
    
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("bash");
    
    let shell_rc = match shell_name {
        "zsh" => dirs::home_dir().unwrap().join(".zshrc"),
        "bash" => dirs::home_dir().unwrap().join(".bashrc"),
        "fish" => dirs::config_dir().unwrap().join("fish/config.fish"),
        _ => dirs::home_dir().unwrap().join(".profile"),
    };
    
    Ok(SystemInfo {
        os,
        arch,
        distro,
        has_brew,
        has_apt,
        has_yum,
        has_dnf,
        shell: shell_name.to_string(),
        shell_rc,
    })
}

fn detect_linux_distro() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("ID=") {
                return Some(line.trim_start_matches("ID=").trim_matches('"').to_string());
            }
        }
    }
    None
}

fn display_system_info(system: &SystemInfo) {
    println!("{}", "📋 System Information".bold());
    println!("   OS: {} {}", system.os, system.arch);
    if let Some(ref distro) = system.distro {
        println!("   Distribution: {}", distro);
    }
    println!("   Shell: {}", system.shell);
    println!("   Package managers: {}", 
        [
            system.has_brew.then(|| "brew"),
            system.has_apt.then(|| "apt"),
            system.has_yum.then(|| "yum"),
            system.has_dnf.then(|| "dnf"),
        ]
        .iter()
        .filter_map(|&x| x)
        .collect::<Vec<_>>()
        .join(", ")
    );
    println!();
}

fn check_installed_tools(cli: &Cli) -> Result<Vec<Tool>> {
    let mut tools = vec![
        Tool::new("rust", "rustc", true),
        Tool::new("cargo", "cargo", true),
        Tool::new("git", "git", true),
        Tool::new("docker", "docker", !cli.minimal),
        Tool::new("go", "go", !cli.minimal),
        Tool::new("dagger", "dagger", !cli.minimal),
        Tool::new("make", "make", false),
        Tool::new("curl", "curl", true),
    ];
    
    // Check for rustup specifically
    if !which("rustup").is_ok() && tools[0].installed {
        // Rust is installed but not through rustup
        tools[0].version = Some("installed without rustup".to_string());
    }
    
    Ok(tools)
}

fn get_version(cmd: &str) -> Option<String> {
    let version_flag = match cmd {
        "rustc" => "--version",
        "go" => "version",
        "dagger" => "version",
        _ => "--version",
    };
    
    Command::new(cmd)
        .arg(version_flag)
        .output()
        .ok()
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.lines().next().unwrap_or("").to_string())
        })
}

fn display_tool_status(tools: &[Tool]) {
    println!("{}", "🔧 Tool Status".bold());
    
    for tool in tools {
        let status = if tool.installed {
            format!("{} {}", 
                "✓".green().bold(),
                tool.version.as_ref().unwrap_or(&"installed".to_string()).dimmed()
            )
        } else {
            format!("{} not installed", "✗".red().bold())
        };
        
        println!("   {:<10} {}", tool.name, status);
    }
}

fn install_tool(tool: &Tool, system: &SystemInfo, cli: &Cli) -> Result<()> {
    println!("\n{} Installing {}...", "📦".blue(), tool.name.bold());
    
    let pb = if !cli.verbose {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        pb.set_message(format!("Installing {}", tool.name));
        Some(pb)
    } else {
        None
    };
    
    let result = match tool.name {
        "rust" | "cargo" => install_rust(system, cli.verbose),
        "docker" => install_docker(system, cli.verbose),
        "go" => install_go(system, cli.verbose),
        "dagger" => install_dagger(system, cli.verbose),
        "git" => install_git(system, cli.verbose),
        "make" => install_make(system, cli.verbose),
        "curl" => install_curl(system, cli.verbose),
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool.name)),
    };
    
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }
    
    match result {
        Ok(_) => println!("   {} {} installed successfully", "✓".green().bold(), tool.name),
        Err(e) => println!("   {} Failed to install {}: {}", "✗".red().bold(), tool.name, e),
    }
    
    result
}

fn install_rust(system: &SystemInfo, verbose: bool) -> Result<()> {
    // Install rustup
    let install_cmd = if system.os == "macos" || system.os == "linux" {
        "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
    } else {
        return Err(anyhow::anyhow!("Unsupported OS for Rust installation"));
    };
    
    run_shell_command(install_cmd, verbose)?;
    
    // Add to PATH for current session
    let cargo_bin = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".cargo/bin");
    
    env::set_var("PATH", format!("{}:{}", cargo_bin.display(), env::var("PATH")?));
    
    Ok(())
}

fn install_docker(system: &SystemInfo, verbose: bool) -> Result<()> {
    match system.os.as_str() {
        "macos" => {
            if system.has_brew {
                run_command("brew", &["install", "--cask", "docker"], verbose)?;
            } else {
                return Err(anyhow::anyhow!("Docker Desktop installation requires Homebrew on macOS"));
            }
        }
        "linux" => {
            // Docker installation script
            run_shell_command(
                "curl -fsSL https://get.docker.com | sh",
                verbose
            )?;
            
            // Add user to docker group
            if let Ok(user) = env::var("USER") {
                run_command("sudo", &["usermod", "-aG", "docker", &user], verbose)?;
                println!("   {} Added user to docker group (logout required)", "ℹ️".blue());
            }
        }
        _ => return Err(anyhow::anyhow!("Unsupported OS for Docker installation")),
    }
    
    Ok(())
}

fn install_go(system: &SystemInfo, verbose: bool) -> Result<()> {
    match system.os.as_str() {
        "macos" => {
            if system.has_brew {
                run_command("brew", &["install", "go"], verbose)?;
            } else {
                install_go_binary(system, verbose)?;
            }
        }
        "linux" => {
            if system.has_apt {
                run_command("sudo", &["apt-get", "update"], verbose)?;
                run_command("sudo", &["apt-get", "install", "-y", "golang"], verbose)?;
            } else if system.has_dnf {
                run_command("sudo", &["dnf", "install", "-y", "golang"], verbose)?;
            } else if system.has_yum {
                run_command("sudo", &["yum", "install", "-y", "golang"], verbose)?;
            } else {
                install_go_binary(system, verbose)?;
            }
        }
        _ => return Err(anyhow::anyhow!("Unsupported OS for Go installation")),
    }
    
    Ok(())
}

fn install_go_binary(system: &SystemInfo, verbose: bool) -> Result<()> {
    // Download and install Go binary
    let go_version = "1.23.0";
    let os = if system.os == "macos" { "darwin" } else { "linux" };
    let arch = if system.arch == "x86_64" { "amd64" } else { "arm64" };
    
    let url = format!(
        "https://go.dev/dl/go{}.{}-{}.tar.gz",
        go_version, os, arch
    );
    
    run_shell_command(
        &format!(
            "curl -L {} | sudo tar -C /usr/local -xzf -",
            url
        ),
        verbose
    )?;
    
    Ok(())
}

fn install_dagger(system: &SystemInfo, verbose: bool) -> Result<()> {
    // Dagger requires Go to be installed first
    if !which("go").is_ok() {
        return Err(anyhow::anyhow!("Dagger requires Go to be installed first"));
    }
    
    // Install using official script
    run_shell_command(
        "curl -fsSL https://dl.dagger.io/dagger/install.sh | sh",
        verbose
    )?;
    
    Ok(())
}

fn install_git(system: &SystemInfo, verbose: bool) -> Result<()> {
    match system.os.as_str() {
        "macos" => {
            if system.has_brew {
                run_command("brew", &["install", "git"], verbose)?;
            } else {
                // Git comes with Xcode Command Line Tools
                run_command("xcode-select", &["--install"], verbose)?;
            }
        }
        "linux" => {
            if system.has_apt {
                run_command("sudo", &["apt-get", "update"], verbose)?;
                run_command("sudo", &["apt-get", "install", "-y", "git"], verbose)?;
            } else if system.has_dnf {
                run_command("sudo", &["dnf", "install", "-y", "git"], verbose)?;
            } else if system.has_yum {
                run_command("sudo", &["yum", "install", "-y", "git"], verbose)?;
            }
        }
        _ => return Err(anyhow::anyhow!("Unsupported OS for Git installation")),
    }
    
    Ok(())
}

fn install_make(system: &SystemInfo, verbose: bool) -> Result<()> {
    match system.os.as_str() {
        "macos" => {
            // Make comes with Xcode Command Line Tools
            run_command("xcode-select", &["--install"], verbose)?;
        }
        "linux" => {
            if system.has_apt {
                run_command("sudo", &["apt-get", "install", "-y", "build-essential"], verbose)?;
            } else if system.has_dnf {
                run_command("sudo", &["dnf", "install", "-y", "make"], verbose)?;
            } else if system.has_yum {
                run_command("sudo", &["yum", "install", "-y", "make"], verbose)?;
            }
        }
        _ => return Err(anyhow::anyhow!("Unsupported OS for Make installation")),
    }
    
    Ok(())
}

fn install_curl(system: &SystemInfo, verbose: bool) -> Result<()> {
    match system.os.as_str() {
        "macos" => {
            // curl comes with macOS
            Ok(())
        }
        "linux" => {
            if system.has_apt {
                run_command("sudo", &["apt-get", "install", "-y", "curl"], verbose)?;
            } else if system.has_dnf {
                run_command("sudo", &["dnf", "install", "-y", "curl"], verbose)?;
            } else if system.has_yum {
                run_command("sudo", &["yum", "install", "-y", "curl"], verbose)?;
            }
            Ok(())
        }
        _ => Err(anyhow::anyhow!("Unsupported OS for curl installation")),
    }
}

fn run_command(cmd: &str, args: &[&str], verbose: bool) -> Result<()> {
    let mut command = Command::new(cmd);
    command.args(args);
    
    if !verbose {
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
    }
    
    let status = command.status()
        .with_context(|| format!("Failed to run command: {} {}", cmd, args.join(" ")))?;
    
    if !status.success() {
        return Err(anyhow::anyhow!("Command failed: {} {}", cmd, args.join(" ")));
    }
    
    Ok(())
}

fn run_shell_command(cmd: &str, verbose: bool) -> Result<()> {
    let mut command = Command::new("sh");
    command.args(&["-c", cmd]);
    
    if !verbose {
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
    }
    
    let status = command.status()
        .with_context(|| format!("Failed to run shell command: {}", cmd))?;
    
    if !status.success() {
        return Err(anyhow::anyhow!("Shell command failed: {}", cmd));
    }
    
    Ok(())
}

fn update_shell_config(system: &SystemInfo) -> Result<()> {
    let mut additions = Vec::new();
    
    // Add Rust/Cargo to PATH
    let cargo_path = r#"export PATH="$HOME/.cargo/bin:$PATH""#;
    if !shell_config_contains(&system.shell_rc, cargo_path)? {
        additions.push(cargo_path);
    }
    
    // Add Go to PATH if installed via binary
    if Path::new("/usr/local/go").exists() {
        let go_path = r#"export PATH="/usr/local/go/bin:$PATH""#;
        if !shell_config_contains(&system.shell_rc, go_path)? {
            additions.push(go_path);
        }
    }
    
    // Add Dagger to PATH
    let dagger_path = r#"export PATH="$HOME/.dagger/bin:$PATH""#;
    if Path::new(&dirs::home_dir().unwrap().join(".dagger/bin")).exists() {
        if !shell_config_contains(&system.shell_rc, dagger_path)? {
            additions.push(dagger_path);
        }
    }
    
    if !additions.is_empty() {
        println!("\n{} Updating shell configuration...", "🔧".blue());
        
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&system.shell_rc)?;
        
        writeln!(file, "\n# Added by patina-bootstrap")?;
        for addition in additions {
            writeln!(file, "{}", addition)?;
            println!("   Added: {}", addition.dimmed());
        }
        
        println!("   {} Please restart your shell or run: source {}", 
            "ℹ️".blue(), 
            system.shell_rc.display()
        );
    }
    
    Ok(())
}

fn shell_config_contains(path: &Path, text: &str) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    
    let content = fs::read_to_string(path)?;
    Ok(content.contains(text))
}

fn create_design_toml() -> Result<()> {
    // Use the design module if available, otherwise create a simple one
    if Path::new("src/commands/design.rs").exists() {
        println!("\n{} Run 'patina design' for interactive PROJECT_DESIGN.toml creation", "💡".blue());
    } else {
        println!("\n{} Creating PROJECT_DESIGN.toml...", "📝".blue());
        
        // Simple interactive design creation
        print!("Project name: ");
        io::stdout().flush()?;
        let mut project_name = String::new();
        io::stdin().read_line(&mut project_name)?;
        
        print!("Project purpose (one line): ");
        io::stdout().flush()?;
        let mut purpose = String::new();
        io::stdin().read_line(&mut purpose)?;
        
        let design = format!(r#"[project]
name = "{}"
type = "application"
purpose = "{}"

[why]
problem = "TODO: What problem does this solve?"
solution = "TODO: How does it solve it?"
users = "developers"
value = "TODO: Core value proposition"

[how]
patterns = []
architecture = "TODO: High-level architecture"
core_abstractions = []

[what]
core_features = []
future_features = []
non_goals = []

[technical]
language = "rust"
dependencies = []
constraints = []

[development]
[development.commands]
test = "cargo test"
build = "cargo build"
run = "cargo run"
"#, 
            project_name.trim(),
            purpose.trim()
        );
        
        fs::write("PROJECT_DESIGN.toml", design)?;
        println!("   {} Created PROJECT_DESIGN.toml", "✓".green().bold());
    }
    
    Ok(())
}

fn display_summary(tools: &[Tool]) {
    println!("\n{}", "📊 Installation Summary".bold().blue());
    
    let installed = tools.iter().filter(|t| t.installed).count();
    let total = tools.len();
    
    println!("   Total tools: {}", total);
    println!("   Installed: {} {}", 
        installed,
        if installed == total { "✨" } else { "" }
    );
    
    if installed < total {
        println!("\n   {} Some tools could not be installed:", "⚠️".yellow());
        for tool in tools.iter().filter(|t| !t.installed) {
            println!("   - {}", tool.name);
        }
    }
}

fn display_next_steps() {
    println!("\n{}", "🎯 Next Steps".bold().green());
    println!("   1. Restart your shell or run: source ~/.bashrc (or ~/.zshrc)");
    println!("   2. Install Patina: cargo install patina");
    println!("   3. Initialize a project: patina init <name> --llm=claude --design=PROJECT_DESIGN.toml");
    println!("\n{}", "Happy coding! 🚀".bold());
}

// For Cargo.toml:
// [[bin]]
// name = "patina-bootstrap"
// path = "src/bin/patina-bootstrap.rs"
//
// [dependencies]
// indicatif = "0.17"
// colored = "2"