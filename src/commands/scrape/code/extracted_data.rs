// ============================================================================
// COMMON DATA STRUCTURES FOR LANGUAGE PROCESSORS
// ============================================================================
//! Shared data structures that all language processors return.
//!
//! This replaces SQL string generation with type-safe structs that can be
//! directly inserted into the database using prepared statements.
//!
//! ## Polymorphic Pipeline Contract
//!
//! `ExtractedPayload` is the tagged union that plugins return. The host
//! deserializes by `kind` and routes to the correct insert path.
//! This is a **bridge type** — will be superseded by schema-generated
//! variants once [[fact-schema-registry]] lands.

use super::database::{CodeSymbol, FunctionFact, ImportFact, TypeFact};
use super::types::CallGraphEntry;

use crate::commands::scrape::events;

/// Pipeline-facing issue type. Uses scrape::events domain types.
/// Will be replaced by schema-generated type in [[fact-schema-registry]].
pub type ExtractedIssue = events::Issue;

/// Pipeline-facing pull request type. Uses scrape::events domain types.
/// Will be replaced by schema-generated type in [[fact-schema-registry]].
pub type ExtractedPullRequest = events::PullRequest;

/// Pipeline plugins return JSON matching one of these variants.
/// If no `kind` field is present, defaults to Code (backward compat).
///
/// Bridge type: will be superseded by schema-generated variants
/// once [[fact-schema-registry]] lands. Keep `#[non_exhaustive]`.
#[derive(Debug, serde::Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind")]
pub enum ExtractedPayload {
    #[serde(rename = "code")]
    Code(ExtractedData),

    #[serde(rename = "issue")]
    Issue(ExtractedIssue),

    #[serde(rename = "pull-request")]
    PullRequest(ExtractedPullRequest),
}

/// Represents a constant, macro, enum value, or static variable
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstantFact {
    pub file: String,
    pub name: String,
    pub value: Option<String>,
    pub const_type: String, // "macro", "const", "enum_value", "static", "global"
    pub scope: String,      // "global", "ClassName::", "namespace::", "module"
    pub line: usize,
}

/// Represents a class/struct member (field or method)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_code_payload_with_kind() {
        let json = r#"{
            "kind": "code",
            "symbols": [],
            "functions": [],
            "types": [],
            "imports": [],
            "call_edges": [],
            "constants": [],
            "members": []
        }"#;
        let payload: ExtractedPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(payload, ExtractedPayload::Code(_)));
    }

    #[test]
    fn deserialize_code_payload_without_kind_fallback() {
        // Backward compat: no "kind" field → must fail ExtractedPayload,
        // caller wraps as Code
        let json = r#"{
            "symbols": [],
            "functions": [],
            "types": [],
            "imports": [],
            "call_edges": [],
            "constants": [],
            "members": []
        }"#;
        // ExtractedPayload requires "kind" — this should fail
        assert!(serde_json::from_str::<ExtractedPayload>(json).is_err());
        // But ExtractedData should succeed
        let data: ExtractedData = serde_json::from_str(json).unwrap();
        assert!(data.symbols.is_empty());
    }

    #[test]
    fn deserialize_issue_payload() {
        let json = r#"{
            "kind": "issue",
            "number": 42,
            "title": "Fix the bug",
            "body": "Something is broken",
            "state": "open",
            "author": "alice",
            "labels": ["bug"],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-02T00:00:00Z",
            "url": "https://github.com/owner/repo/issues/42"
        }"#;
        let payload: ExtractedPayload = serde_json::from_str(json).unwrap();
        match payload {
            ExtractedPayload::Issue(issue) => {
                assert_eq!(issue.number, 42);
                assert_eq!(issue.title, "Fix the bug");
                assert_eq!(issue.labels, vec!["bug"]);
            }
            _ => panic!("expected Issue variant"),
        }
    }

    #[test]
    fn deserialize_pull_request_payload() {
        let json = r#"{
            "kind": "pull-request",
            "number": 99,
            "title": "Add feature",
            "body": "Implements the thing",
            "state": "merged",
            "author": "bob",
            "labels": ["enhancement"],
            "created_at": "2026-02-01T00:00:00Z",
            "merged_at": "2026-02-05T00:00:00Z",
            "url": "https://github.com/owner/repo/pull/99",
            "linked_issues": [42],
            "comments": [],
            "approvals": 2
        }"#;
        let payload: ExtractedPayload = serde_json::from_str(json).unwrap();
        match payload {
            ExtractedPayload::PullRequest(pr) => {
                assert_eq!(pr.number, 99);
                assert_eq!(pr.title, "Add feature");
                assert_eq!(pr.linked_issues, vec![42]);
                assert_eq!(pr.approvals, 2);
            }
            _ => panic!("expected PullRequest variant"),
        }
    }

    #[test]
    fn deserialize_unknown_kind_fails() {
        let json = r#"{
            "kind": "email",
            "subject": "Hello"
        }"#;
        assert!(serde_json::from_str::<ExtractedPayload>(json).is_err());
    }
}
