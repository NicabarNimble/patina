use anyhow::{Context, Result};
use std::process::Command;
use std::path::Path;
use super::{KnowledgeStore, Symbol};
use crate::semantic::extractor::*;

/// DuckDB implementation of the knowledge store
pub struct DuckDbStore {
    db_path: String,
}

impl DuckDbStore {
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }
    
    /// Execute SQL and return output as string
    fn execute_sql(&self, sql: &str) -> Result<String> {
        let output = Command::new("duckdb")
            .arg(&self.db_path)
            .arg("-csv")
            .arg("-c")
            .arg(sql)
            .output()
            .context("Failed to execute DuckDB query")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("DuckDB query failed: {}", stderr);
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    /// Execute SQL via stdin (for large queries)
    fn execute_sql_stdin(&self, sql: &str) -> Result<()> {
        use std::io::Write;
        use std::process::Stdio;
        
        let mut child = Command::new("duckdb")
            .arg(&self.db_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start DuckDB")?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(sql.as_bytes())
                .context("Failed to write SQL to DuckDB")?;
        }
        
        let output = child.wait_with_output()
            .context("Failed to execute DuckDB")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("DuckDB execution failed: {}", stderr);
        }
        
        Ok(())
    }
}

impl KnowledgeStore for DuckDbStore {
    fn initialize(&self) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = Path::new(&self.db_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Initialize schema (delegating to fingerprint module for now)
        let schema = crate::semantic::fingerprint::generate_schema();
        
        let init_sql = format!(
            "ATTACH '{}' AS knowledge (BLOCK_SIZE 16384);\nUSE knowledge;\n{}",
            self.db_path, schema
        );
        
        self.execute_sql_stdin(&init_sql)?;
        Ok(())
    }
    
    fn store_results(&self, results: &ProcessingResult, file_path: &str) -> Result<()> {
        let mut sql = String::from("BEGIN TRANSACTION;\n");
        
        // Store functions - note the order matches the actual table schema
        for func in &results.functions {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO function_facts (file, name, takes_mut_self, takes_mut_params, returns_result, returns_option, is_async, is_unsafe, is_public, parameter_count, generic_count, parameters, return_type) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', '{}');\n",
                func.file, func.name, 
                func.takes_mut_self, func.takes_mut_params,
                func.returns_result, func.returns_option,
                func.is_async, func.is_unsafe, func.is_public,
                func.parameter_count, func.generics_count,
                func.parameters.replace('\'', "''"),
                func.return_type.replace('\'', "''")
            ));
        }
        
        // Store code_search entries
        for search in &results.code_search {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO code_search (path, name, signature, context) VALUES ('{}', '{}', '{}', '{}');\n",
                search.file, search.name,
                search.signature.replace('\'', "''"),
                search.context.replace('\'', "''")
            ));
        }
        
        // Store documentation
        for doc in &results.documentation {
            let keywords_array = format!("[{}]", 
                doc.documentation.keywords.iter()
                    .map(|k| format!("'{}'", k.replace('\'', "''")))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO documentation VALUES ('{}', '{}', '{}', {}, '{}', '{}', '{}', {}, {}, {}, {}, NULL);\n",
                doc.file, doc.symbol_name, doc.symbol_type, doc.line_number,
                doc.documentation.raw.replace('\'', "''"),
                doc.documentation.clean.replace('\'', "''"),
                doc.documentation.summary.replace('\'', "''"),
                keywords_array,
                doc.documentation.doc_length,
                doc.documentation.has_examples,
                doc.documentation.has_params
            ));
        }
        
        // Store call graph
        for call in &results.call_graph {
            sql.push_str(&format!(
                "INSERT INTO call_graph VALUES ('{}', '{}', '{}', '{}');\n",
                call.caller, call.callee, file_path, call.call_type.as_str()
            ));
        }
        
        // Store types with all fields
        for typ in &results.types {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO type_vocabulary (file, name, definition, kind, visibility) VALUES ('{}', '{}', '{}', '{}', '{}');\n",
                typ.file, typ.name, 
                typ.definition.replace('\'', "''"),
                typ.kind, typ.visibility
            ));
        }
        
        // Store imports with all fields
        for import in &results.imports {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, '{}');\n",
                import.file, 
                import.imported_item.replace('\'', "''"),
                import.imported_from.replace('\'', "''"),
                import.is_external,
                import.import_kind
            ));
        }
        
        // Store behavioral hints with all fields
        for hint in &results.behavioral_hints {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO behavioral_hints (file, function, calls_unwrap, calls_expect, has_panic_macro, has_todo_macro, has_unsafe_block, has_mutex, has_arc) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {});\n",
                hint.file, hint.function,
                hint.calls_unwrap, hint.calls_expect,
                hint.has_panic_macro, hint.has_todo_macro,
                hint.has_unsafe_block, hint.has_mutex, hint.has_arc
            ));
        }
        
        // Store fingerprints
        for fp in &results.fingerprints {
            let bytes = fp.fingerprint.to_bytes();
            let hex = bytes.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO code_fingerprints VALUES ('{}', '{}', '\\x{}');\n",
                fp.file, fp.symbol, hex
            ));
        }
        
        sql.push_str("COMMIT;\n");
        
        self.execute_sql_stdin(&sql)?;
        Ok(())
    }
    
    fn query_by_keywords(&self, keywords: &[&str]) -> Result<Vec<Symbol>> {
        let mut conditions = Vec::new();
        for keyword in keywords {
            conditions.push(format!("list_contains(keywords, '{}')", keyword));
        }
        
        let query = format!(
            "SELECT DISTINCT 
                d.symbol_name as name,
                d.file,
                d.line_number,
                d.symbol_type,
                d.doc_summary,
                f.parameters,
                f.return_type
            FROM documentation d
            LEFT JOIN function_facts f ON d.symbol_name = f.name AND d.file = f.file
            WHERE {}
            ORDER BY d.doc_length DESC
            LIMIT 50",
            conditions.join(" OR ")
        );
        
        let output = self.execute_sql(&query)?;
        let mut symbols = Vec::new();
        
        for line in output.lines().skip(1) { // Skip CSV header
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 5 {
                symbols.push(Symbol {
                    name: parts[0].to_string(),
                    file: parts[1].to_string(),
                    line_number: parts[2].parse().unwrap_or(0),
                    symbol_type: parts[3].to_string(),
                    doc_summary: Some(parts[4].to_string()),
                    parameters: parts.get(5).map(|s| s.to_string()),
                    return_type: parts.get(6).map(|s| s.to_string()),
                });
            }
        }
        
        Ok(symbols)
    }
    
    fn get_call_graph(&self, symbol: &str) -> Result<Vec<CallRelation>> {
        let query = format!(
            "SELECT caller, callee, file, call_type 
            FROM call_graph 
            WHERE caller = '{}'",
            symbol
        );
        
        let output = self.execute_sql(&query)?;
        let mut relations = Vec::new();
        
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                relations.push(CallRelation {
                    caller: parts[0].to_string(),
                    callee: parts[1].to_string(),
                    call_type: match parts[3] {
                        "method" => CallType::Method,
                        "async" => CallType::Async,
                        "constructor" => CallType::Constructor,
                        "callback" => CallType::Callback,
                        _ => CallType::Direct,
                    },
                    line_number: 0, // Not stored in DB currently
                });
            }
        }
        
        Ok(relations)
    }
    
    fn get_call_chain(&self, entry_point: &str, max_depth: usize) -> Result<Vec<String>> {
        let query = format!(
            "WITH RECURSIVE call_chain AS (
                SELECT '{}' as func, 0 as depth
                UNION
                SELECT DISTINCT cg.callee, cc.depth + 1
                FROM call_graph cg
                JOIN call_chain cc ON cg.caller = cc.func
                WHERE cc.depth < {}
            )
            SELECT DISTINCT func FROM call_chain
            ORDER BY func",
            entry_point, max_depth
        );
        
        let output = self.execute_sql(&query)?;
        let mut chain = Vec::new();
        
        for line in output.lines().skip(1) {
            if !line.is_empty() {
                chain.push(line.to_string());
            }
        }
        
        Ok(chain)
    }
    
    fn get_documentation(&self, symbol: &str) -> Result<Option<DocumentationFact>> {
        let query = format!(
            "SELECT file, symbol_name, symbol_type, line_number, 
                    doc_raw, doc_clean, doc_summary, keywords,
                    doc_length, has_examples, has_params
            FROM documentation
            WHERE symbol_name = '{}'
            LIMIT 1",
            symbol
        );
        
        let output = self.execute_sql(&query)?;
        
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.splitn(11, ',').collect();
            if parts.len() >= 11 {
                // Parse keywords array
                let keywords_str = parts[7].trim_matches(|c| c == '[' || c == ']');
                let keywords: Vec<String> = if keywords_str.is_empty() {
                    Vec::new()
                } else {
                    keywords_str.split(',')
                        .map(|k| k.trim().trim_matches('\'').to_string())
                        .collect()
                };
                
                return Ok(Some(DocumentationFact {
                    file: parts[0].to_string(),
                    symbol_name: parts[1].to_string(),
                    symbol_type: parts[2].to_string(),
                    line_number: parts[3].parse().unwrap_or(0),
                    documentation: Documentation {
                        raw: parts[4].to_string(),
                        clean: parts[5].to_string(),
                        summary: parts[6].to_string(),
                        keywords,
                        doc_length: parts[8].parse().unwrap_or(0),
                        has_examples: parts[9] == "true",
                        has_params: parts[10] == "true",
                    },
                }));
            }
        }
        
        Ok(None)
    }
    
    fn get_function_facts(&self, symbol: &str) -> Result<Option<FunctionFact>> {
        let query = format!(
            "SELECT file, name, takes_mut_self, takes_mut_params, returns_result, 
                    returns_option, is_async, is_unsafe, is_public, parameter_count,
                    generic_count, parameters, return_type
            FROM function_facts
            WHERE name = '{}'
            LIMIT 1",
            symbol
        );
        
        let output = self.execute_sql(&query)?;
        
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.splitn(13, ',').collect();
            if parts.len() >= 13 {
                let parameters = parts[11].to_string();
                let return_type = parts[12].to_string();
                let signature = format!("{}({}){}", 
                    parts[1], // name
                    &parameters,
                    if !return_type.is_empty() { format!(" -> {}", return_type) } else { String::new() }
                );
                
                return Ok(Some(FunctionFact {
                    file: parts[0].to_string(),
                    name: parts[1].to_string(),
                    line_number: 0, // Not stored in database currently
                    takes_mut_self: parts[2] == "true",
                    takes_mut_params: parts[3] == "true",
                    returns_result: parts[4] == "true",
                    returns_option: parts[5] == "true",
                    is_async: parts[6] == "true",
                    is_unsafe: parts[7] == "true",
                    is_public: parts[8] == "true",
                    parameter_count: parts[9].parse().unwrap_or(0),
                    generics_count: parts[10].parse().unwrap_or(0),
                    parameters,
                    return_type,
                    signature,
                }));
            }
        }
        
        Ok(None)
    }
    
    fn execute_query(&self, query: &str) -> Result<String> {
        self.execute_sql(query)
    }
}