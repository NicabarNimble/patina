// ============================================================================
// COMMON DATA STRUCTURES FOR LANGUAGE PROCESSORS
// ============================================================================
//! Shared data structures that all language processors return.
//!
//! This replaces SQL string generation with type-safe structs that can be
//! directly inserted into the database using prepared statements.

use super::database::{CodeSymbol, FunctionFact, ImportFact, TypeFact};
use super::types::CallGraphEntry;

/// Represents a constant, macro, enum value, or static variable
#[derive(Debug, Clone)]
pub struct ConstantFact {
    pub file: String,
    pub name: String,
    pub value: Option<String>,
    pub const_type: String, // "macro", "const", "enum_value", "static", "global"
    pub scope: String,      // "global", "ClassName::", "namespace::", "module"
    pub line: usize,
}

/// Represents a class/struct member (field or method)
#[derive(Debug, Clone)]
pub struct MemberFact {
    pub file: String,
    pub container: String, // Class/struct/interface name
    pub name: String,
    pub member_type: String, // "field", "method", "property", "constructor", "destructor"
    pub visibility: String,  // "public", "private", "protected", "internal"
    pub modifiers: Vec<String>, // ["static", "const", "virtual", "override", "abstract"]
    pub line: usize,
}

/// Container for all data extracted from a source file
#[derive(Debug, Default)]
pub struct ExtractedData {
    pub symbols: Vec<CodeSymbol>,
    pub functions: Vec<FunctionFact>,
    pub types: Vec<TypeFact>,
    pub imports: Vec<ImportFact>,
    pub call_edges: Vec<CallGraphEntry>,
    pub constants: Vec<ConstantFact>,
    pub members: Vec<MemberFact>,
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
    pub fn add_call_edge(&mut self, edge: CallGraphEntry) {
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

    /// Add a constant fact (with deduplication)
    pub fn add_constant(&mut self, constant: ConstantFact) {
        // Check if we already have this constant (same file, name, and scope)
        let already_exists = self.constants.iter().any(|c| {
            c.file == constant.file && c.name == constant.name && c.scope == constant.scope
        });

        if !already_exists {
            self.constants.push(constant);
        }
    }

    /// Add a member fact (with deduplication)
    pub fn add_member(&mut self, member: MemberFact) {
        // Check if we already have this member (same container and name)
        let already_exists = self.members.iter().any(|m| {
            m.file == member.file && m.container == member.container && m.name == member.name
        });

        if !already_exists {
            self.members.push(member);
        }
    }

    /// Merge another ExtractedData into this one
    pub fn merge(&mut self, other: ExtractedData) {
        self.symbols.extend(other.symbols);
        self.functions.extend(other.functions);
        self.types.extend(other.types);
        self.imports.extend(other.imports);
        self.call_edges.extend(other.call_edges);
        self.constants.extend(other.constants);
        self.members.extend(other.members);
    }
}
