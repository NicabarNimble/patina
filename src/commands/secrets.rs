//! Secrets command - Secure secret management with age encryption
//!
//! Local-first secrets: age encryption + macOS Keychain + Touch ID.
//! LLMs never see secret values.

use anyhow::{bail, Result};
use patina::{scanner, secrets};
use std::env;
use std::io::{self, BufRead, Write};

/// Secrets CLI subcommands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum SecretsCommands {
    /// Add a secret to the vault
    Add {
        /// Secret name (lowercase-hyphen, e.g., "github-token")
        name: String,

        /// Environment variable name (optional, inferred from name)
        #[arg(long)]
        env: Option<String>,

        /// Read secret value from stdin (for scripting/piping)
        #[arg(long)]
        stdin: bool,

        /// Add to global vault instead of project vault
        #[arg(long)]
        global: bool,
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

    /// Add a recipient to the project vault
    AddRecipient {
        /// age public key (age1...)
        key: String,
    },

    /// Remove a recipient from the project vault
    RemoveRecipient {
        /// age public key (age1...)
        key: String,
    },

    /// List recipients for the project vault
    ListRecipients,

    /// Scan staged files for exposed secrets (pre-commit)
    Check,

    /// Scan all tracked files for exposed secrets
    Audit,
}

/// Flags for bare `patina secrets` command
#[derive(Debug, Clone, clap::Args)]
pub struct SecretsFlags {
    /// Remove a secret
    #[arg(long)]
    pub remove: Option<String>,

    /// Export identity (requires --confirm)
    #[arg(long)]
    pub export_key: bool,

    /// Import identity from stdin
    #[arg(long)]
    pub import_key: bool,

    /// Reset identity - remove from Keychain (requires --confirm)
    #[arg(long)]
    pub reset: bool,

    /// Clear session cache
    #[arg(long)]
    pub lock: bool,

    /// Confirm dangerous operation
    #[arg(long)]
    pub confirm: bool,

    /// Operate on global vault instead of project
    #[arg(long)]
    pub global: bool,
}

/// Execute secrets command from CLI
pub fn execute_cli(command: Option<SecretsCommands>, flags: SecretsFlags) -> Result<()> {
    // Handle flags first
    if flags.lock {
        return secrets::lock_session();
    }

    if flags.export_key {
        return export_key(flags.confirm);
    }

    if flags.import_key {
        return import_key();
    }

    if flags.reset {
        return reset_identity(flags.confirm);
    }

    if let Some(name) = flags.remove {
        let project_root = env::current_dir().ok();
        return secrets::remove_secret(&name, flags.global, project_root.as_deref());
    }

    // Handle subcommands
    match command {
        Some(cmd) => execute(cmd),
        None => status(), // Bare `patina secrets` shows status
    }
}

/// Execute secrets subcommand
pub fn execute(command: SecretsCommands) -> Result<()> {
    match command {
        SecretsCommands::Add {
            name,
            env,
            stdin,
            global,
        } => add(&name, env.as_deref(), stdin, global),
        SecretsCommands::Run { ssh, command } => run(ssh.as_deref(), &command),
        SecretsCommands::AddRecipient { key } => add_recipient(&key),
        SecretsCommands::RemoveRecipient { key } => remove_recipient(&key),
        SecretsCommands::ListRecipients => list_recipients(),
        SecretsCommands::Check => check_staged(),
        SecretsCommands::Audit => audit_tracked(),
    }
}

/// Show status: global and project vaults
fn status() -> Result<()> {
    let project_root = env::current_dir().ok();
    let status = secrets::check_status(project_root.as_deref())?;

    // Identity status
    println!("Identity:");
    match status.identity_source {
        Some(source) => {
            println!("  ✓ Available via {}", source);
            if let Some(ref key) = status.recipient_key {
                println!("  Public key: {}", key);
            }
        }
        None => {
            println!("  ✗ Not configured");
            println!("    Run: patina secrets add <name> to create vault and identity");
        }
    }

    println!();

    // Global vault
    println!("Global vault (~/.patina/):");
    if status.global.exists {
        println!(
            "  ✓ {} secrets, {} recipients",
            status.global.secret_count, status.global.recipient_count
        );
        if !status.global.secret_names.is_empty() {
            println!("  Secrets: {}", status.global.secret_names.join(", "));
        }
    } else {
        println!("  ✗ Not initialized");
    }

    // Project vault
    if let Some(project) = &status.project {
        println!();
        println!("Project vault (.patina/):");
        if project.exists {
            println!(
                "  ✓ {} secrets, {} recipients",
                project.secret_count, project.recipient_count
            );
            if !project.secret_names.is_empty() {
                println!("  Secrets: {}", project.secret_names.join(", "));
            }
        } else {
            println!("  ✗ Not initialized");
        }
    }

    println!();
    println!("Commands:");
    println!("  patina secrets add NAME [--stdin]     Add a secret");
    println!("  patina secrets run -- CMD            Run with secrets");
    println!("  patina secrets --lock                Clear session cache");

    Ok(())
}

/// Add a secret to the vault
fn add(name: &str, env: Option<&str>, from_stdin: bool, global: bool) -> Result<()> {
    let project_root = env::current_dir().ok();

    // Get value: from --stdin flag or interactive masked prompt
    let secret_value = if from_stdin {
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        line.trim().to_string()
    } else if atty::is(atty::Stream::Stdin) {
        // Interactive: masked prompt (no echo)
        eprint!("Value for {}: ", name);
        secrets::prompt_for_value(name)?
    } else {
        // Piped input without --stdin flag
        bail!("Use --stdin to read secret values from a pipe");
    };

    if secret_value.is_empty() {
        bail!("Secret value cannot be empty");
    }

    secrets::add_secret(name, &secret_value, env, global, project_root.as_deref())
}

/// Run command with secrets
fn run(ssh: Option<&str>, command: &[String]) -> Result<()> {
    let project_root = env::current_dir().ok();

    let exit_code = if let Some(host) = ssh {
        secrets::run_with_secrets_ssh(project_root.as_deref(), host, command)?
    } else {
        secrets::run_with_secrets(project_root.as_deref(), command)?
    };

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}

/// Export identity key
fn export_key(confirm: bool) -> Result<()> {
    if !confirm {
        println!("⚠️  This will print your private key.");
        println!("  Add --confirm to proceed.");
        return Ok(());
    }

    let identity = secrets::export_identity()?;
    println!("⚠️  PRIVATE KEY - DO NOT SHARE");
    println!("{}", identity);

    Ok(())
}

/// Import identity key
fn import_key() -> Result<()> {
    print!("Paste identity: ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;

    let recipient = secrets::import_identity(line.trim())?;
    println!("✓ Stored in macOS Keychain (Touch ID protected)");
    println!("  Public key: {}", recipient);

    Ok(())
}

/// Reset identity - remove from Keychain
fn reset_identity(confirm: bool) -> Result<()> {
    if !confirm {
        println!("⚠️  This will DELETE your private key from Keychain.");
        println!("  You will lose access to all encrypted vaults unless you have a backup.");
        println!("  Add --confirm to proceed.");
        return Ok(());
    }

    secrets::reset_identity()?;
    println!("✓ Identity removed from Keychain");

    Ok(())
}

/// Add a recipient to project vault
fn add_recipient(key: &str) -> Result<()> {
    let project_root = env::current_dir()?;
    secrets::add_recipient(&project_root, key)
}

/// Remove a recipient from project vault
fn remove_recipient(key: &str) -> Result<()> {
    let project_root = env::current_dir()?;
    secrets::remove_recipient(&project_root, key)
}

/// List recipients for project vault
fn list_recipients() -> Result<()> {
    let project_root = env::current_dir()?;
    let recipients = secrets::list_recipients(&project_root)?;

    if recipients.is_empty() {
        println!("No recipients configured.");
        println!("  Run: patina secrets add <name> to initialize vault");
    } else {
        println!("Recipients ({}):", recipients.len());
        for r in recipients {
            println!("  {}", r);
        }
    }

    Ok(())
}

/// Scan staged files for exposed secrets (pre-commit check)
fn check_staged() -> Result<()> {
    let repo_root = env::current_dir()?;

    let findings = scanner::scan_staged(&repo_root)?;

    if findings.is_empty() {
        println!("No secrets found in staged files.");
        return Ok(());
    }

    println!("Found {} secret(s):\n", findings.len());
    print_findings(&findings);

    println!("\nCommit blocked. Remove secret or use `patina secrets add`.");
    std::process::exit(1);
}

/// Scan all tracked files for exposed secrets
fn audit_tracked() -> Result<()> {
    let repo_root = env::current_dir()?;

    let findings = scanner::scan_tracked(&repo_root)?;

    if findings.is_empty() {
        println!("All clear - no secrets found.");
        return Ok(());
    }

    println!("Found {} secret(s):\n", findings.len());
    print_findings(&findings);

    std::process::exit(1);
}

fn print_findings(findings: &[scanner::Finding]) {
    for f in findings {
        println!("  {}:{}:{}", f.path.display(), f.line, f.column);
        println!("    Pattern: {}", f.pattern);
        println!("    Severity: {}", f.severity);
        println!("    Match: {}", f.matched);
        println!();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_secrets_command_parse() {
        // Just verify the module compiles
        assert!(true);
    }
}
