//! Internal implementation for secrets management.
//!
//! 1Password integration, registry parsing, validation logic.
//! Not exposed in public API.

use crate::paths;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

// =============================================================================
// Types
// =============================================================================

/// 1Password CLI and vault status.
#[derive(Debug, Clone)]
pub struct OpStatus {
    /// Whether `op` CLI is installed
    pub installed: bool,
    /// CLI version (e.g., "2.30.0")
    pub version: Option<String>,
    /// Whether user is signed in
    pub signed_in: bool,
    /// Account email if signed in
    pub account: Option<String>,
    /// Whether "Patina" vault exists
    pub vault_exists: bool,
    /// Number of items in vault
    pub vault_item_count: Option<u32>,
}

/// A secret definition in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretDef {
    /// 1Password item name
    pub item: String,
    /// Field within the item (defaults to "credential")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Environment variable name (always explicit)
    pub env: String,
}

impl SecretDef {
    /// Get the field name, defaulting to "credential".
    pub fn field_or_default(&self) -> &str {
        self.field.as_deref().unwrap_or("credential")
    }

    /// Generate the op:// reference for this secret.
    pub fn op_reference(&self, vault: &str) -> String {
        format!(
            "op://{}/{}/{}",
            vault,
            self.item,
            self.field_or_default()
        )
    }
}

/// The mothership secrets registry (~/.patina/secrets.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsRegistry {
    /// 1Password vault name (always "Patina")
    #[serde(default = "default_vault")]
    pub vault: String,
    /// Secret definitions
    #[serde(default)]
    pub secrets: HashMap<String, SecretDef>,
}

fn default_vault() -> String {
    "Patina".to_string()
}

impl Default for SecretsRegistry {
    fn default() -> Self {
        Self {
            vault: default_vault(),
            secrets: HashMap::new(),
        }
    }
}

impl SecretsRegistry {
    /// Load the registry from disk, or return empty if not found.
    pub fn load() -> Result<Self> {
        let path = paths::secrets::registry_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read secrets registry: {:?}", path))?;

        toml::from_str(&content).with_context(|| "Failed to parse secrets.toml")
    }

    /// Save the registry to disk.
    pub fn save(&self) -> Result<()> {
        let path = paths::secrets::registry_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let header = "# Patina Secrets Registry\n\
                      # Maps secret names to 1Password items\n\
                      # See: https://github.com/anthropics/patina\n\n";

        let content = toml::to_string_pretty(&self)?;
        let full_content = format!("{}{}", header, content);

        fs::write(&path, full_content)
            .with_context(|| format!("Failed to write secrets registry: {:?}", path))?;

        Ok(())
    }

    /// Get a secret definition by name.
    pub fn get(&self, name: &str) -> Option<&SecretDef> {
        self.secrets.get(name)
    }

    /// Insert a secret definition.
    pub fn insert(&mut self, name: &str, def: SecretDef) {
        self.secrets.insert(name.to_string(), def);
    }

    /// Check if a secret is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.secrets.contains_key(name)
    }

    /// List all registered secret names.
    pub fn list(&self) -> Vec<&str> {
        self.secrets.keys().map(|s| s.as_str()).collect()
    }
}

/// An op:// reference ready for injection.
#[derive(Debug, Clone)]
pub struct OpRef {
    /// Secret name
    pub name: String,
    /// Environment variable name
    pub env_var: String,
    /// op:// reference (e.g., "op://Patina/github-token/credential")
    pub op_reference: String,
}

/// Result of validating secrets.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Secrets that validated successfully
    pub valid: Vec<String>,
    /// Secrets that failed validation (name, error message)
    pub invalid: Vec<(String, String)>,
}

impl ValidationReport {
    /// Check if all secrets are valid.
    pub fn is_valid(&self) -> bool {
        self.invalid.is_empty()
    }
}

// =============================================================================
// 1Password CLI Integration
// =============================================================================

/// Check 1Password CLI status.
pub fn check_op_status() -> Result<OpStatus> {
    // Check if op is installed
    let version_output = Command::new("op").arg("--version").output();

    let (installed, version) = match version_output {
        Ok(output) if output.status.success() => {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(ver))
        }
        _ => (false, None),
    };

    if !installed {
        return Ok(OpStatus {
            installed: false,
            version: None,
            signed_in: false,
            account: None,
            vault_exists: false,
            vault_item_count: None,
        });
    }

    // Check if signed in by trying to get account info
    let account_output = Command::new("op")
        .args(["account", "get", "--format=json"])
        .output()?;

    let (signed_in, account) = if account_output.status.success() {
        let json: serde_json::Value =
            serde_json::from_slice(&account_output.stdout).unwrap_or_default();
        let email = json["email"].as_str().map(|s| s.to_string());
        (true, email)
    } else {
        (false, None)
    };

    if !signed_in {
        return Ok(OpStatus {
            installed,
            version,
            signed_in: false,
            account: None,
            vault_exists: false,
            vault_item_count: None,
        });
    }

    // Check if Patina vault exists
    let vault_output = Command::new("op")
        .args(["vault", "get", "Patina", "--format=json"])
        .output()?;

    let (vault_exists, vault_item_count) = if vault_output.status.success() {
        // Get item count
        let items_output = Command::new("op")
            .args(["item", "list", "--vault=Patina", "--format=json"])
            .output()?;

        let count = if items_output.status.success() {
            let items: Vec<serde_json::Value> =
                serde_json::from_slice(&items_output.stdout).unwrap_or_default();
            Some(items.len() as u32)
        } else {
            None
        };

        (true, count)
    } else {
        (false, None)
    };

    Ok(OpStatus {
        installed,
        version,
        signed_in,
        account,
        vault_exists,
        vault_item_count,
    })
}

/// Initialize the Patina vault.
pub fn init_vault() -> Result<()> {
    let status = check_op_status()?;

    println!("1Password:");
    if !status.installed {
        bail!("  \u{2717} CLI not installed. Install with: brew install 1password-cli");
    }
    println!(
        "  \u{2713} CLI installed ({})",
        status.version.as_deref().unwrap_or("unknown")
    );

    if !status.signed_in {
        bail!("  \u{2717} Not signed in. Run: op signin");
    }
    println!(
        "  \u{2713} Signed in as {}",
        status.account.as_deref().unwrap_or("unknown")
    );

    if status.vault_exists {
        println!(
            "  \u{2713} Patina vault exists ({} items)",
            status.vault_item_count.unwrap_or(0)
        );
    } else {
        println!("\nCreating vault...");
        let create_output = Command::new("op")
            .args(["vault", "create", "Patina"])
            .output()?;

        if !create_output.status.success() {
            let err = String::from_utf8_lossy(&create_output.stderr);
            bail!("Failed to create vault: {}", err);
        }
        println!("  \u{2713} Vault 'Patina' created");
    }

    // Create or verify secrets.toml
    let registry = SecretsRegistry::load()?;
    registry.save()?;
    println!("\n\u{2713} ~/.patina/secrets.toml ready");

    Ok(())
}

// =============================================================================
// Project Requirements
// =============================================================================

/// Load project secret requirements from config.toml.
pub fn load_project_requirements(project_root: &Path) -> Result<Vec<String>> {
    let config_path = crate::paths::project::config_path(project_root);

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read project config: {:?}", config_path))?;

    let config: toml::Value = toml::from_str(&content)?;

    // Extract [secrets] requires = [...]
    let requires = config
        .get("secrets")
        .and_then(|s| s.get("requires"))
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(requires)
}

// =============================================================================
// Validation
// =============================================================================

/// Validate secrets against 1Password.
pub fn validate_secrets(names: &[String], registry: &SecretsRegistry) -> Result<ValidationReport> {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();

    for name in names {
        // Check if registered
        let Some(def) = registry.get(name) else {
            invalid.push((name.clone(), "not registered in mothership".to_string()));
            continue;
        };

        // Check if 1Password item exists
        let op_ref = def.op_reference(&registry.vault);
        let check = Command::new("op")
            .args(["read", &op_ref])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match check {
            Ok(status) if status.success() => {
                valid.push(name.clone());
            }
            _ => {
                invalid.push((
                    name.clone(),
                    format!("item '{}' not found in 1Password", def.item),
                ));
            }
        }
    }

    Ok(ValidationReport { valid, invalid })
}

/// Generate op:// references for secrets.
pub fn generate_op_refs(names: &[String], registry: &SecretsRegistry) -> Result<Vec<OpRef>> {
    let mut refs = Vec::new();

    for name in names {
        let def = registry
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not registered", name))?;

        refs.push(OpRef {
            name: name.clone(),
            env_var: def.env.clone(),
            op_reference: def.op_reference(&registry.vault),
        });
    }

    Ok(refs)
}

// =============================================================================
// Secret Management
// =============================================================================

/// Add a secret to the registry.
pub fn add_secret(
    name: &str,
    item: Option<&str>,
    field: Option<&str>,
    env: &str,
) -> Result<()> {
    // Validate name format (lowercase-hyphen)
    if !is_valid_secret_name(name) {
        bail!(
            "Invalid secret name '{}'. Use lowercase letters, digits, and hyphens (e.g., 'github-token')",
            name
        );
    }

    // Validate env format (SCREAMING_SNAKE)
    if !is_valid_env_name(env) {
        bail!(
            "Invalid env name '{}'. Use uppercase letters, digits, and underscores (e.g., 'GITHUB_TOKEN')",
            env
        );
    }

    let item_name = item.unwrap_or(name);

    // Check if item exists in 1Password
    let status = check_op_status()?;
    if !status.vault_exists {
        bail!("Patina vault not found. Run: patina secrets init");
    }

    let op_ref = format!(
        "op://Patina/{}/{}",
        item_name,
        field.unwrap_or("credential")
    );

    let check = Command::new("op")
        .args(["read", &op_ref])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !check.success() {
        bail!(
            "\u{2717} Item '{}' not found in Patina vault.\n  Create it in 1Password first, then retry.",
            item_name
        );
    }

    println!("\u{2713} Found item '{}' in Patina vault", item_name);

    // Add to registry
    let mut registry = SecretsRegistry::load()?;
    let def = SecretDef {
        item: item_name.to_string(),
        field: field.map(|s| s.to_string()),
        env: env.to_string(),
    };

    registry.insert(name, def.clone());
    registry.save()?;

    // Format the TOML representation
    let field_str = field
        .map(|f| format!(", field = \"{}\"", f))
        .unwrap_or_default();
    println!(
        "\u{2713} Added to ~/.patina/secrets.toml:\n  {} = {{ item = \"{}\"{}, env = \"{}\" }}",
        name, item_name, field_str, env
    );

    Ok(())
}

fn is_valid_secret_name(name: &str) -> bool {
    // ^[a-z][a-z0-9]*(-[a-z0-9]+)*$
    if name.is_empty() {
        return false;
    }

    let chars: Vec<char> = name.chars().collect();

    // Must start with lowercase letter
    if !chars[0].is_ascii_lowercase() {
        return false;
    }

    let mut prev_hyphen = false;
    for c in &chars[1..] {
        if *c == '-' {
            if prev_hyphen {
                return false; // No consecutive hyphens
            }
            prev_hyphen = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false;
        }
    }

    // Can't end with hyphen
    !prev_hyphen
}

fn is_valid_env_name(name: &str) -> bool {
    // ^[A-Z][A-Z0-9_]*$
    if name.is_empty() {
        return false;
    }

    let chars: Vec<char> = name.chars().collect();

    // Must start with uppercase letter
    if !chars[0].is_ascii_uppercase() {
        return false;
    }

    chars[1..]
        .iter()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
}

// =============================================================================
// Execution
// =============================================================================

/// Run a command with secrets injected locally.
pub fn run_with_secrets(project_root: &Path, command: &[String]) -> Result<i32> {
    let requirements = load_project_requirements(project_root)?;

    if requirements.is_empty() {
        // No secrets required, just run the command directly
        println!("No secrets required for this project.");
        let status = Command::new(&command[0])
            .args(&command[1..])
            .status()?;
        return Ok(status.code().unwrap_or(1));
    }

    let registry = SecretsRegistry::load()?;

    // Validate all secrets
    let report = validate_secrets(&requirements, &registry)?;
    if !report.is_valid() {
        eprintln!("Secret validation failed:");
        for (name, err) in &report.invalid {
            eprintln!("  \u{2717} {}: {}", name, err);
        }

        // Suggest fix for first unregistered secret
        for (name, err) in &report.invalid {
            if err.contains("not registered") {
                let suggested_env = name.to_uppercase().replace('-', "_");
                eprintln!(
                    "\nRun: patina secrets add {} --env {}",
                    name, suggested_env
                );
                break;
            }
        }

        return Ok(1);
    }

    // Generate op:// references
    let refs = generate_op_refs(&requirements, &registry)?;

    println!("Resolving secrets for project...");
    for r in &refs {
        println!("  \u{2713} {} \u{2192} {}", r.name, r.env_var);
    }

    // Create temp env file for op run
    let env_file = std::env::temp_dir().join(format!("patina-secrets-{}.env", std::process::id()));
    {
        let mut file = fs::File::create(&env_file)?;
        for r in &refs {
            writeln!(file, "{}={}", r.env_var, r.op_reference)?;
        }
    }

    // Run via op run
    println!("\nRunning: {}", command.join(" "));
    let status = Command::new("op")
        .args(["run", "--env-file"])
        .arg(&env_file)
        .arg("--")
        .args(command)
        .status()?;

    // Cleanup
    let _ = fs::remove_file(&env_file);

    Ok(status.code().unwrap_or(1))
}

/// Run a command on remote host with secrets injected via SSH.
pub fn run_with_secrets_ssh(project_root: &Path, host: &str, command: &[String]) -> Result<i32> {
    let requirements = load_project_requirements(project_root)?;

    if requirements.is_empty() {
        println!("No secrets required for this project.");
        let cmd_str = command.join(" ");
        let status = Command::new("ssh")
            .arg(host)
            .arg(&cmd_str)
            .status()?;
        return Ok(status.code().unwrap_or(1));
    }

    let registry = SecretsRegistry::load()?;

    // Validate all secrets
    let report = validate_secrets(&requirements, &registry)?;
    if !report.is_valid() {
        eprintln!("Secret validation failed:");
        for (name, err) in &report.invalid {
            eprintln!("  \u{2717} {}: {}", name, err);
        }
        return Ok(1);
    }

    // Generate op:// references and resolve them locally
    let refs = generate_op_refs(&requirements, &registry)?;

    println!("Resolving secrets for project...");
    let mut env_prefix = String::new();
    for r in &refs {
        // Read secret value locally
        let output = Command::new("op")
            .args(["read", &r.op_reference])
            .output()?;

        if !output.status.success() {
            bail!("Failed to read secret '{}' from 1Password", r.name);
        }

        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Escape single quotes in value for shell
        let escaped_value = value.replace('\'', "'\\''");
        env_prefix.push_str(&format!("{}='{}' ", r.env_var, escaped_value));

        println!("  \u{2713} {} \u{2192} {}", r.name, r.env_var);
    }

    // Construct remote command with env vars
    let remote_cmd = format!("{}{}", env_prefix, command.join(" "));

    println!("\nInjecting via SSH...");
    println!("Running on {}: {}", host, command.join(" "));

    let status = Command::new("ssh")
        .arg(host)
        .arg(&remote_cmd)
        .status()?;

    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_name_validation() {
        // Valid names
        assert!(is_valid_secret_name("github-token"));
        assert!(is_valid_secret_name("openai-key"));
        assert!(is_valid_secret_name("a"));
        assert!(is_valid_secret_name("abc123"));
        assert!(is_valid_secret_name("my-app-db"));

        // Invalid names
        assert!(!is_valid_secret_name(""));
        assert!(!is_valid_secret_name("GITHUB_TOKEN")); // uppercase
        assert!(!is_valid_secret_name("-token")); // starts with hyphen
        assert!(!is_valid_secret_name("token-")); // ends with hyphen
        assert!(!is_valid_secret_name("token--key")); // consecutive hyphens
        assert!(!is_valid_secret_name("1token")); // starts with digit
        assert!(!is_valid_secret_name("token_key")); // underscore
    }

    #[test]
    fn test_env_name_validation() {
        // Valid names
        assert!(is_valid_env_name("GITHUB_TOKEN"));
        assert!(is_valid_env_name("A"));
        assert!(is_valid_env_name("ABC123"));
        assert!(is_valid_env_name("MY_APP_DB_URL"));

        // Invalid names
        assert!(!is_valid_env_name(""));
        assert!(!is_valid_env_name("github-token")); // lowercase
        assert!(!is_valid_env_name("_TOKEN")); // starts with underscore
        assert!(!is_valid_env_name("1TOKEN")); // starts with digit
        assert!(!is_valid_env_name("TOKEN-KEY")); // hyphen
    }

    #[test]
    fn test_secret_def_op_reference() {
        let def = SecretDef {
            item: "github-token".to_string(),
            field: None,
            env: "GITHUB_TOKEN".to_string(),
        };

        assert_eq!(
            def.op_reference("Patina"),
            "op://Patina/github-token/credential"
        );

        let def_with_field = SecretDef {
            item: "postgres-prod".to_string(),
            field: Some("connection_string".to_string()),
            env: "DATABASE_URL".to_string(),
        };

        assert_eq!(
            def_with_field.op_reference("Patina"),
            "op://Patina/postgres-prod/connection_string"
        );
    }

    #[test]
    fn test_registry_roundtrip() {
        let mut registry = SecretsRegistry::default();
        assert_eq!(registry.vault, "Patina");

        registry.insert(
            "github-token",
            SecretDef {
                item: "github-token".to_string(),
                field: None,
                env: "GITHUB_TOKEN".to_string(),
            },
        );

        let toml_str = toml::to_string_pretty(&registry).unwrap();
        assert!(toml_str.contains("vault = \"Patina\""));
        assert!(toml_str.contains("[secrets.github-token]"));
        assert!(toml_str.contains("env = \"GITHUB_TOKEN\""));

        let parsed: SecretsRegistry = toml::from_str(&toml_str).unwrap();
        assert!(parsed.contains("github-token"));
        assert_eq!(parsed.get("github-token").unwrap().env, "GITHUB_TOKEN");
    }
}
