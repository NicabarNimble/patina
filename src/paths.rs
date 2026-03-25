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
//! ├── connections/             # Connection records (TOML)
//! ├── personas/default/events/ # Source (valuable)
//! ├── run/                     # Runtime (socket, pid, token)
//! │   ├── serve.sock           # Unix domain socket
//! │   └── serve.token          # Bearer token file (TCP only)
//! └── cache/                   # Derived (rebuildable)
//!     ├── repos/               # Cloned reference repos
//!     └── personas/default/    # Materialized indices
//! ```
//!
//! # Project-Level Paths (project/.patina/)
//!
//! ```text
//! project/.patina/
//! ├── config.toml              # Project config (committed)
//! ├── uid                      # Project identity (committed)
//! ├── oxidize.yaml             # Embedding recipe (committed)
//! ├── versions.json            # Version manifest (committed)
//! └── local/                   # Local state (gitignored)
//!     ├── data/
//!     │   ├── patina.db        # SQLite database
//!     │   └── embeddings/      # Vector indices
//!     └── backups/             # Backup files
//! ```

use std::path::{Path, PathBuf};

// =============================================================================
// User Level (~/.patina/)
// =============================================================================

/// User's patina home directory: `~/.patina/`
pub fn patina_home() -> PathBuf {
    if let Some(override_path) = std::env::var_os("PATINA_HOME") {
        return PathBuf::from(override_path);
    }

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
    // Global (mother) paths - ~/.patina/
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

/// Serve daemon runtime paths (socket, pid, token)
pub mod serve {
    use super::*;

    /// Runtime directory: `~/.patina/run/`
    /// Permissions: 0o700 (owner only)
    pub fn run_dir() -> PathBuf {
        patina_home().join("run")
    }

    /// Unix domain socket: `~/.patina/run/serve.sock`
    /// Permissions: 0o600 (owner only)
    pub fn socket_path() -> PathBuf {
        run_dir().join("serve.sock")
    }

    /// Bearer token file (TCP only): `~/.patina/run/serve.token`
    /// Permissions: 0o600 (owner only)
    pub fn token_path() -> PathBuf {
        run_dir().join("serve.token")
    }

    /// PID file: `~/.patina/run/mother.pid`
    /// Permissions: 0o600 (owner only)
    pub fn pid_path() -> PathBuf {
        run_dir().join("mother.pid")
    }
}

/// Child runtime paths (WASM children, command children, work dirs)
pub mod child {
    use super::*;

    /// WASM children directory: `~/.patina/children/`
    /// Contains .wasm files + child manifests (`child.toml`) for Mother daemon children.
    pub fn children_dir() -> PathBuf {
        patina_home().join("children")
    }

    /// CLI command plugins directory: `~/.patina/plugins/`
    /// Contains .wasm files + `.toml` child manifests for CLI command plugins (Phase 2+).
    pub fn plugins_dir() -> PathBuf {
        patina_home().join("plugins")
    }

    /// Plugin work directory (WASI sandbox root): `~/.patina/plugins/{name}/work/`
    /// Mapped to `/work/` in the plugin's virtual filesystem (Phase 2+ when WASI lands).
    pub fn work_dir(name: &str) -> PathBuf {
        plugins_dir().join(name).join("work")
    }

    /// Pipeline grammar children directory: `~/.patina/pipeline/`
    /// Contains grammar-{lang}/ subdirectories with `child.wasm` + `child.toml`.
    pub fn pipeline_dir() -> PathBuf {
        patina_home().join("pipeline")
    }

    /// Secret grants file: `~/.patina/plugin-config/secret-grants.toml`
    /// Maps plugin names to allowed secret names for credential injection.
    pub fn secret_grants_path() -> PathBuf {
        patina_home()
            .join("plugin-config")
            .join("secret-grants.toml")
    }
}

/// Legacy plugin-path alias maintained during vocabulary migration.
pub mod plugin {
    pub use super::child::{children_dir, pipeline_dir, plugins_dir, secret_grants_path, work_dir};
}

/// User-level layer paths (~/.patina/layer/)
pub mod user_layer {
    use super::*;

    /// User-level beliefs directory: `~/.patina/layer/surface/beliefs/`
    pub fn beliefs_dir() -> PathBuf {
        patina_home().join("layer/surface/beliefs")
    }
}

/// Lake paths (DuckLake storage)
pub mod lakes {
    use super::*;

    /// Lakes directory: `~/.patina/lakes/`
    pub fn lakes_dir() -> PathBuf {
        patina_home().join("lakes")
    }

    /// Resolve a lake path by name.
    ///
    /// Returns the lake directory path if it exists and has a lake.toml.
    pub fn resolve_lake_path(name: &str) -> Result<PathBuf, String> {
        let lake_dir = lakes_dir().join(name);
        let lake_toml = lake_dir.join("lake.toml");

        if !lake_toml.exists() {
            return Err(format!(
                "lake '{}' not found (expected {} to exist)\n  \
                 Run: patina lake create {}",
                name,
                lake_toml.display(),
                name
            ));
        }

        Ok(lake_dir)
    }
}

/// Connection record paths (user-level, global scope in v1)
pub mod connections {
    use super::*;

    /// Connection configs directory: `~/.patina/connections/`
    pub fn connections_dir() -> PathBuf {
        patina_home().join("connections")
    }

    /// Individual connection config: `~/.patina/connections/{name}.toml`
    pub fn connection_path(name: &str) -> PathBuf {
        connections_dir().join(format!("{}.toml", name))
    }
}

/// Mother paths (cross-project graph and federation)
pub mod mother {
    use super::*;

    /// Mother data directory: `~/.patina/mother/`
    pub fn data_dir() -> PathBuf {
        patina_home().join("mother")
    }

    /// Relationship graph: `~/.patina/mother/graph.db`
    pub fn graph_db() -> PathBuf {
        data_dir().join("graph.db")
    }

    /// Knowledge-child runtime state: `~/.patina/mother/runtime.db`
    pub fn runtime_db() -> PathBuf {
        data_dir().join("runtime.db")
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
/// assert_eq!(db, Path::new("/home/user/myproject/.patina/local/data/patina.db"));
/// ```
pub mod project {
    use super::*;

    /// Project's patina directory: `.patina/`
    pub fn patina_dir(root: &Path) -> PathBuf {
        root.join(".patina")
    }

    /// Project config: `.patina/config.toml` (committed)
    pub fn config_path(root: &Path) -> PathBuf {
        root.join(".patina/config.toml")
    }

    /// Local state directory (gitignored): `.patina/local/`
    pub fn local_dir(root: &Path) -> PathBuf {
        root.join(".patina/local")
    }

    /// Derived data directory: `.patina/local/data/`
    pub fn data_dir(root: &Path) -> PathBuf {
        root.join(".patina/local/data")
    }

    /// Main SQLite database: `.patina/local/data/patina.db`
    pub fn db_path(root: &Path) -> PathBuf {
        root.join(".patina/local/data/patina.db")
    }

    /// Embedding indices: `.patina/local/data/embeddings/`
    pub fn embeddings_dir(root: &Path) -> PathBuf {
        root.join(".patina/local/data/embeddings")
    }

    /// Model-specific projections: `.patina/local/data/embeddings/{model}/projections/`
    pub fn model_projections_dir(root: &Path, model: &str) -> PathBuf {
        root.join(format!(
            ".patina/local/data/embeddings/{}/projections",
            model
        ))
    }

    /// Oxidize recipe: `.patina/oxidize.yaml` (committed)
    pub fn recipe_path(root: &Path) -> PathBuf {
        root.join(".patina/oxidize.yaml")
    }

    /// Version manifest: `.patina/versions.json` (committed)
    pub fn versions_path(root: &Path) -> PathBuf {
        root.join(".patina/versions.json")
    }

    /// Backup directory: `.patina/local/backups/`
    pub fn backups_dir(root: &Path) -> PathBuf {
        root.join(".patina/local/backups")
    }

    /// Installed schemas directory: `.patina/schemas/`
    pub fn schemas_dir(root: &Path) -> PathBuf {
        root.join(".patina/schemas")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mother_crate::secrets_paths as mother_paths;

    fn with_temp_patina_home<T>(f: impl FnOnce(PathBuf) -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::TempDir::new().unwrap();
        let expected_home = temp.path().join("patina-home");
        std::fs::create_dir_all(&expected_home).unwrap();
        let old = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &expected_home);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(expected_home)));
        match old {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn test_patina_home() {
        with_temp_patina_home(|expected_home| {
            let home = patina_home();
            assert_eq!(home, expected_home);
        });
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
    fn test_serve_paths() {
        with_temp_patina_home(|expected_home| {
            let run = serve::run_dir();
            assert_eq!(run, expected_home.join("run"));

            let sock = serve::socket_path();
            assert_eq!(sock, expected_home.join("run/serve.sock"));

            let token = serve::token_path();
            assert_eq!(token, expected_home.join("run/serve.token"));

            let pid = serve::pid_path();
            assert_eq!(pid, expected_home.join("run/mother.pid"));
        });
    }

    #[test]
    fn test_mother_paths_contract_user_level() {
        with_temp_patina_home(|expected_home| {
            assert_eq!(patina_home(), expected_home);
            assert_eq!(mother_paths::patina_home(), expected_home);

            assert_eq!(serve::run_dir(), mother_paths::serve::run_dir());
            assert_eq!(serve::socket_path(), mother_paths::serve::socket_path());
            assert_eq!(serve::token_path(), mother_paths::serve::token_path());

            assert_eq!(
                secrets::registry_path(),
                mother_paths::secrets::registry_path()
            );
            assert_eq!(secrets::vault_path(), mother_paths::secrets::vault_path());
            assert_eq!(
                secrets::recipient_path(),
                mother_paths::secrets::recipient_path()
            );
        });
    }

    #[test]
    fn test_mother_paths_contract_project_secrets() {
        let root = Path::new("/tmp/test-project");

        assert_eq!(
            secrets::project_registry_path(root),
            mother_paths::secrets::project_registry_path(root)
        );
        assert_eq!(
            secrets::project_vault_path(root),
            mother_paths::secrets::project_vault_path(root)
        );
        assert_eq!(
            secrets::project_recipients_path(root),
            mother_paths::secrets::project_recipients_path(root)
        );
    }

    #[test]
    fn test_user_layer_paths() {
        let beliefs = user_layer::beliefs_dir();
        assert!(beliefs.to_string_lossy().contains("layer/surface/beliefs"));
        assert!(beliefs.starts_with(patina_home()));
    }

    #[test]
    fn test_connections_paths() {
        with_temp_patina_home(|expected_home| {
            let dir = connections::connections_dir();
            assert_eq!(dir, expected_home.join("connections"));
            assert!(dir.starts_with(patina_home()));

            let path = connections::connection_path("github");
            assert_eq!(path, expected_home.join("connections/github.toml"));
            assert!(path.starts_with(connections::connections_dir()));
        });
    }

    #[test]
    fn test_project_paths() {
        let root = Path::new("/tmp/test-project");

        assert_eq!(
            project::patina_dir(root),
            PathBuf::from("/tmp/test-project/.patina")
        );
        assert_eq!(
            project::local_dir(root),
            PathBuf::from("/tmp/test-project/.patina/local")
        );
        assert_eq!(
            project::db_path(root),
            PathBuf::from("/tmp/test-project/.patina/local/data/patina.db")
        );
        assert_eq!(
            project::model_projections_dir(root, "e5-base-v2"),
            PathBuf::from("/tmp/test-project/.patina/local/data/embeddings/e5-base-v2/projections")
        );
    }
}
