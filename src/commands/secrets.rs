//! Secrets command - Secure secret management with 1Password
//!
//! LLMs never see secret values. Only names and references.
//! Values stored in 1Password "Patina" vault.

use anyhow::Result;
use patina::paths;
use patina::secrets;
use std::env;

/// Secrets CLI subcommands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum SecretsCommands {
    /// One-time setup: create Patina vault in 1Password
    Init,

    /// Register a secret from 1Password to mothership
    Add {
        /// Secret name (lowercase-hyphen, e.g., "github-token")
        name: String,

        /// 1Password item name (defaults to secret name)
        #[arg(long)]
        item: Option<String>,

        /// Field within item (defaults to "credential")
        #[arg(long)]
        field: Option<String>,

        /// Environment variable name (required, e.g., "GITHUB_TOKEN")
        #[arg(long)]
        env: String,
    },

    /// Execute command with secrets injected
    Run {
        /// Remote host for SSH execution (e.g., "root@server")
        #[arg(long)]
        ssh: Option<String>,

        /// Command and arguments to run
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
}

/// Execute secrets command from CLI
pub fn execute_cli(command: Option<SecretsCommands>) -> Result<()> {
    match command {
        Some(cmd) => execute(cmd),
        None => status(), // Bare `patina secrets` shows status
    }
}

/// Execute secrets command
pub fn execute(command: SecretsCommands) -> Result<()> {
    match command {
        SecretsCommands::Init => init(),
        SecretsCommands::Add {
            name,
            item,
            field,
            env,
        } => add(&name, item.as_deref(), field.as_deref(), &env),
        SecretsCommands::Run { ssh, command } => run(ssh.as_deref(), &command),
    }
}

/// Show status: mothership registry + project requirements
fn status() -> Result<()> {
    let op_status = secrets::check_op_status()?;

    // Vault status
    if !op_status.installed {
        println!("1Password CLI: \u{2717} not installed");
        println!("\nInstall with: brew install 1password-cli");
        return Ok(());
    }

    if !op_status.signed_in {
        println!("Patina Vault: \u{2717} not signed in");
        println!("\nRun: op signin");
        return Ok(());
    }

    if op_status.vault_exists {
        println!("Patina Vault: \u{2713} connected");
    } else {
        println!("Patina Vault: \u{2717} not found");
        println!("\nRun: patina secrets init");
        return Ok(());
    }

    // Load registry
    let registry = secrets::load_registry()?;

    // Determine project context
    let project_root = env::current_dir()?;
    let project_config = paths::project::config_path(&project_root);
    let in_project = project_config.exists();

    if in_project {
        // Try to get project name
        let project_name = project_root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string());
        println!("Project: {}", project_name);
    }

    println!();

    // Show registered secrets
    println!("Registered (mothership):");
    if registry.secrets.is_empty() {
        println!("  (no secrets registered)");
    } else {
        let mut secrets: Vec<_> = registry.secrets.iter().collect();
        secrets.sort_by_key(|(name, _)| *name);

        for (name, def) in secrets {
            // Check if secret resolves in 1Password
            let report = secrets::validate_secrets(&[name.clone()], &registry)?;
            let status_icon = if report.is_valid() {
                "\u{2713}"
            } else {
                "\u{2717}"
            };

            let status_suffix = if !report.is_valid() {
                "  NOT FOUND"
            } else {
                ""
            };

            println!(
                "  {} {:<20} \u{2192} {}{}",
                status_icon, name, def.env, status_suffix
            );
        }
    }

    // Show project requirements if in a project
    if in_project {
        let requirements = secrets::load_project_requirements(&project_root)?;

        println!();
        println!("Required (project):");

        if requirements.is_empty() {
            println!("  (no secrets required)");
            println!();
            println!("Add requirements to .patina/config.toml:");
            println!("  [secrets]");
            println!("  requires = [\"github-token\", \"database\"]");
        } else {
            for name in &requirements {
                let (status_icon, status_text) = if registry.contains(name) {
                    let report = secrets::validate_secrets(&[name.clone()], &registry)?;
                    if report.is_valid() {
                        ("\u{2713}", "(registered, resolves)")
                    } else {
                        ("\u{2717}", "(registered, NOT FOUND in 1Password)")
                    }
                } else {
                    ("\u{2717}", "(not registered)")
                };

                println!("  {} {:<20} {}", status_icon, name, status_text);
            }

            // Show action needed
            let missing: Vec<_> = requirements
                .iter()
                .filter(|name| !registry.contains(name))
                .collect();

            if !missing.is_empty() {
                println!();
                println!("Action needed:");
                for name in missing {
                    let suggested_env = name.to_uppercase().replace('-', "_");
                    println!("  patina secrets add {} --env {}", name, suggested_env);
                }
            }
        }
    }

    Ok(())
}

/// Initialize Patina vault
fn init() -> Result<()> {
    secrets::init_vault()
}

/// Add a secret to the registry
fn add(name: &str, item: Option<&str>, field: Option<&str>, env: &str) -> Result<()> {
    secrets::add_secret(name, item, field, env)
}

/// Run command with secrets
fn run(ssh: Option<&str>, command: &[String]) -> Result<()> {
    let project_root = env::current_dir()?;

    let exit_code = if let Some(host) = ssh {
        secrets::run_with_secrets_ssh(&project_root, host, command)?
    } else {
        secrets::run_with_secrets(&project_root, command)?
    };

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_secrets_command_parse() {
        // Just verify the module compiles
        assert!(true);
    }
}
