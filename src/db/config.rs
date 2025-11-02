//! Database configuration and backend selection
//!
//! Provides configuration types and factory functions for creating
//! database backends based on user configuration.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

use super::DatabaseBackend;

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Which backend to use (sqlite or turso)
    pub backend: BackendType,

    /// SQLite-specific configuration
    #[serde(default)]
    pub sqlite: SqliteConfig,

    /// Turso-specific configuration (optional, only if backend = "turso")
    pub turso: Option<TursoConfig>,
}

/// Backend type selection
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// SQLite with sqlite-vec extension
    Sqlite,
    /// Turso (libsql) backend
    Turso,
}

/// SQLite configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SqliteConfig {
    /// Path to database file
    #[serde(default = "default_sqlite_path")]
    pub path: String,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: default_sqlite_path(),
        }
    }
}

fn default_sqlite_path() -> String {
    ".patina/db/facts.db".to_string()
}

/// Turso configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TursoConfig {
    /// Turso mode (local, embedded, or remote)
    pub mode: TursoMode,

    /// Path to local database file (for local and embedded modes)
    pub path: Option<String>,

    /// Turso database URL (for embedded and remote modes)
    pub url: Option<String>,

    /// Authentication token (for embedded and remote modes)
    pub auth_token: Option<String>,

    /// Auto-sync interval in seconds (for embedded mode)
    pub sync_interval_seconds: Option<u64>,
}

/// Turso operation mode
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TursoMode {
    /// Local-only libsql database (no cloud sync)
    Local,
    /// Embedded replica (local file + cloud sync)
    Embedded,
    /// Remote-only (HTTP to Turso cloud)
    Remote,
}

impl DatabaseConfig {
    /// Load configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Self = toml::from_str(&content)
            .context("Failed to parse database configuration")?;

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from string
    pub fn load_from_str(content: &str) -> Result<Self> {
        let config: Self = toml::from_str(content)
            .context("Failed to parse database configuration")?;

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        match self.backend {
            BackendType::Turso => {
                let turso = self.turso.as_ref()
                    .context("Turso backend selected but no turso configuration provided")?;

                match turso.mode {
                    TursoMode::Local => {
                        if turso.path.is_none() {
                            anyhow::bail!("Turso local mode requires 'path' field");
                        }
                    }
                    TursoMode::Embedded => {
                        if turso.path.is_none() {
                            anyhow::bail!("Turso embedded mode requires 'path' field");
                        }
                        if turso.url.is_none() {
                            anyhow::bail!("Turso embedded mode requires 'url' field");
                        }
                        if turso.auth_token.is_none() {
                            anyhow::bail!("Turso embedded mode requires 'auth_token' field");
                        }
                    }
                    TursoMode::Remote => {
                        if turso.url.is_none() {
                            anyhow::bail!("Turso remote mode requires 'url' field");
                        }
                        if turso.auth_token.is_none() {
                            anyhow::bail!("Turso remote mode requires 'auth_token' field");
                        }
                    }
                }
            }
            BackendType::Sqlite => {
                // SQLite config is always valid (has defaults)
            }
        }

        Ok(())
    }

    /// Open database backend from this configuration
    pub fn open(&self) -> Result<DatabaseBackend> {
        match self.backend {
            BackendType::Sqlite => {
                let path = &self.sqlite.path;
                DatabaseBackend::open_sqlite(path)
                    .with_context(|| format!("Failed to open SQLite database at {}", path))
            }
            BackendType::Turso => {
                anyhow::bail!("Turso backend not yet implemented")
                // TODO: Implement Turso backend opening
                // let turso = self.turso.as_ref().unwrap(); // Validated above
                // match turso.mode {
                //     TursoMode::Local => {
                //         DatabaseBackend::open_turso_local(turso.path.as_ref().unwrap())
                //     }
                //     TursoMode::Embedded => {
                //         DatabaseBackend::open_turso_embedded(
                //             turso.path.as_ref().unwrap(),
                //             turso.url.as_ref().unwrap(),
                //             turso.auth_token.as_ref().unwrap(),
                //         )
                //     }
                //     TursoMode::Remote => {
                //         DatabaseBackend::open_turso_remote(
                //             turso.url.as_ref().unwrap(),
                //             turso.auth_token.as_ref().unwrap(),
                //         )
                //     }
                // }
            }
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            backend: BackendType::Sqlite,
            sqlite: SqliteConfig::default(),
            turso: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DatabaseConfig::default();
        assert_eq!(config.backend, BackendType::Sqlite);
        assert_eq!(config.sqlite.path, ".patina/db/facts.db");
    }

    #[test]
    fn test_parse_sqlite_config() {
        let toml = r#"
            backend = "sqlite"

            [sqlite]
            path = "/custom/path/db.sqlite"
        "#;

        let config = DatabaseConfig::load_from_str(toml).unwrap();
        assert_eq!(config.backend, BackendType::Sqlite);
        assert_eq!(config.sqlite.path, "/custom/path/db.sqlite");
    }

    #[test]
    fn test_parse_turso_local_config() {
        let toml = r#"
            backend = "turso"

            [turso]
            mode = "local"
            path = ".patina/db/facts.db"
        "#;

        let config = DatabaseConfig::load_from_str(toml).unwrap();
        assert_eq!(config.backend, BackendType::Turso);

        let turso = config.turso.unwrap();
        assert_eq!(turso.mode, TursoMode::Local);
        assert_eq!(turso.path.unwrap(), ".patina/db/facts.db");
    }

    #[test]
    fn test_parse_turso_embedded_config() {
        let toml = r#"
            backend = "turso"

            [turso]
            mode = "embedded"
            path = ".patina/db/facts.db"
            url = "libsql://patina-oxidizer.turso.io"
            auth_token = "test-token"
            sync_interval_seconds = 300
        "#;

        let config = DatabaseConfig::load_from_str(toml).unwrap();
        assert_eq!(config.backend, BackendType::Turso);

        let turso = config.turso.unwrap();
        assert_eq!(turso.mode, TursoMode::Embedded);
        assert_eq!(turso.path.unwrap(), ".patina/db/facts.db");
        assert_eq!(turso.url.unwrap(), "libsql://patina-oxidizer.turso.io");
        assert_eq!(turso.auth_token.unwrap(), "test-token");
        assert_eq!(turso.sync_interval_seconds.unwrap(), 300);
    }

    #[test]
    fn test_validate_turso_missing_config() {
        let toml = r#"
            backend = "turso"
        "#;

        let result = DatabaseConfig::load_from_str(toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no turso configuration"));
    }

    #[test]
    fn test_validate_turso_local_missing_path() {
        let toml = r#"
            backend = "turso"

            [turso]
            mode = "local"
        "#;

        let result = DatabaseConfig::load_from_str(toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires 'path'"));
    }

    #[test]
    fn test_validate_turso_embedded_missing_url() {
        let toml = r#"
            backend = "turso"

            [turso]
            mode = "embedded"
            path = ".patina/db/facts.db"
            auth_token = "test-token"
        "#;

        let result = DatabaseConfig::load_from_str(toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires 'url'"));
    }
}
