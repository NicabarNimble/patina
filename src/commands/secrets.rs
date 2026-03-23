//! Secrets command - Secure secret management with age encryption
//!
//! Local-first secrets: age encryption + macOS Keychain + Touch ID.
//! LLMs never see secret values.

use anyhow::{bail, Result};
use patina::{mother, paths, scanner, secrets};
use serde::Deserialize;
use serde_json::Value;
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

    /// Set up Claude Code auth token for headless/SSH sessions
    SetupClaude,
}

/// Flags for bare `patina secrets` command
#[derive(Debug, Clone, clap::Args)]
pub struct SecretsFlags {
    /// Remove a secret
    #[arg(long)]
    pub remove: Option<String>,

    /// Export identity to file (requires --confirm, use --stdout for pipe)
    #[arg(long)]
    pub export_key: bool,

    /// Write exported key to stdout instead of file (for piping)
    #[arg(long)]
    pub stdout: bool,

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
        return lock_session();
    }

    if flags.export_key {
        return export_key(flags.confirm, flags.stdout);
    }

    if flags.import_key {
        return import_key();
    }

    if flags.reset {
        return reset_identity(flags.confirm);
    }

    if let Some(name) = flags.remove {
        return remove_secret(&name, flags.global);
    }

    // Handle subcommands
    match command {
        Some(cmd) => execute(cmd),
        None => status(), // Bare `patina secrets` shows status
    }
}

fn authority_client() -> mother::Client {
    let address = mother::get_address().unwrap_or_else(|| "localhost:50051".to_string());
    mother::Client::new(address)
}

fn current_project_root_str() -> Option<String> {
    env::current_dir()
        .ok()
        .map(|path| path.display().to_string())
}

fn dispatch_secrets_authority(op: &str, mut payload: Value) -> Result<Option<Value>> {
    let mut request = serde_json::Map::new();
    request.insert("op".to_string(), Value::String(op.to_string()));

    if let Some(root) = current_project_root_str() {
        if let Some(map) = payload.as_object_mut() {
            map.entry("project_root".to_string())
                .or_insert(Value::String(root));
        }
    }
    if let Some(map) = payload.as_object() {
        for (k, v) in map {
            request.insert(k.clone(), v.clone());
        }
    }

    let client = authority_client();
    let wrapped_payload = Value::Object(request);

    match client.child_action("secrets-authority", "dispatch", &wrapped_payload) {
        Ok(value) => Ok(Some(value)),
        Err(error) if is_mother_unavailable_error(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

fn is_mother_unavailable_error(error: &anyhow::Error) -> bool {
    let message = error.to_string().to_lowercase();
    message.contains("failed to connect")
        || message.contains("failed to send child request")
        || message.contains("connection refused")
        || message.contains("connect error")
        || message.contains("no such file")
        || message.contains("timed out")
        || message.contains("socket")
}

#[derive(Debug, Deserialize)]
struct AuthorityAddResponse {
    env_var: String,
    created_vault: bool,
}

#[derive(Debug, Deserialize)]
struct AuthorityStatusVault {
    exists: bool,
    secret_count: usize,
    recipient_count: usize,
    secret_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorityStatusResponse {
    identity_source: Option<String>,
    recipient_key: Option<String>,
    global: AuthorityStatusVault,
    project: Option<AuthorityStatusVault>,
}

#[derive(Debug, Deserialize)]
struct AuthorityRecipientsResponse {
    recipients: Vec<String>,
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
        SecretsCommands::SetupClaude => setup_claude(),
    }
}

/// Show status: global and project vaults
fn status() -> Result<()> {
    if let Some(response) = dispatch_secrets_authority("status", serde_json::json!({}))? {
        let status: AuthorityStatusResponse = serde_json::from_value(response)?;

        println!("Identity:");
        match status.identity_source {
            Some(source) => {
                println!("  ✓ Available via {}", source);
                if let Some(key) = status.recipient_key {
                    println!("  Public key: {}", key);
                }
            }
            None => {
                println!("  ✗ Not configured");
                println!("    Run: patina secrets add <name> to create vault and identity");
            }
        }

        println!();
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

        if let Some(project) = status.project {
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
        return Ok(());
    }

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

    let result = if let Some(response) = dispatch_secrets_authority(
        "add_secret",
        serde_json::json!({
            "name": name,
            "value": secret_value,
            "env": env,
            "global": global,
        }),
    )? {
        let parsed: AuthorityAddResponse = serde_json::from_value(response)?;
        secrets::AddResult {
            env_var: parsed.env_var,
            created_vault: parsed.created_vault,
        }
    } else {
        secrets::add_secret(name, &secret_value, env, global, project_root.as_deref())?
    };
    if result.created_vault {
        println!("Vault created.");
    }
    println!("Added {} -> {}", name, result.env_var);
    Ok(())
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

/// Export identity key to file (default) or stdout (--stdout)
fn export_key(confirm: bool, to_stdout: bool) -> Result<()> {
    if !confirm {
        println!("⚠️  This will export your private key.");
        println!("  Add --confirm to proceed.");
        println!("  Add --stdout to print to terminal (for piping).");
        return Ok(());
    }

    let identity = secrets::export_identity()?; // Zeroizing<String> — zeroed on drop

    if to_stdout {
        println!("{}", &*identity);
    } else {
        let key_path = paths::patina_home().join("identity.age");

        std::fs::write(&key_path, identity.as_bytes())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
        }

        println!("✓ Key exported to {} (0o600)", key_path.display());
    }

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
    if dispatch_secrets_authority("add_recipient", serde_json::json!({ "key": key }))?.is_some() {
        println!("✓ Added recipient");
        return Ok(());
    }

    let project_root = env::current_dir()?;
    secrets::add_recipient(&project_root, key)
}

/// Remove a recipient from project vault
fn remove_recipient(key: &str) -> Result<()> {
    if dispatch_secrets_authority("remove_recipient", serde_json::json!({ "key": key }))?.is_some()
    {
        println!("✓ Removed recipient");
        return Ok(());
    }

    let project_root = env::current_dir()?;
    secrets::remove_recipient(&project_root, key)
}

/// List recipients for project vault
fn list_recipients() -> Result<()> {
    let recipients = if let Some(response) =
        dispatch_secrets_authority("list_recipients", serde_json::json!({}))?
    {
        let parsed: AuthorityRecipientsResponse = serde_json::from_value(response)?;
        parsed.recipients
    } else {
        let project_root = env::current_dir()?;
        secrets::list_recipients(&project_root)?
    };

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

fn lock_session() -> Result<()> {
    if dispatch_secrets_authority("lock_session", serde_json::json!({}))?.is_some() {
        return Ok(());
    }
    secrets::lock_session()
}

fn remove_secret(name: &str, global: bool) -> Result<()> {
    if dispatch_secrets_authority(
        "remove_secret",
        serde_json::json!({
            "name": name,
            "global": global,
        }),
    )?
    .is_some()
    {
        println!("✓ Removed {}", name);
        return Ok(());
    }

    let project_root = env::current_dir().ok();
    secrets::remove_secret(name, global, project_root.as_deref())
}

/// Guided setup for Claude Code authentication token.
///
/// Walks the user through generating and storing a long-lived OAuth token
/// so the launcher can inject it for headless/SSH/tmux sessions.
fn setup_claude() -> Result<()> {
    let project_root = env::current_dir().ok();
    let replacing = matches!(secrets::get_global_secret("claude-oauth"), Ok(Some(_)));

    // First-time users need instructions; repeat users just need the prompt
    if !replacing {
        println!("Claude Code headless auth setup");
        println!();
        println!("  1. Run: claude setup-token");
        println!("     (opens browser once, generates a ~1 year token)");
        println!();
        println!("  2. Paste the token below");
        println!();
    }

    eprint!("Token: ");
    let token = secrets::prompt_for_value("claude-oauth")?;

    if token.is_empty() {
        bail!("Token cannot be empty");
    }

    if !token.starts_with("sk-ant-") {
        eprintln!("Warning: doesn't look like a Claude token (expected sk-ant-...)");
        print!("Save anyway? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            return Ok(());
        }
    }

    let _result = secrets::add_secret(
        "claude-oauth",
        &token,
        Some("CLAUDE_CODE_OAUTH_TOKEN"),
        true,
        project_root.as_deref(),
    )?;

    // Migrate the identity to AlwaysThisDeviceOnly so SSH sessions can read it.
    // Re-stores the existing key with the correct Keychain access policy.
    if let Ok(key) = secrets::export_identity() {
        let _ = secrets::import_identity(&key);
    }

    if replacing {
        println!("Token updated.");
    } else {
        println!("Token saved. It will be used automatically by `patina`.");
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
    use super::*;

    #[test]
    fn test_secrets_command_parse() {
        let _ = execute;
    }
}
