pub mod ast_processor;
pub mod call_graph;
pub mod documentation;

// Re-export commonly used types
pub use ast_processor::{
    ProcessingResult,
    FunctionFact,
    TypeFact,
    ImportFact,
    BehavioralHint,
    FingerprintFact,
    DocumentationFact,
    CodeSearchFact,
    process_tree,
};

pub use call_graph::{
    CallRelation,
    CallType,
};

pub use documentation::{
    Documentation,
};