//! Mother daemon for Patina
//!
//! Provides HTTP server for:
//! - Container queries to Mac mother
//! - Hot model caching (E5 embeddings)
//! - Cross-project knowledge access
//!
//! Design: Blocking HTTP with rouille (no async/tokio)

mod internal;

use anyhow::Result;

/// Options for the serve command
pub struct ServeOptions {
    /// Host to bind to (default: 127.0.0.1)
    pub host: String,
    /// Port to bind to (default: 50051)
    pub port: u16,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 50051,
        }
    }
}

/// Start the Mother daemon
pub fn execute(options: ServeOptions) -> Result<()> {
    internal::run_server(options)
}
