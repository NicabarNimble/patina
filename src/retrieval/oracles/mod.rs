//! Concrete oracle implementations
//!
//! These wrap existing scry/persona functions as Oracle trait implementations.
//! Not exposed publicly - QueryEngine constructs them internally.

mod lexical;
mod persona;
mod semantic;

pub(crate) use lexical::LexicalOracle;
pub(crate) use persona::PersonaOracle;
pub(crate) use semantic::SemanticOracle;
