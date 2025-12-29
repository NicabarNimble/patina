//! Internal implementation for assay command
//!
//! Follows dependable-rust pattern: private modules with curated re-exports.

mod imports;
mod inventory;
mod util;

pub(super) use imports::{execute_importers, execute_imports};
pub(super) use inventory::{collect_inventory_json, execute_inventory};
pub(super) use util::truncate;
