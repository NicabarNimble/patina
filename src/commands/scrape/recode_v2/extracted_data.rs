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

    /// Add a code symbol
    pub fn add_symbol(&mut self, symbol: CodeSymbol) {
        self.symbols.push(symbol);
    }

    /// Add a function fact
    pub fn add_function(&mut self, function: FunctionFact) {
        self.functions.push(function);
    }

    /// Add a type fact
    pub fn add_type(&mut self, type_fact: TypeFact) {
        self.types.push(type_fact);
    }

    /// Add an import fact
    pub fn add_import(&mut self, import: ImportFact) {
        self.imports.push(import);
    }

    /// Add a call graph edge
    pub fn add_call_edge(&mut self, edge: CallEdge) {
        self.call_edges.push(edge);
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

/// Helper builders for common patterns

impl ExtractedData {
    /// Create a function fact with common defaults
    pub fn function_builder(file: &str, name: &str) -> FunctionFactBuilder {
        FunctionFactBuilder::new(file, name)
    }

    /// Create a type fact with common defaults
    pub fn type_builder(file: &str, name: &str) -> TypeFactBuilder {
        TypeFactBuilder::new(file, name)
    }
}

/// Builder for FunctionFact with fluent API
pub struct FunctionFactBuilder {
    fact: FunctionFact,
}

impl FunctionFactBuilder {
    pub fn new(file: &str, name: &str) -> Self {
        Self {
            fact: FunctionFact {
                file: file.to_string(),
                name: name.to_string(),
                takes_mut_self: false,
                takes_mut_params: false,
                returns_result: false,
                returns_option: false,
                is_async: false,
                is_unsafe: false,
                is_public: false,
                parameter_count: 0,
                generic_count: 0,
                parameters: Vec::new(),
                return_type: None,
            },
        }
    }

    pub fn takes_mut_self(mut self, value: bool) -> Self {
        self.fact.takes_mut_self = value;
        self
    }

    pub fn takes_mut_params(mut self, value: bool) -> Self {
        self.fact.takes_mut_params = value;
        self
    }

    pub fn returns_result(mut self, value: bool) -> Self {
        self.fact.returns_result = value;
        self
    }

    pub fn returns_option(mut self, value: bool) -> Self {
        self.fact.returns_option = value;
        self
    }

    pub fn is_async(mut self, value: bool) -> Self {
        self.fact.is_async = value;
        self
    }

    pub fn is_unsafe(mut self, value: bool) -> Self {
        self.fact.is_unsafe = value;
        self
    }

    pub fn is_public(mut self, value: bool) -> Self {
        self.fact.is_public = value;
        self
    }

    pub fn parameters(mut self, params: Vec<String>) -> Self {
        self.fact.parameter_count = params.len() as i32;
        self.fact.parameters = params;
        self
    }

    pub fn generic_count(mut self, count: i32) -> Self {
        self.fact.generic_count = count;
        self
    }

    pub fn return_type(mut self, return_type: Option<String>) -> Self {
        self.fact.return_type = return_type;
        self
    }

    pub fn build(self) -> FunctionFact {
        self.fact
    }
}

/// Builder for TypeFact with fluent API
pub struct TypeFactBuilder {
    fact: TypeFact,
}

impl TypeFactBuilder {
    pub fn new(file: &str, name: &str) -> Self {
        Self {
            fact: TypeFact {
                file: file.to_string(),
                name: name.to_string(),
                definition: String::new(),
                kind: String::new(),
                visibility: "private".to_string(),
                usage_count: 0,
            },
        }
    }

    pub fn definition(mut self, def: String) -> Self {
        self.fact.definition = def;
        self
    }

    pub fn kind(mut self, kind: &str) -> Self {
        self.fact.kind = kind.to_string();
        self
    }

    pub fn visibility(mut self, vis: &str) -> Self {
        self.fact.visibility = vis.to_string();
        self
    }

    pub fn usage_count(mut self, count: i32) -> Self {
        self.fact.usage_count = count;
        self
    }

    pub fn build(self) -> TypeFact {
        self.fact
    }
}
