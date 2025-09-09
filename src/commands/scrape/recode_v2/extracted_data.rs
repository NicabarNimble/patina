// ============================================================================
// COMMON DATA STRUCTURES FOR LANGUAGE PROCESSORS
// ============================================================================
//! Shared data structures that all language processors return.
//!
//! This replaces SQL string generation with type-safe structs that can be
//! directly inserted into the database using prepared statements.

use super::database::{CallEdge, CodeSymbol, FunctionFact, ImportFact, TypeFact};

/// Container for all data extracted from a source file
#[derive(Debug, Default)]
pub struct ExtractedData {
    pub symbols: Vec<CodeSymbol>,
    pub functions: Vec<FunctionFact>,
    pub types: Vec<TypeFact>,
    pub imports: Vec<ImportFact>,
    pub call_edges: Vec<CallEdge>,
}

impl ExtractedData {
    /// Create a new empty container
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a code symbol (with deduplication)
    pub fn add_symbol(&mut self, symbol: CodeSymbol) {
        // Check if we already have this exact symbol (same path, name, and line)
        let already_exists = self
            .symbols
            .iter()
            .any(|s| s.path == symbol.path && s.name == symbol.name && s.line == symbol.line);

        if !already_exists {
            self.symbols.push(symbol);
        }
    }

    /// Add a function fact (with deduplication)
    pub fn add_function(&mut self, function: FunctionFact) {
        // Check if we already have this function (same file and name)
        let already_exists = self
            .functions
            .iter()
            .any(|f| f.file == function.file && f.name == function.name);

        if !already_exists {
            self.functions.push(function);
        }
    }

    /// Add a type fact (with deduplication)
    pub fn add_type(&mut self, type_fact: TypeFact) {
        // Check if we already have this type (same file and name)
        let already_exists = self
            .types
            .iter()
            .any(|t| t.file == type_fact.file && t.name == type_fact.name);

        if !already_exists {
            self.types.push(type_fact);
        }
    }

    /// Add an import fact (with deduplication)
    pub fn add_import(&mut self, import: ImportFact) {
        // Check if we already have this import (same file and import_path)
        let already_exists = self
            .imports
            .iter()
            .any(|i| i.file == import.file && i.import_path == import.import_path);

        if !already_exists {
            self.imports.push(import);
        }
    }

    /// Add a call graph edge (with deduplication)
    pub fn add_call_edge(&mut self, edge: CallEdge) {
        // Check if we already have this edge (same caller, callee, file, and line)
        let already_exists = self.call_edges.iter().any(|e| {
            e.caller == edge.caller
                && e.callee == edge.callee
                && e.file == edge.file
                && e.line_number == edge.line_number
        });

        if !already_exists {
            self.call_edges.push(edge);
        }
    }

    /// Merge another ExtractedData into this one
    pub fn merge(&mut self, other: ExtractedData) {
        self.symbols.extend(other.symbols);
        self.functions.extend(other.functions);
        self.types.extend(other.types);
        self.imports.extend(other.imports);
        self.call_edges.extend(other.call_edges);
    }

    /// Get total count of all extracted items
    pub fn total_count(&self) -> usize {
        self.symbols.len()
            + self.functions.len()
            + self.types.len()
            + self.imports.len()
            + self.call_edges.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
            && self.functions.is_empty()
            && self.types.is_empty()
            && self.imports.is_empty()
            && self.call_edges.is_empty()
    }
}
