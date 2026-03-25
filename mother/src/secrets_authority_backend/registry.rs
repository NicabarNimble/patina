use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const REGISTRY_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretDef {
    pub env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsRegistry {
    #[serde(default = "default_version")]
    pub version: u32,
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
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read secrets registry: {:?}", path))?;

        let registry: Self =
            toml::from_str(&content).with_context(|| "Failed to parse secrets.toml")?;

        if registry.version != REGISTRY_VERSION {
            bail!(
                "Unsupported secrets.toml version {}. Expected {}.",
                registry.version,
                REGISTRY_VERSION
            );
        }

        Ok(registry)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let header = "# Patina Secrets Registry\n\
                      # Maps secret names to environment variables\n\n";
        let content = toml::to_string_pretty(self)?;
        let full_content = format!("{}{}", header, content);

        fs::write(path, full_content)
            .with_context(|| format!("Failed to write secrets registry: {:?}", path))?;

        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;

        Ok(())
    }

    pub fn insert(&mut self, name: &str, env: &str) {
        self.secrets.insert(
            name.to_string(),
            SecretDef {
                env: env.to_string(),
            },
        );
    }

    pub fn remove(&mut self, name: &str) -> bool {
        self.secrets.remove(name).is_some()
    }

    pub fn list(&self) -> Vec<&str> {
        self.secrets.keys().map(|s| s.as_str()).collect()
    }
}

pub fn is_valid_secret_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let chars: Vec<char> = name.chars().collect();
    if !chars[0].is_ascii_lowercase() {
        return false;
    }

    let mut prev_hyphen = false;
    for c in &chars[1..] {
        if *c == '-' {
            if prev_hyphen {
                return false;
            }
            prev_hyphen = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false;
        }
    }

    !prev_hyphen
}

pub fn is_valid_env_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let chars: Vec<char> = name.chars().collect();
    if !chars[0].is_ascii_uppercase() {
        return false;
    }

    chars[1..]
        .iter()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
}

pub fn infer_env_name(secret_name: &str) -> String {
    secret_name.to_uppercase().replace('-', "_")
}
