//! Secrets command - Secure secret management with age encryption
//!
//! Local-first secrets: age encryption + macOS Keychain + Touch ID.
//! LLMs never see secret values.

use anyhow::{bail, Result};
use patina::{mother, paths, scanner, secrets};
use patina_protocol::{
    BuiltinChild, BuiltinChildAction, BuiltinChildRequest, BuiltinChildResult,
    SecretsAuthorityOperation, SecretsDispatchRequest,
};
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
    mother::control_plane_client()
}

fn current_project_root_str() -> Option<String> {
    env::current_dir()
        .ok()
        .map(|path| path.display().to_string())
}

fn build_authority_payload(op: &str, mut payload: Value, project_root: Option<String>) -> Value {
    let mut request = serde_json::Map::new();
    request.insert("op".to_string(), Value::String(op.to_string()));

    if let Some(root) = project_root {
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

    Value::Object(request)
}

fn dispatch_secrets_authority(op: &str, payload: Value) -> Result<Option<Value>> {
    let client = authority_client();
    let wrapped_payload = build_authority_payload(op, payload, current_project_root_str());
    let operation = SecretsAuthorityOperation::from_payload(wrapped_payload)
        .map_err(|e| anyhow::anyhow!("Invalid secrets-authority request: {}", e))?;
    let request = BuiltinChildRequest::new(
        BuiltinChild::SecretsAuthority,
        BuiltinChildAction::SecretsDispatch(SecretsDispatchRequest { operation }),
    );

    match client.child_action_typed(&request) {
        Ok(response) => match response.result {
            BuiltinChildResult::Dispatch { payload } => Ok(Some(payload)),
            other => Err(anyhow::anyhow!(
                "Unexpected typed response from secrets-authority: {:?}",
                other
            )),
        },
        Err(error) if is_mother_unavailable_error(&error) => bail!(
            "Mother secrets authority unavailable for '{}'. Start `patina mother start`.",
            op
        ),
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

#[derive(Debug, Deserialize)]
struct AuthorityExportIdentityResponse {
    identity: String,
}

#[derive(Debug, Deserialize)]
struct AuthorityImportIdentityResponse {
    recipient: String,
}

#[derive(Debug, Deserialize)]
struct AuthoritySetupClaudeResponse {
    replacing: bool,
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
    let response = dispatch_secrets_authority("status", serde_json::json!({}))?
        .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    let status: AuthorityStatusResponse = serde_json::from_value(response)?;

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

    Ok(())
}

/// Add a secret to the vault
fn add(name: &str, env: Option<&str>, from_stdin: bool, global: bool) -> Result<()> {
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

    let response = dispatch_secrets_authority(
        "add_secret",
        serde_json::json!({
            "name": name,
            "value": secret_value,
            "env": env,
            "global": global,
        }),
    )?
    .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    let parsed: AuthorityAddResponse = serde_json::from_value(response)?;
    if parsed.created_vault {
        println!("Vault created.");
    }
    println!("Added {} -> {}", name, parsed.env_var);
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

    let response = dispatch_secrets_authority("export_identity", serde_json::json!({}))?
        .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    let parsed: AuthorityExportIdentityResponse = serde_json::from_value(response)?;
    let identity = zeroize::Zeroizing::new(parsed.identity);

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

    let response = dispatch_secrets_authority(
        "import_identity",
        serde_json::json!({ "identity": line.trim() }),
    )?
    .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    let parsed: AuthorityImportIdentityResponse = serde_json::from_value(response)?;
    let recipient = parsed.recipient;
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

    let _ = dispatch_secrets_authority("reset_identity", serde_json::json!({}))?
        .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    println!("✓ Identity removed from Keychain");

    Ok(())
}

/// Add a recipient to project vault
fn add_recipient(key: &str) -> Result<()> {
    let _ = dispatch_secrets_authority("add_recipient", serde_json::json!({ "key": key }))?
        .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    println!("✓ Added recipient");
    Ok(())
}

/// Remove a recipient from project vault
fn remove_recipient(key: &str) -> Result<()> {
    let _ = dispatch_secrets_authority("remove_recipient", serde_json::json!({ "key": key }))?
        .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    println!("✓ Removed recipient");
    Ok(())
}

/// List recipients for project vault
fn list_recipients() -> Result<()> {
    let response = dispatch_secrets_authority("list_recipients", serde_json::json!({}))?
        .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    let parsed: AuthorityRecipientsResponse = serde_json::from_value(response)?;
    let recipients = parsed.recipients;

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
    let _ = dispatch_secrets_authority("lock_session", serde_json::json!({}))?
        .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    Ok(())
}

fn remove_secret(name: &str, global: bool) -> Result<()> {
    let _ = dispatch_secrets_authority(
        "remove_secret",
        serde_json::json!({
            "name": name,
            "global": global,
        }),
    )?
    .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    println!("✓ Removed {}", name);
    Ok(())
}

/// Guided setup for Claude Code authentication token.
///
/// Walks the user through generating and storing a long-lived OAuth token
/// so the launcher can inject it for headless/SSH/tmux sessions.
fn setup_claude() -> Result<()> {
    let replacing_hint = matches!(mother::get_global_secret("claude-oauth"), Ok(Some(_)));

    // First-time users need instructions; repeat users just need the prompt
    if !replacing_hint {
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

    let response =
        dispatch_secrets_authority("setup_claude_token", serde_json::json!({ "token": token }))?
            .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    let parsed: AuthoritySetupClaudeResponse = serde_json::from_value(response)?;
    let replacing = parsed.replacing;

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

    #[test]
    fn authority_payload_contains_op_and_project_root() {
        let payload = build_authority_payload(
            "status",
            serde_json::json!({}),
            Some("/tmp/project".to_string()),
        );
        assert_eq!(payload.get("op").and_then(|v| v.as_str()), Some("status"));
        assert_eq!(
            payload.get("project_root").and_then(|v| v.as_str()),
            Some("/tmp/project")
        );
    }

    #[test]
    fn mother_unavailable_error_detection_matches_connection_failures() {
        let error = anyhow::anyhow!("Failed to send child request to http://localhost:50051");
        assert!(is_mother_unavailable_error(&error));

        let error = anyhow::anyhow!("Connection refused (os error 61)");
        assert!(is_mother_unavailable_error(&error));

        let error = anyhow::anyhow!("Invalid JSON payload");
        assert!(!is_mother_unavailable_error(&error));
    }
}
