//! Internal implementation for assay command
//!
//! Follows dependable-rust pattern: private modules with curated re-exports.

mod derive;
mod functions;
mod imports;
mod inventory;
mod util;

pub(super) use derive::execute_derive;
pub(super) use functions::{execute_callees, execute_callers, execute_functions};
pub(super) use imports::{execute_importers, execute_imports};
pub(super) use inventory::{collect_inventory_json, execute_inventory};
pub(super) use util::truncate;
