//! Atlas command — deterministic visibility for specs + MCT surface.
//!
//! Atlas is a read-only lens:
//! - spec sprawl visibility (inventory + dependency edges)
//! - child/toy visibility for MCT review

mod internal;

use anyhow::Result;

/// Options for atlas generation.
#[derive(Debug, Clone, Default)]
pub struct AtlasOptions {
    /// Output file path. If omitted for JSON mode, prints to stdout.
    pub output: Option<String>,
    /// Render standalone HTML dashboard.
    pub html: bool,
    /// Emit JSON snapshot.
    pub json: bool,
    /// Run local read-only dashboard server.
    pub serve: bool,
    /// Host to bind when --serve is set.
    pub host: String,
    /// Port to bind when --serve is set.
    pub port: u16,
}

/// Execute atlas command.
pub fn execute(options: AtlasOptions) -> Result<()> {
    internal::generate(options)
}
