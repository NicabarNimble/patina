// ============================================================================
// TYPE-SAFE DATABASE OPERATIONS WITH EMBEDDED SQLITE
// ============================================================================
//! Direct SQLite library integration for safe, high-performance data storage.
//!
//! This module replaces unsafe SQL string concatenation with:
//! - Prepared statements (no SQL injection possible)
//! - Bulk inserts via transactions
//! - Proper type preservation (arrays as JSON, booleans, JSON)
//! - Transaction support with automatic rollback
//!
//! Uses SqliteDatabase directly for type-safe database operations.

use crate::commands::scrape::code::types::CallGraphEntry;
use anyhow::{Context, Result};
use patina::db::SqliteDatabase;
use rusqlite::params;
use std::path::Path;

// Unified eventlog support
use crate::commands::scrape::database as unified_db;

// ============================================================================
// DOMAIN TYPES
// ============================================================================

/// Symbol extracted from source code
#[derive(Debug, Clone)]
pub struct CodeSymbol {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub context: String,
}

/// Function with rich metadata
#[derive(Debug, Clone)]
pub struct FunctionFact {
    pub file: String,
    pub name: String,
    pub takes_mut_self: bool,
    pub takes_mut_params: bool,
    pub returns_result: bool,
    pub returns_option: bool,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_public: bool,
    pub parameter_count: i32,
    pub generic_count: i32,
    pub parameters: Vec<String>, // Preserved as array!
    pub return_type: Option<String>,
}

/// Type definition
#[derive(Debug, Clone)]
pub struct TypeFact {
    pub file: String,
    pub name: String,
    pub definition: String,
    pub kind: String,
    pub visibility: String,
    pub usage_count: i32,
}

/// Import statement
#[derive(Debug, Clone)]
pub struct ImportFact {
    pub file: String,
    pub import_path: String,
    pub imported_names: Vec<String>, // Preserved as array!
    pub import_kind: String,
    pub line_number: i32,
}

// ============================================================================
// DATABASE CONNECTION
// ============================================================================

/// Database wrapper for code scraping operations
///
/// Follows the same pattern as other modules (embeddings, semantic_search):
/// - Owns SqliteDatabase
/// - Domain-specific methods for code facts
pub struct Database {
    db: SqliteDatabase,
}

impl Database {
    /// Open or create a SQLite database file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = SqliteDatabase::open(path).context("Failed to open SQLite database")?;
        Ok(Self { db })
    }

    /// Create an in-memory database for testing
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let db = SqliteDatabase::open_in_memory().context("Failed to create in-memory database")?;
        Ok(Self { db })
    }

    /// Initialize schema with proper types
    pub fn init_schema(&mut self) -> Result<()> {
        // Use a transaction for atomic schema creation
        let tx = self.db.connection_mut().transaction()?;

        // Code search table with full-text indexing
        tx.execute(
            "CREATE TABLE IF NOT EXISTS code_search (
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                kind TEXT,
                line INTEGER,
                context TEXT,
                PRIMARY KEY (path, name, line)
            )",
            [],
        )?;

        // Function facts with proper boolean types
        tx.execute(
            "CREATE TABLE IF NOT EXISTS function_facts (
                file TEXT NOT NULL,
                name TEXT NOT NULL,
                takes_mut_self BOOLEAN DEFAULT FALSE,
                takes_mut_params BOOLEAN DEFAULT FALSE,
                returns_result BOOLEAN DEFAULT FALSE,
                returns_option BOOLEAN DEFAULT FALSE,
                is_async BOOLEAN DEFAULT FALSE,
                is_unsafe BOOLEAN DEFAULT FALSE,
                is_public BOOLEAN DEFAULT FALSE,
                parameter_count INTEGER DEFAULT 0,
                generic_count INTEGER DEFAULT 0,
                parameters TEXT,  -- Comma-separated parameter names
                return_type TEXT,
                PRIMARY KEY (file, name)
            )",
            [],
        )?;

        // Type vocabulary
        tx.execute(
            "CREATE TABLE IF NOT EXISTS type_vocabulary (
                file TEXT NOT NULL,
                name TEXT NOT NULL,
                definition TEXT,
                kind TEXT,
                visibility TEXT,
                usage_count INTEGER DEFAULT 0,
                PRIMARY KEY (file, name)
            )",
            [],
        )?;

        // Import facts
        tx.execute(
            "CREATE TABLE IF NOT EXISTS import_facts (
                file TEXT NOT NULL,
                import_path TEXT NOT NULL,
                imported_names TEXT,  -- Comma-separated import names
                import_kind TEXT,
                line_number INTEGER,
                PRIMARY KEY (file, import_path)
            )",
            [],
        )?;

        // Constants table for macros, enum values, globals, statics
        tx.execute(
            "CREATE TABLE IF NOT EXISTS constant_facts (
                file TEXT NOT NULL,
                name TEXT NOT NULL,
                value TEXT,
                const_type TEXT NOT NULL,  -- macro, const, enum_value, static, global
                scope TEXT NOT NULL,        -- global, ClassName::, namespace::, module
                line INTEGER,
                PRIMARY KEY (file, name, scope)
            )",
            [],
        )?;

        // Members table for class/struct fields and methods
        tx.execute(
            "CREATE TABLE IF NOT EXISTS member_facts (
                file TEXT NOT NULL,
                container TEXT NOT NULL,   -- Class/struct/interface name
                name TEXT NOT NULL,
                member_type TEXT NOT NULL, -- field, method, property, constructor, destructor
                visibility TEXT,           -- public, private, protected, internal
                modifiers TEXT,              -- JSON array: [\"static\", \"const\", \"virtual\"]
                line INTEGER,
                PRIMARY KEY (file, container, name)
            )",
            [],
        )?;

        // Call graph
        tx.execute(
            "CREATE TABLE IF NOT EXISTS call_graph (
                caller TEXT NOT NULL,
                callee TEXT NOT NULL,
                file TEXT NOT NULL,
                call_type TEXT DEFAULT 'direct',
                line_number INTEGER,
                PRIMARY KEY (caller, callee, file, line_number)
            )",
            [],
        )?;

        // Index state for incremental updates
        tx.execute(
            "CREATE TABLE IF NOT EXISTS index_state (
                path TEXT PRIMARY KEY,
                mtime BIGINT NOT NULL,
                size BIGINT NOT NULL,
                hash TEXT,
                line_count INTEGER
            )",
            [],
        )?;

        // Module signals for structural oracle (assay derive)
        tx.execute(
            "CREATE TABLE IF NOT EXISTS module_signals (
                path TEXT PRIMARY KEY,
                is_used INTEGER,
                importer_count INTEGER,
                activity_level TEXT,
                last_commit_days INTEGER,
                top_contributors TEXT,
                centrality_score REAL,
                staleness_flags TEXT,
                computed_at TEXT
            )",
            [],
        )?;

        // Skipped files tracking
        tx.execute(
            "CREATE TABLE IF NOT EXISTS skipped_files (
                path TEXT PRIMARY KEY,
                reason TEXT,
                attempted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        tx.commit()?;
        Ok(())
    }
}

// ============================================================================
// BULK INSERT OPERATIONS
// ============================================================================

impl Database {
    /// Bulk insert code symbols using transaction
    pub fn insert_symbols(&self, symbols: &[CodeSymbol]) -> Result<usize> {
        if symbols.is_empty() {
            return Ok(0);
        }

        let conn = self.db.connection();
        let tx = conn.unchecked_transaction()?;

        for symbol in symbols {
            // 1. Insert into eventlog (source of truth)
            let event_data = serde_json::json!({
                "path": &symbol.path,
                "name": &symbol.name,
                "kind": &symbol.kind,
                "line": symbol.line,
                "context": &symbol.context,
            });

            unified_db::insert_event(
                &tx,
                "code.symbol",
                &chrono::Utc::now().to_rfc3339(),
                &format!("{}::{}", symbol.path, symbol.name),
                Some(&symbol.path),
                &event_data.to_string(),
            )?;

            // 2. Insert into materialized view (existing logic)
            tx.execute(
                "INSERT OR REPLACE INTO code_search (path, name, kind, line, context) VALUES (?, ?, ?, ?, ?)",
                params![
                    &symbol.path,
                    &symbol.name,
                    &symbol.kind,
                    symbol.line as i32,
                    &symbol.context,
                ],
            )?;
        }

        tx.commit()?;
        Ok(symbols.len())
    }

    /// Bulk insert function facts
    pub fn insert_functions(&self, functions: &[FunctionFact]) -> Result<usize> {
        if functions.is_empty() {
            return Ok(0);
        }

        let conn = self.db.connection();
        let tx = conn.unchecked_transaction()?;

        for func in functions {
            // 1. Insert into eventlog (source of truth)
            let event_data = serde_json::json!({
                "file": &func.file,
                "name": &func.name,
                "takes_mut_self": func.takes_mut_self,
                "takes_mut_params": func.takes_mut_params,
                "returns_result": func.returns_result,
                "returns_option": func.returns_option,
                "is_async": func.is_async,
                "is_unsafe": func.is_unsafe,
                "is_public": func.is_public,
                "parameter_count": func.parameter_count,
                "generic_count": func.generic_count,
                "parameters": &func.parameters,
                "return_type": &func.return_type,
            });

            unified_db::insert_event(
                &tx,
                "code.function",
                &chrono::Utc::now().to_rfc3339(),
                &format!("{}::{}", func.file, func.name),
                Some(&func.file),
                &event_data.to_string(),
            )?;

            // 2. Insert into materialized view (existing logic)
            let params_str = func.parameters.join(", ");
            tx.execute(
                "INSERT OR REPLACE INTO function_facts VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    &func.file,
                    &func.name,
                    func.takes_mut_self,
                    func.takes_mut_params,
                    func.returns_result,
                    func.returns_option,
                    func.is_async,
                    func.is_unsafe,
                    func.is_public,
                    func.parameter_count,
                    func.generic_count,
                    &params_str,
                    &func.return_type,
                ],
            )?;
        }

        tx.commit()?;
        Ok(functions.len())
    }

    /// Bulk insert type facts
    pub fn insert_types(&self, types: &[TypeFact]) -> Result<usize> {
        if types.is_empty() {
            return Ok(0);
        }

        let conn = self.db.connection();
        let tx = conn.unchecked_transaction()?;

        for type_fact in types {
            // 1. Insert into eventlog (source of truth)
            let event_data = serde_json::json!({
                "file": &type_fact.file,
                "name": &type_fact.name,
                "definition": &type_fact.definition,
                "kind": &type_fact.kind,
                "visibility": &type_fact.visibility,
                "usage_count": type_fact.usage_count,
            });

            // Map kind to specific event type for better queryability
            let event_type = match type_fact.kind.as_str() {
                "struct" => "code.struct",
                "enum" => "code.enum",
                "class" => "code.class",
                "interface" => "code.interface",
                "trait" => "code.trait",
                _ => "code.type",
            };

            unified_db::insert_event(
                &tx,
                event_type,
                &chrono::Utc::now().to_rfc3339(),
                &format!("{}::{}", type_fact.file, type_fact.name),
                Some(&type_fact.file),
                &event_data.to_string(),
            )?;

            // 2. Insert into materialized view (existing logic)
            tx.execute(
                "INSERT OR REPLACE INTO type_vocabulary (file, name, definition, kind, visibility, usage_count) VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    &type_fact.file,
                    &type_fact.name,
                    &type_fact.definition,
                    &type_fact.kind,
                    &type_fact.visibility,
                    type_fact.usage_count,
                ],
            )?;
        }

        tx.commit()?;
        Ok(types.len())
    }

    /// Bulk insert import facts
    pub fn insert_imports(&self, imports: &[ImportFact]) -> Result<usize> {
        if imports.is_empty() {
            return Ok(0);
        }

        let conn = self.db.connection();
        let tx = conn.unchecked_transaction()?;

        for import in imports {
            // 1. Insert into eventlog (source of truth)
            let event_data = serde_json::json!({
                "file": &import.file,
                "import_path": &import.import_path,
                "imported_names": &import.imported_names,
                "import_kind": &import.import_kind,
                "line_number": import.line_number,
            });

            unified_db::insert_event(
                &tx,
                "code.import",
                &chrono::Utc::now().to_rfc3339(),
                &format!("{}::{}", import.file, import.import_path),
                Some(&import.file),
                &event_data.to_string(),
            )?;

            // 2. Insert into materialized view (existing logic)
            let names_str = import.imported_names.join(", ");
            tx.execute(
                "INSERT OR REPLACE INTO import_facts (file, import_path, imported_names, import_kind, line_number) VALUES (?, ?, ?, ?, ?)",
                params![
                    &import.file,
                    &import.import_path,
                    &names_str,
                    &import.import_kind,
                    import.line_number,
                ],
            )?;
        }

        tx.commit()?;
        Ok(imports.len())
    }

    /// Bulk insert call graph edges
    pub fn insert_call_edges(&self, edges: &[CallGraphEntry]) -> Result<usize> {
        if edges.is_empty() {
            return Ok(0);
        }

        let conn = self.db.connection();
        let tx = conn.unchecked_transaction()?;

        for edge in edges {
            // 1. Insert into eventlog (source of truth)
            let event_data = serde_json::json!({
                "caller": &edge.caller,
                "callee": &edge.callee,
                "file": &edge.file,
                "call_type": edge.call_type.as_str(),
                "line_number": edge.line_number,
            });

            unified_db::insert_event(
                &tx,
                "code.call",
                &chrono::Utc::now().to_rfc3339(),
                &format!("{}::{}→{}", edge.file, edge.caller, edge.callee),
                Some(&edge.file),
                &event_data.to_string(),
            )?;

            // 2. Insert into materialized view (existing logic)
            tx.execute(
                "INSERT OR REPLACE INTO call_graph (caller, callee, file, call_type, line_number) VALUES (?, ?, ?, ?, ?)",
                params![
                    &edge.caller,
                    &edge.callee,
                    &edge.file,
                    edge.call_type.as_str(),
                    edge.line_number,
                ],
            )?;
        }

        tx.commit()?;
        Ok(edges.len())
    }

    /// Update index state for a file
    pub fn update_index_state(
        &self,
        path: &str,
        mtime: i64,
        size: i64,
        hash: Option<&str>,
        line_count: Option<i64>,
    ) -> Result<()> {
        self.db.connection().execute(
            "INSERT OR REPLACE INTO index_state (path, mtime, size, hash, line_count) VALUES (?, ?, ?, ?, ?)",
            params![path, mtime, size, hash, line_count],
        )?;
        Ok(())
    }

    /// Mark a file as skipped
    pub fn mark_skipped(&self, path: &str, reason: &str) -> Result<()> {
        self.db.connection().execute(
            "INSERT OR REPLACE INTO skipped_files (path, reason) VALUES (?, ?)",
            params![path, reason],
        )?;
        Ok(())
    }

    /// Bulk insert constants
    pub fn insert_constants(
        &self,
        constants: &[crate::commands::scrape::code::extracted_data::ConstantFact],
    ) -> Result<usize> {
        if constants.is_empty() {
            return Ok(0);
        }

        let conn = self.db.connection();
        let tx = conn.unchecked_transaction()?;

        for constant in constants {
            // 1. Insert into eventlog (source of truth)
            let event_data = serde_json::json!({
                "file": &constant.file,
                "name": &constant.name,
                "value": &constant.value,
                "const_type": &constant.const_type,
                "scope": &constant.scope,
                "line": constant.line,
            });

            unified_db::insert_event(
                &tx,
                "code.constant",
                &chrono::Utc::now().to_rfc3339(),
                &format!("{}::{}", constant.file, constant.name),
                Some(&constant.file),
                &event_data.to_string(),
            )?;

            // 2. Insert into materialized view (existing logic)
            tx.execute(
                "INSERT OR REPLACE INTO constant_facts VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    &constant.file,
                    &constant.name,
                    &constant.value,
                    &constant.const_type,
                    &constant.scope,
                    constant.line as i32,
                ],
            )?;
        }

        tx.commit()?;
        Ok(constants.len())
    }

    /// Bulk insert members
    pub fn insert_members(
        &self,
        members: &[crate::commands::scrape::code::extracted_data::MemberFact],
    ) -> Result<usize> {
        if members.is_empty() {
            return Ok(0);
        }

        let conn = self.db.connection();
        let tx = conn.unchecked_transaction()?;

        for member in members {
            // 1. Insert into eventlog (source of truth)
            let event_data = serde_json::json!({
                "file": &member.file,
                "container": &member.container,
                "name": &member.name,
                "member_type": &member.member_type,
                "visibility": &member.visibility,
                "modifiers": &member.modifiers,
                "line": member.line,
            });

            unified_db::insert_event(
                &tx,
                "code.member",
                &chrono::Utc::now().to_rfc3339(),
                &format!("{}::{}::{}", member.file, member.container, member.name),
                Some(&member.file),
                &event_data.to_string(),
            )?;

            // 2. Insert into materialized view (existing logic)
            let modifiers_json = serde_json::to_string(&member.modifiers)?;
            tx.execute(
                "INSERT OR REPLACE INTO member_facts VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![
                    &member.file,
                    &member.container,
                    &member.name,
                    &member.member_type,
                    &member.visibility,
                    &modifiers_json,
                    member.line as i32,
                ],
            )?;
        }

        tx.commit()?;
        Ok(members.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_operations() -> Result<()> {
        // Create in-memory database for testing
        let mut db = Database::open_in_memory()?;

        // Initialize unified eventlog schema (required for dual-write)
        let conn = db.db.connection();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS eventlog (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_file TEXT,
                data TEXT NOT NULL,
                CHECK(json_valid(data))
            );
            CREATE TABLE IF NOT EXISTS scrape_meta (
                key TEXT PRIMARY KEY,
                value TEXT
            );
            "#,
        )?;

        // Initialize code-specific schema
        db.init_schema()?;

        // Test inserting symbols
        let symbols = vec![CodeSymbol {
            path: "src/main.rs".to_string(),
            name: "main".to_string(),
            kind: "function".to_string(),
            line: 10,
            context: "fn main() {".to_string(),
        }];
        assert_eq!(db.insert_symbols(&symbols)?, 1);

        // Test inserting functions with arrays
        let functions = vec![FunctionFact {
            file: "src/lib.rs".to_string(),
            name: "process".to_string(),
            takes_mut_self: false,
            takes_mut_params: true,
            returns_result: true,
            returns_option: false,
            is_async: true,
            is_unsafe: false,
            is_public: true,
            parameter_count: 2,
            generic_count: 1,
            parameters: vec!["data: &mut [u8]".to_string(), "opts: Options".to_string()],
            return_type: Some("Result<()>".to_string()),
        }];
        assert_eq!(db.insert_functions(&functions)?, 1);

        Ok(())
    }
}
