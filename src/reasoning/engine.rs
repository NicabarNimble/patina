//! Embedded Prolog reasoning engine using Scryer Prolog.
//!
//! Provides symbolic reasoning capabilities by embedding Scryer Prolog as a library,
//! enabling automatic validation and confidence calculation without shell overhead.

use anyhow::{anyhow, Result};
use scryer_prolog::{LeafAnswer, Machine, MachineBuilder, Term};

/// An observation with its semantic similarity score from vector search
#[derive(Debug, Clone)]
pub struct ScoredObservation {
    pub id: String,
    pub observation_type: String,
    pub content: String,
    pub similarity: f32,
    pub reliability: f32,
    pub source_type: String,
}

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

    /// Inject semantic search results as Prolog facts for reasoning.
    ///
    /// Converts observations to Prolog facts in the format:
    /// `observation(Id, Type, Content, Similarity, Reliability, SourceType).`
    ///
    /// This enables symbolic reasoning over neural search results.
    ///
    /// # Arguments
    /// * `observations` - Slice of observations with similarity scores from vector search
    pub fn load_observations(&mut self, observations: &[ScoredObservation]) -> Result<()> {
        // Convert observations to Prolog facts
        let facts: Vec<String> = observations
            .iter()
            .map(|obs| {
                format!(
                    "observation('{}', '{}', '{}', {}, {}, '{}').",
                    escape_prolog_string(&obs.id),
                    escape_prolog_string(&obs.observation_type),
                    escape_prolog_string(&obs.content),
                    obs.similarity,
                    obs.reliability,
                    escape_prolog_string(&obs.source_type)
                )
            })
            .collect();

        // Join facts with newlines
        let prolog_program = facts.join("\n");

        // Load facts into Prolog machine
        self.machine
            .consult_module_string("observations", &prolog_program);

        Ok(())
    }
}

/// Escape a string for safe use in Prolog fact literals
fn escape_prolog_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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

    #[test]
    fn test_load_observations() {
        let mut engine = ReasoningEngine::new().unwrap();

        // Create test observations
        let observations = vec![
            ScoredObservation {
                id: "obs_1".to_string(),
                observation_type: "pattern".to_string(),
                content: "Always run security audits".to_string(),
                similarity: 0.85,
                reliability: 0.85,
                source_type: "session".to_string(),
            },
            ScoredObservation {
                id: "obs_2".to_string(),
                observation_type: "decision".to_string(),
                content: "Use Rust for core logic".to_string(),
                similarity: 0.72,
                reliability: 0.70,
                source_type: "commit".to_string(),
            },
        ];

        // Load observations
        engine.load_observations(&observations).unwrap();

        // Query to verify facts were loaded
        let query = "observation(Id, Type, Content, Sim, Rel, Source).";
        let mut results = engine.machine.run_query(query);

        // Should get results
        let first = results.next();
        assert!(first.is_some(), "Should have at least one observation");

        match first.unwrap() {
            Ok(LeafAnswer::LeafAnswer { bindings, .. }) => {
                // Verify we got an observation
                assert!(bindings.contains_key("Id"));
                assert!(bindings.contains_key("Type"));
            }
            _ => panic!("Expected successful query result"),
        }
    }

    #[test]
    fn test_escape_prolog_string() {
        assert_eq!(escape_prolog_string("hello"), "hello");
        assert_eq!(escape_prolog_string("hello'world"), "hello\\'world");
        assert_eq!(escape_prolog_string("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_prolog_string("back\\slash"), "back\\\\slash");
    }
}
