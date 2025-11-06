//! Symbolic reasoning layer using embedded Scryer Prolog.
//!
//! This module provides neuro-symbolic integration by embedding Prolog
//! directly in Rust, enabling symbolic reasoning over neural search results.

mod engine;

pub use engine::ReasoningEngine;
