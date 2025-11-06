//! Embedded Prolog reasoning engine using Scryer Prolog.
//!
//! Provides symbolic reasoning capabilities by embedding Scryer Prolog as a library,
//! enabling automatic validation and confidence calculation without shell overhead.

use anyhow::Result;
use scryer_prolog::{Machine, MachineBuilder};

/// Embedded Prolog reasoning engine.
///
/// Wraps a Scryer Prolog Machine to provide confidence calculation,
/// belief validation, and symbolic reasoning over semantic search results.
pub struct ReasoningEngine {
    machine: Machine,
}

impl ReasoningEngine {
    /// Create a new reasoning engine with confidence rules loaded.
    ///
    /// Loads the confidence-rules.pl file from .patina/ directory and
    /// initializes the Prolog machine for queries.
    pub fn new() -> Result<Self> {
        let mut machine = MachineBuilder::default().build();

        // Load confidence rules from .patina directory
        let rules = include_str!("../../.patina/confidence-rules.pl");
        machine.load_module_string("confidence", rules);

        Ok(Self { machine })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ReasoningEngine::new();
        assert!(engine.is_ok());
    }
}
