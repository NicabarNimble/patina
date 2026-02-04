//! Secrets registry parsing (secrets.toml).
//!
//! Maps secret names to environment variable names.
//! Simpler than v1 - no 1Password item/field mapping.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Registry format version.
const REGISTRY_VERSION: u32 = 1;

/// A secret definition in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretDef {
    /// Environment variable name
    pub env: String,
}

/// The secrets registry (secrets.toml).
///
/// Format:
/// ```toml
/// version = 1
///
/// [secrets]
/// github-token = { env = "GITHUB_TOKEN" }
/// openai-key = { env = "OPENAI_API_KEY" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsRegistry {
    /// Registry format version
    #[serde(default = "default_version")]
    pub version: u32,
    /// Secret definitions: name → env var
    #[serde(default)]
    pub secrets: HashMap<String, SecretDef>,
}

fn default_version() -> u32 {
    REGISTRY_VERSION
}

impl Default for SecretsRegistry {
    fn default() -> Self {
        Self {
            version: REGISTRY_VERSION,
            secrets: HashMap::new(),
        }
    }
}

impl SecretsRegistry {
    /// Load registry from a path, or return empty if not found.
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read secrets registry: {:?}", path))?;

        let registry: Self =
            toml::from_str(&content).with_context(|| "Failed to parse secrets.toml")?;

        // Version check
        if registry.version != REGISTRY_VERSION {
            bail!(
                "Unsupported secrets.toml version {}. Expected {}.",
                registry.version,
                REGISTRY_VERSION
            );
        }

        Ok(registry)
    }

    /// Save registry to a path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let header = "# Patina Secrets Registry\n\
                      # Maps secret names to environment variables\n\n";

        let content = toml::to_string_pretty(self)?;
        let full_content = format!("{}{}", header, content);

        fs::write(path, full_content)
            .with_context(|| format!("Failed to write secrets registry: {:?}", path))?;

        // Restrict to owner-only (0o600) — registry maps secret names to env vars
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;

        Ok(())
    }

    /// Insert a secret definition.
    pub fn insert(&mut self, name: &str, env: &str) {
        self.secrets.insert(
            name.to_string(),
            SecretDef {
                env: env.to_string(),
            },
        );
    }

    /// Remove a secret definition.
    pub fn remove(&mut self, name: &str) -> bool {
        self.secrets.remove(name).is_some()
    }

    /// List all registered secret names.
    pub fn list(&self) -> Vec<&str> {
        self.secrets.keys().map(|s| s.as_str()).collect()
    }

    /// Get all secrets as (name, env_var) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.secrets
            .iter()
            .map(|(k, v)| (k.as_str(), v.env.as_str()))
    }
}

// =============================================================================
// Validation helpers (reused from v1)
// =============================================================================

/// Validate secret name format (lowercase-hyphen).
///
/// Valid: `github-token`, `openai-key`, `my-app-db`
/// Invalid: `GITHUB_TOKEN`, `-token`, `token-`, `token--key`
pub fn is_valid_secret_name(name: &str) -> bool {
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

/// Validate environment variable name format (SCREAMING_SNAKE).
///
/// Valid: `GITHUB_TOKEN`, `OPENAI_API_KEY`, `DATABASE_URL`
/// Invalid: `github-token`, `_TOKEN`, `1TOKEN`
pub fn is_valid_env_name(name: &str) -> bool {
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

/// Infer env var name from secret name.
///
/// `github-token` → `GITHUB_TOKEN`
pub fn infer_env_name(secret_name: &str) -> String {
    secret_name.to_uppercase().replace('-', "_")
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
    fn test_infer_env_name() {
        assert_eq!(infer_env_name("github-token"), "GITHUB_TOKEN");
        assert_eq!(infer_env_name("openai-api-key"), "OPENAI_API_KEY");
        assert_eq!(infer_env_name("db"), "DB");
    }

    #[test]
    fn test_registry_roundtrip() {
        let mut registry = SecretsRegistry::default();
        assert_eq!(registry.version, 1);

        registry.insert("github-token", "GITHUB_TOKEN");
        registry.insert("openai-key", "OPENAI_API_KEY");

        let toml_str = toml::to_string_pretty(&registry).unwrap();
        assert!(toml_str.contains("version = 1"));
        assert!(toml_str.contains("[secrets.github-token]"));
        assert!(toml_str.contains("env = \"GITHUB_TOKEN\""));

        let parsed: SecretsRegistry = toml::from_str(&toml_str).unwrap();
        assert!(parsed.secrets.contains_key("github-token"));
        assert_eq!(
            parsed.secrets.get("github-token").unwrap().env,
            "GITHUB_TOKEN"
        );
    }
}
