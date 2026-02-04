//! Concrete oracle implementations
//!
//! These wrap existing scry/persona functions as Oracle trait implementations.
//! Not exposed publicly - QueryEngine constructs them internally.
//!
//! Note: StructuralOracle was removed. Structural signals (assay derive) are
//! priors/orientation, not relevance signals. See spec-work-deferred.md for
//! rebuild plan when query-type routing is implemented.

mod belief;
mod lexical;
mod persona;
mod semantic;
mod temporal;

pub(crate) use belief::BeliefOracle;
pub(crate) use lexical::LexicalOracle;
pub(crate) use persona::PersonaOracle;
pub(crate) use semantic::SemanticOracle;
pub(crate) use temporal::TemporalOracle;
