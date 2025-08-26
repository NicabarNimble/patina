pub mod ast_processor;
pub mod call_graph;
pub mod documentation;

// Re-export commonly used types
pub use ast_processor::{
    process_tree, BehavioralHint, CodeSearchFact, DocumentationFact, FingerprintFact, FunctionFact,
    ImportFact, ProcessingResult, TypeFact,
};

pub use call_graph::{CallRelation, CallType};

pub use documentation::Documentation;
