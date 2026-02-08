//! Oracle implementations
//!
//! After the semantic-structural split, only the semantic oracle remains here.
//! Factual oracles (lexical, temporal, persona, belief) moved to assay.
//! The old oracle files are preserved for reference but no longer compiled.

mod semantic;

pub(crate) use semantic::SemanticOracle;
