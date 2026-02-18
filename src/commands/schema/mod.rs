//! Fact schema management — install, list, and query schema packages.
//!
//! Schemas are WIT-based data contracts that describe fact types (records,
//! enums) with companion TOML metadata (event types, embedding config,
//! index hints). Installed schemas live under `.patina/schemas/<name>/`.
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API for schema operations
//! - Internal implementation: all logic in internal.rs

mod internal;

/// Schema CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum SchemaCommands {
    /// Install a schema package from a local path
    Install {
        /// Path to schema package directory (contains *.wit + schema.toml)
        path: String,
    },

    /// List installed schemas
    List {
        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Show details of an installed schema
    Show {
        /// Schema name (e.g., "forge")
        name: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Scaffold a new schema package (WIT + schema.toml)
    New {
        /// Schema name (lowercase, e.g., "workspace")
        name: String,

        /// Schema version (default: 1.0.0)
        #[arg(long, default_value = "1.0.0")]
        version: String,

        /// Schema description
        #[arg(long, default_value = "")]
        description: String,

        /// Fact names (comma-separated, e.g., "item,event")
        #[arg(long)]
        facts: Option<String>,
    },

    /// Generate code from installed schemas
    Generate {
        /// Generate Rust types with serde derives
        #[arg(long)]
        types: bool,

        /// Generate SQLite migration DDL
        #[arg(long)]
        migrations: bool,

        /// Generate embedding offset + corpus config
        #[arg(long)]
        embeddings: bool,

        /// Schema name (generates for all if omitted)
        #[arg(long)]
        schema: Option<String>,
    },
}

/// Install a schema package from a local path to .patina/schemas/<name>/
pub fn install(path: &str) -> anyhow::Result<()> {
    internal::install_schema(path)
}

/// List installed schemas
pub fn list(json: bool) -> anyhow::Result<()> {
    internal::list_schemas(json)
}

/// Show details of an installed schema
pub fn show(name: &str, json: bool) -> anyhow::Result<()> {
    internal::show_schema(name, json)
}

/// Scaffold a new schema package under wit/schema/<name>/
pub fn new_schema(
    name: &str,
    version: &str,
    description: &str,
    facts: Option<&str>,
) -> anyhow::Result<()> {
    internal::new_schema(name, version, description, facts)
}

/// Generate code from installed schemas
pub fn generate(
    types: bool,
    migrations: bool,
    embeddings: bool,
    schema: Option<&str>,
) -> anyhow::Result<()> {
    internal::generate(types, migrations, embeddings, schema)
}
