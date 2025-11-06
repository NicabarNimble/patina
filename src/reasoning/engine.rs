//! Embedded Prolog reasoning engine using Scryer Prolog.
//!
//! Provides symbolic reasoning capabilities by embedding Scryer Prolog as a library,
//! enabling automatic validation and confidence calculation without shell overhead.

use anyhow::{anyhow, Context, Result};
use scryer_prolog::{LeafAnswer, Machine, MachineBuilder, Term};

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

    /// Calculate initial confidence score based on evidence count.
    ///
    /// Queries the Prolog rule: `query_initial_confidence(EvidenceCount, Confidence)`
    ///
    /// # Arguments
    /// * `evidence_count` - Number of supporting observations
    ///
    /// # Returns
    /// Confidence score between 0.3 and 0.95 according to Prolog rules
    pub fn calculate_confidence(&mut self, evidence_count: usize) -> Result<f32> {
        // Query Prolog: query_initial_confidence(3, C).
        let query = format!("query_initial_confidence({}, C).", evidence_count);
        let mut results = self.machine.run_query(&query);

        // Get first solution
        match results.next() {
            Some(Ok(LeafAnswer::LeafAnswer { bindings, .. })) => {
                // Extract confidence value from variable C
                if let Some(term) = bindings.get("C") {
                    match term {
                        Term::Float(conf) => Ok(*conf as f32),
                        Term::Rational(r) => {
                            // Convert rational to float via approximation
                            Ok(r.to_f64().value() as f32)
                        }
                        Term::Integer(i) => {
                            // Convert integer to float via approximation
                            Ok(i.to_f64().value() as f32)
                        }
                        _ => Err(anyhow!(
                            "Expected numeric confidence value, got: {:?}",
                            term
                        )),
                    }
                } else {
                    Err(anyhow!("No binding for variable C in query result"))
                }
            }
            Some(Ok(LeafAnswer::False)) => {
                Err(anyhow!("Prolog query returned false (no solutions)"))
            }
            Some(Ok(LeafAnswer::Exception(term))) => {
                Err(anyhow!("Prolog exception: {:?}", term))
            }
            Some(Err(term)) => Err(anyhow!("Prolog error: {:?}", term)),
            None => Err(anyhow!("No results from Prolog query")),
            _ => Err(anyhow!("Unexpected Prolog result")),
        }
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

    #[test]
    fn test_confidence_calculation_no_evidence() {
        let mut engine = ReasoningEngine::new().unwrap();
        let confidence = engine.calculate_confidence(0).unwrap();
        assert!((confidence - 0.5).abs() < 0.01, "Expected 0.5, got {}", confidence);
    }

    #[test]
    fn test_confidence_calculation_with_evidence() {
        let mut engine = ReasoningEngine::new().unwrap();

        // 1 evidence: 0.5 + (1 * 0.15) = 0.65
        let conf1 = engine.calculate_confidence(1).unwrap();
        assert!((conf1 - 0.65).abs() < 0.01, "Expected 0.65, got {}", conf1);

        // 2 evidence: 0.5 + (2 * 0.15) = 0.80
        let conf2 = engine.calculate_confidence(2).unwrap();
        assert!((conf2 - 0.80).abs() < 0.01, "Expected 0.80, got {}", conf2);

        // 3+ evidence: min(0.85, 0.5 + (3 * 0.1)) = 0.80
        let conf3 = engine.calculate_confidence(3).unwrap();
        assert!((conf3 - 0.80).abs() < 0.01, "Expected 0.80, got {}", conf3);

        // 5 evidence: min(0.85, 0.5 + (5 * 0.1)) = 0.85 (capped)
        let conf5 = engine.calculate_confidence(5).unwrap();
        assert!((conf5 - 0.85).abs() < 0.01, "Expected 0.85, got {}", conf5);
    }
}
