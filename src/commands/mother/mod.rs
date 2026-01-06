//! Mother command - Cross-project graph management
//!
//! Manages the relationship graph between projects and reference repos.
//! The graph enables smart routing for cross-project queries.
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> anyhow::Result<()> {
//! // Sync graph from registry
//! // patina mother sync
//!
//! // Show graph state
//! // patina mother graph
//!
//! // Add a relationship
//! // patina mother link patina dojo TESTS_WITH --evidence "benchmark subject"
//!
//! // Remove a relationship
//! // patina mother unlink patina dojo TESTS_WITH
//! # Ok(())
//! # }
//! ```

pub(crate) mod internal;

use anyhow::Result;

/// Mother CLI subcommands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum MotherCommands {
    /// Sync graph nodes from registry
    ///
    /// Creates nodes for all projects and repos in ~/.patina/registry.yaml.
    /// Run this after adding new repos with `patina repo add`.
    Sync,

    /// Show graph state
    ///
    /// Displays all nodes and edges in the relationship graph.
    Graph {
        /// Show only nodes
        #[arg(long)]
        nodes: bool,

        /// Show only edges
        #[arg(long)]
        edges: bool,
    },

    /// Add a relationship between nodes
    ///
    /// Creates a directed edge from one node to another.
    /// Edge types: USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN
    Link {
        /// Source node (e.g., "patina")
        from: String,

        /// Target node (e.g., "dojo")
        to: String,

        /// Relationship type (e.g., "TESTS_WITH")
        edge_type: String,

        /// Optional evidence/reason for this relationship
        #[arg(long)]
        evidence: Option<String>,
    },

    /// Remove a relationship
    Unlink {
        /// Source node
        from: String,

        /// Target node
        to: String,

        /// Relationship type
        edge_type: String,
    },

    /// Show edge usage statistics
    ///
    /// Displays usage statistics for all edges: how often each edge
    /// was used in graph routing, and how often it led to useful results.
    Stats,
}

/// Execute mother command from CLI
pub fn execute_cli(command: Option<MotherCommands>) -> Result<()> {
    let cmd = command.unwrap_or(MotherCommands::Graph {
        nodes: false,
        edges: false,
    });

    match cmd {
        MotherCommands::Sync => sync(),
        MotherCommands::Graph { nodes, edges } => graph(nodes, edges),
        MotherCommands::Link {
            from,
            to,
            edge_type,
            evidence,
        } => link(&from, &to, &edge_type, evidence.as_deref()),
        MotherCommands::Unlink {
            from,
            to,
            edge_type,
        } => unlink(&from, &to, &edge_type),
        MotherCommands::Stats => stats(),
    }
}

/// Show edge usage statistics
pub fn stats() -> Result<()> {
    internal::show_stats()
}

/// Sync graph nodes from registry
pub fn sync() -> Result<()> {
    internal::sync_from_registry()
}

/// Show graph state
pub fn graph(nodes_only: bool, edges_only: bool) -> Result<()> {
    internal::show_graph(nodes_only, edges_only)
}

/// Add a relationship
pub fn link(from: &str, to: &str, edge_type: &str, evidence: Option<&str>) -> Result<()> {
    internal::add_link(from, to, edge_type, evidence)
}

/// Remove a relationship
pub fn unlink(from: &str, to: &str, edge_type: &str) -> Result<()> {
    internal::remove_link(from, to, edge_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mother_command_variants() {
        let sync = MotherCommands::Sync;
        assert!(matches!(sync, MotherCommands::Sync));

        let graph = MotherCommands::Graph {
            nodes: true,
            edges: false,
        };
        assert!(matches!(graph, MotherCommands::Graph { .. }));
    }
}
