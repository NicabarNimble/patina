//! Internal implementation for assay command
//!
//! Follows dependable-rust pattern: private modules with curated re-exports.

mod util;

pub(super) use util::truncate;
