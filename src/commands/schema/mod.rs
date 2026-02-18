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
