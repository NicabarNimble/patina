//! Single source of truth for ALL Patina filesystem layout.
//!
//! This module defines WHERE data lives. It has no I/O, no validation,
//! no business logic. One file shows the entire filesystem layout.
//!
//! # Design Philosophy
//!
//! From `rationale-eskil-steenberg.md`:
//! > "It's faster to write 5 lines of code today than to write 1 line today and edit it later."
//!
//! This API is complete from day one - user-level AND project-level paths.
//! The API never needs to change. Migrations can happen incrementally.
//!
//! # User-Level Paths (~/.patina/)
//!
//! ```text
//! ~/.patina/
//! ├── config.toml              # Global config
//! ├── registry.yaml            # Project/repo registry
//! ├── adapters/                # LLM adapter templates
//! ├── personas/default/events/ # Source (valuable)
//! └── cache/                   # Derived (rebuildable)
//!     ├── repos/               # Cloned reference repos
//!     └── personas/default/    # Materialized indices
//! ```
//!
//! # Project-Level Paths (project/.patina/)
//!
//! ```text
//! project/.patina/
//! ├── config.toml              # Project config
//! ├── oxidize.yaml             # Embedding recipe
//! ├── versions.json            # Version manifest
//! ├── backups/                 # Backup files
//! └── data/                    # Derived (gitignored)
//!     ├── patina.db            # SQLite database
//!     └── embeddings/          # Vector indices
//! ```

use std::path::{Path, PathBuf};

// =============================================================================
// User Level (~/.patina/)
// =============================================================================

/// User's patina home directory: `~/.patina/`
pub fn patina_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".patina")
}

/// Cache directory for all rebuildable data: `~/.patina/cache/`
pub fn patina_cache() -> PathBuf {
    patina_home().join("cache")
}

/// Global config file: `~/.patina/config.toml`
pub fn config_path() -> PathBuf {
    patina_home().join("config.toml")
}

/// Project/repo registry: `~/.patina/registry.yaml`
pub fn registry_path() -> PathBuf {
    patina_home().join("registry.yaml")
}

/// LLM adapter templates: `~/.patina/adapters/`
pub fn adapters_dir() -> PathBuf {
    patina_home().join("adapters")
}

/// Persona paths (cross-project user knowledge)
pub mod persona {
    use super::*;

    /// Source events (valuable): `~/.patina/personas/default/events/`
    pub fn events_dir() -> PathBuf {
        patina_home().join("personas/default/events")
    }

    /// Materialized cache (rebuildable): `~/.patina/cache/personas/default/`
    pub fn cache_dir() -> PathBuf {
        patina_cache().join("personas/default")
    }
}

/// Reference repository paths
pub mod repos {
    use super::*;

    /// Cloned repos (rebuildable): `~/.patina/cache/repos/`
    pub fn cache_dir() -> PathBuf {
        patina_cache().join("repos")
    }
}

/// Secrets management paths (v2 - local age-encrypted vault)
pub mod secrets {
    use super::*;
    use std::path::Path;

    // =========================================================================
    // Global (mothership) paths - ~/.patina/
    // =========================================================================

    /// Global secrets registry: `~/.patina/secrets.toml`
    pub fn registry_path() -> PathBuf {
        patina_home().join("secrets.toml")
    }

    /// Global vault (encrypted): `~/.patina/vault.age`
    pub fn vault_path() -> PathBuf {
        patina_home().join("vault.age")
    }

    /// Global recipient (your public key): `~/.patina/recipient.txt`
    /// Note: singular - global vault has one recipient (you)
    pub fn recipient_path() -> PathBuf {
        patina_home().join("recipient.txt")
    }

    // =========================================================================
    // Project paths - {project}/.patina/
    // =========================================================================

    /// Project secrets registry: `{root}/.patina/secrets.toml`
    pub fn project_registry_path(root: &Path) -> PathBuf {
        root.join(".patina").join("secrets.toml")
    }

    /// Project vault (encrypted): `{root}/.patina/vault.age`
    pub fn project_vault_path(root: &Path) -> PathBuf {
        root.join(".patina").join("vault.age")
    }

    /// Project recipients (shared): `{root}/.patina/recipients.txt`
    /// Note: plural - project vault has multiple recipients
    pub fn project_recipients_path(root: &Path) -> PathBuf {
        root.join(".patina").join("recipients.txt")
    }
}

/// Model management paths (base models shared across projects)
pub mod models {
    use super::*;

    /// Model cache directory: `~/.patina/cache/models/`
    pub fn cache_dir() -> PathBuf {
        patina_cache().join("models")
    }

    /// Specific model directory: `~/.patina/cache/models/{name}/`
    pub fn model_dir(name: &str) -> PathBuf {
        cache_dir().join(name)
    }

    /// Model ONNX file: `~/.patina/cache/models/{name}/model.onnx`
    pub fn model_onnx(name: &str) -> PathBuf {
        model_dir(name).join("model.onnx")
    }

    /// Model tokenizer: `~/.patina/cache/models/{name}/tokenizer.json`
    pub fn model_tokenizer(name: &str) -> PathBuf {
        model_dir(name).join("tokenizer.json")
    }

    /// Lock file tracking provenance: `~/.patina/models.lock`
    pub fn lock_path() -> PathBuf {
        patina_home().join("models.lock")
    }
}

// =============================================================================
// Project Level (project/.patina/)
// =============================================================================

/// Project-level paths, relative to a project root.
///
/// All functions take a `root: &Path` parameter - the project directory.
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use patina::paths::project;
///
/// let root = Path::new("/home/user/myproject");
/// let db = project::db_path(root);
/// assert_eq!(db, Path::new("/home/user/myproject/.patina/data/patina.db"));
/// ```
pub mod project {
    use super::*;

    /// Project's patina directory: `.patina/`
    pub fn patina_dir(root: &Path) -> PathBuf {
        root.join(".patina")
    }

    /// Project config: `.patina/config.toml`
    pub fn config_path(root: &Path) -> PathBuf {
        root.join(".patina/config.toml")
    }

    /// Derived data directory (gitignored): `.patina/data/`
    pub fn data_dir(root: &Path) -> PathBuf {
        root.join(".patina/data")
    }

    /// Main SQLite database: `.patina/data/patina.db`
    pub fn db_path(root: &Path) -> PathBuf {
        root.join(".patina/data/patina.db")
    }

    /// Embedding indices: `.patina/data/embeddings/`
    pub fn embeddings_dir(root: &Path) -> PathBuf {
        root.join(".patina/data/embeddings")
    }

    /// Model-specific projections: `.patina/data/embeddings/{model}/projections/`
    pub fn model_projections_dir(root: &Path, model: &str) -> PathBuf {
        root.join(format!(".patina/data/embeddings/{}/projections", model))
    }

    /// Oxidize recipe: `.patina/oxidize.yaml`
    pub fn recipe_path(root: &Path) -> PathBuf {
        root.join(".patina/oxidize.yaml")
    }

    /// Version manifest: `.patina/versions.json`
    pub fn versions_path(root: &Path) -> PathBuf {
        root.join(".patina/versions.json")
    }

    /// Backup directory: `.patina/backups/`
    pub fn backups_dir(root: &Path) -> PathBuf {
        root.join(".patina/backups")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patina_home() {
        let home = patina_home();
        assert!(home.ends_with(".patina"));
    }

    #[test]
    fn test_patina_cache() {
        let cache = patina_cache();
        assert!(cache.ends_with("cache"));
        assert!(cache.starts_with(patina_home()));
    }

    #[test]
    fn test_persona_paths() {
        let events = persona::events_dir();
        let cache = persona::cache_dir();

        assert!(events.to_string_lossy().contains("personas/default/events"));
        assert!(cache.to_string_lossy().contains("cache/personas/default"));
    }

    #[test]
    fn test_repos_cache() {
        let repos = repos::cache_dir();
        assert!(repos.to_string_lossy().contains("cache/repos"));
    }

    #[test]
    fn test_models_paths() {
        let cache = models::cache_dir();
        assert!(cache.to_string_lossy().contains("cache/models"));

        let model_dir = models::model_dir("e5-base-v2");
        assert!(model_dir
            .to_string_lossy()
            .contains("cache/models/e5-base-v2"));

        let onnx = models::model_onnx("e5-base-v2");
        assert!(onnx.to_string_lossy().ends_with("e5-base-v2/model.onnx"));

        let tokenizer = models::model_tokenizer("e5-base-v2");
        assert!(tokenizer
            .to_string_lossy()
            .ends_with("e5-base-v2/tokenizer.json"));

        let lock = models::lock_path();
        assert!(lock.to_string_lossy().ends_with("models.lock"));
        // Lock is at ~/.patina/, not in cache
        assert!(!lock.to_string_lossy().contains("cache"));
    }

    #[test]
    fn test_project_paths() {
        let root = Path::new("/tmp/test-project");

        assert_eq!(
            project::patina_dir(root),
            PathBuf::from("/tmp/test-project/.patina")
        );
        assert_eq!(
            project::db_path(root),
            PathBuf::from("/tmp/test-project/.patina/data/patina.db")
        );
        assert_eq!(
            project::model_projections_dir(root, "e5-base-v2"),
            PathBuf::from("/tmp/test-project/.patina/data/embeddings/e5-base-v2/projections")
        );
    }
}
