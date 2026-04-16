//! Atlas command — deterministic visibility for specs + MCT surface.
//!
//! Atlas is a read-only lens:
//! - spec sprawl visibility (inventory + dependency edges)
//! - child/toy visibility for MCT review

mod internal;

use anyhow::Result;
use std::path::Path;

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

pub(crate) fn snapshot_json_for_root(root: &Path) -> Result<serde_json::Value> {
    let snapshot = internal::build_snapshot(root)?;
    serde_json::to_value(snapshot).map_err(Into::into)
}

pub(crate) fn dashboard_html_for_root(root: &Path, refresh_seconds: Option<u32>) -> Result<String> {
    let snapshot = internal::build_snapshot(root)?;
    internal::render_html_with_options(&snapshot, refresh_seconds)
}
