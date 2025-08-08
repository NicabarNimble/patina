use anyhow::{Context, Result};
use std::process::Command;

/// Tool information for installation
#[derive(Debug)]
pub struct Tool {
    pub name: &'static str,
    pub check_cmd: &'static str,
    #[allow(dead_code)]
    pub required: bool,
}

/// Get all available tools for the current platform
pub fn get_available_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "docker",
            check_cmd: "docker",
            required: false,
        },
        Tool {
            name: "go",
            check_cmd: "go",
            required: false,
        },
        Tool {
            name: "dagger",
            check_cmd: "dagger",
            required: false,
        },
        Tool {
            name: "gh",
            check_cmd: "gh",
            required: false,
        },
        Tool {
            name: "jq",
            check_cmd: "jq",
            required: false,
        },
    ]
}

/// Check which tools are missing
pub fn detect_missing_tools(tools: &[Tool]) -> Vec<&Tool> {
    tools
        .iter()
        .filter(|tool| {
            !Command::new("which")
                .arg(tool.check_cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .collect()
}

/// Install a list of tools
pub fn install_tools(tools: &[&Tool]) -> Result<()> {
    let os = std::env::consts::OS;

    for tool in tools {
        println!("\n📦 Installing {}...", tool.name);

        let result = match tool.name {
            "docker" => install_docker(os),
            "go" => install_go(os),
            "dagger" => install_dagger(os),
            "gh" => install_gh(os),
            "jq" => install_jq(os),
            _ => Ok(false),
        };

        match result {
            Ok(true) => println!("✅ {} installed successfully", tool.name),
            Ok(false) => println!("⚠️  {} installation skipped or failed", tool.name),
            Err(e) => println!("❌ Failed to install {}: {}", tool.name, e),
        }
    }

    Ok(())
}

// Platform-specific installation functions

fn install_docker(os: &str) -> Result<bool> {
    match os {
        "macos" => {
            println!("Installing Docker Desktop via Homebrew...");
            Command::new("brew")
                .args(["install", "--cask", "docker"])
                .status()
                .map(|s| s.success())
                .context("Failed to run brew install docker")
        }
        "linux" => {
            println!("Installing Docker via official script...");
            println!("This requires sudo access.");

            // Download and run Docker install script
            let script_cmd = "curl -fsSL https://get.docker.com | sh";
            Command::new("sh")
                .arg("-c")
                .arg(script_cmd)
                .status()
                .map(|s| s.success())
                .context("Failed to install Docker")
        }
        _ => {
            println!("Please install Docker manually from https://docker.com");
            Ok(false)
        }
    }
}

fn install_go(os: &str) -> Result<bool> {
    match os {
        "macos" => Command::new("brew")
            .args(["install", "go"])
            .status()
            .map(|s| s.success())
            .context("Failed to run brew install go"),
        "linux" => {
            println!("Installing Go...");
            let arch = std::env::consts::ARCH;
            let go_arch = match arch {
                "x86_64" => "amd64",
                "aarch64" => "arm64",
                _ => return Ok(false),
            };

            let url = format!("https://go.dev/dl/go1.21.5.linux-{go_arch}.tar.gz");

            // Download and extract
            let install_cmd = format!("curl -L {url} | sudo tar -C /usr/local -xzf -");

            Command::new("sh")
                .arg("-c")
                .arg(&install_cmd)
                .status()
                .map(|s| s.success())
                .context("Failed to install Go")
        }
        _ => Ok(false),
    }
}

fn install_dagger(os: &str) -> Result<bool> {
    // Dagger requires Go to be installed first
    if !Command::new("which")
        .arg("go")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        println!("Dagger requires Go. Please install Go first.");
        return Ok(false);
    }

    println!("Installing Dagger via official installer...");

    let install_cmd = match os {
        "macos" | "linux" => "curl -L https://dl.dagger.io/dagger/install.sh | sh",
        _ => return Ok(false),
    };

    Command::new("sh")
        .arg("-c")
        .arg(install_cmd)
        .status()
        .map(|s| s.success())
        .context("Failed to install Dagger")
}

fn install_gh(os: &str) -> Result<bool> {
    match os {
        "macos" => Command::new("brew")
            .args(["install", "gh"])
            .status()
            .map(|s| s.success())
            .context("Failed to run brew install gh"),
        "linux" => {
            println!("Installing GitHub CLI...");

            // Add GitHub's package repository and install
            let install_cmds = vec![
                "type -p curl >/dev/null || sudo apt install curl -y",
                "curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg",
                "sudo chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg",
                "echo \"deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main\" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null",
                "sudo apt update",
                "sudo apt install gh -y",
            ];

            for cmd in install_cmds {
                Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .status()
                    .context("Failed to install GitHub CLI")?;
            }

            Ok(true)
        }
        _ => Ok(false),
    }
}

fn install_jq(os: &str) -> Result<bool> {
    match os {
        "macos" => Command::new("brew")
            .args(["install", "jq"])
            .status()
            .map(|s| s.success())
            .context("Failed to run brew install jq"),
        "linux" => {
            // Try apt first, then yum
            if Command::new("which")
                .arg("apt")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                Command::new("sudo")
                    .args(["apt", "install", "-y", "jq"])
                    .status()
                    .map(|s| s.success())
                    .context("Failed to install jq")
            } else if Command::new("which")
                .arg("yum")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                Command::new("sudo")
                    .args(["yum", "install", "-y", "jq"])
                    .status()
                    .map(|s| s.success())
                    .context("Failed to install jq")
            } else {
                println!("Please install jq manually for your distribution");
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}
