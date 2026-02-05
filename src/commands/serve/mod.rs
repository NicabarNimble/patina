//! Serve command — DEPRECATED, use `patina mother start`
//!
//! This module exists only to preserve the `mod serve` declaration in commands/mod.rs.
//! The real implementation lives in `commands::mother::daemon`.
//!
//! Deprecation timeline (GH #85):
//! - v0.11.x: hidden from help, prints warning on use
//! - v0.12.0+: module removed entirely
