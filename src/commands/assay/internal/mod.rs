//! Internal implementation for assay command
//!
//! Follows dependable-rust pattern: private modules with curated re-exports.

pub(crate) mod belief;
mod derive;
mod functions;
mod imports;
mod inventory;
pub(crate) mod query_prep;
pub(crate) mod search;
pub(crate) mod temporal;
pub(crate) mod util;

pub(super) use derive::{execute_derive, execute_derive_moments};
pub(super) use functions::{execute_callees, execute_callers, execute_functions};
pub(super) use imports::{execute_importers, execute_imports};
pub(super) use inventory::{collect_inventory_json, execute_inventory};
pub(super) use search::execute_search;
pub(super) use util::truncate;
pub(crate) use util::{collect_rows, serialize_result};
