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

pub(crate) use derive::derive_signals_json;
pub(super) use derive::{execute_derive, execute_derive_moments};
pub(crate) use functions::{callees_json, callers_json, functions_json};
pub(super) use functions::{execute_callees, execute_callers, execute_functions};
pub(super) use imports::{execute_importers, execute_imports};
pub(crate) use imports::{importers_json, imports_json};
pub(super) use inventory::{collect_inventory_json, execute_inventory};
pub(crate) use inventory::{inventory_all_repos_json, inventory_json};
pub(super) use search::execute_search;
pub(super) use util::truncate;
