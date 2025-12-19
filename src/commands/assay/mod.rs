//! Assay command - Query codebase structure
//!
//! Complement to scry (semantic search) for exact structural queries:
//! - Module inventory with line counts, function counts
//! - Import/importer relationships
//! - Caller/callee relationships from call graph

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

const DB_PATH: &str = ".patina/data/patina.db";

/// Query type for assay command
#[derive(Debug, Clone, Copy, Default)]
pub enum QueryType {
    #[default]
    Inventory,
    Imports,
    Importers,
    Functions,
    Callers,
    Callees,
}

/// Options for assay command
#[derive(Debug, Clone, Default)]
pub struct AssayOptions {
    pub query_type: QueryType,
    pub pattern: Option<String>,
    pub limit: usize,
    pub json: bool,
    /// Query a specific registered repo by name
    pub repo: Option<String>,
    /// Query all registered repos
    pub all_repos: bool,
}

/// Module stats from inventory query
#[derive(Debug, Serialize)]
pub struct ModuleStats {
    pub path: String,
    pub lines: i64,
    pub bytes: i64,
    pub functions: i64,
    pub imports: i64,
}

/// Import info
#[derive(Debug, Serialize)]
pub struct ImportInfo {
    pub path: String,
    pub kind: String,
}

/// Function info
#[derive(Debug, Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub file: String,
    pub is_public: bool,
    pub is_async: bool,
    pub parameters: String,
    pub return_type: Option<String>,
}

/// Caller/callee info
#[derive(Debug, Serialize)]
pub struct CallInfo {
    pub caller: String,
    pub callee: String,
    pub file: String,
    pub call_type: String,
}

/// Inventory result
#[derive(Debug, Serialize)]
pub struct InventoryResult {
    pub modules: Vec<ModuleStats>,
    pub summary: InventorySummary,
}

#[derive(Debug, Serialize)]
pub struct InventorySummary {
    pub total_files: usize,
    pub total_lines: i64,
    pub total_functions: i64,
}

/// Execute assay command
pub fn execute(options: AssayOptions) -> Result<()> {
    // Handle all_repos mode: iterate over all registered repos
    if options.all_repos {
        return execute_all_repos(&options);
    }

    // Resolve database path: specific repo or current directory
    let db_path = match &options.repo {
        Some(name) => crate::commands::repo::get_db_path(name)?,
        None => DB_PATH.to_string(),
    };

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Show repo context if specified
    if let Some(ref repo) = options.repo {
        println!("Repository: {}\n", repo);
    }

    match options.query_type {
        QueryType::Inventory => execute_inventory(&conn, &options, None),
        QueryType::Imports => execute_imports(&conn, &options),
        QueryType::Importers => execute_importers(&conn, &options),
        QueryType::Functions => execute_functions(&conn, &options),
        QueryType::Callers => execute_callers(&conn, &options),
        QueryType::Callees => execute_callees(&conn, &options),
    }
}

/// Execute assay across all registered repos
fn execute_all_repos(options: &AssayOptions) -> Result<()> {
    let repos = crate::commands::repo::list()?;

    if repos.is_empty() {
        println!("No registered repos. Use 'patina repo add <url>' to add repos.");
        return Ok(());
    }

    // Also query current project if it has a database
    let current_has_db = std::path::Path::new(DB_PATH).exists();

    if options.json {
        // JSON mode: collect all results into a single array
        let mut all_results: Vec<serde_json::Value> = Vec::new();

        if current_has_db {
            if let Ok(conn) = Connection::open(DB_PATH) {
                if let Ok(results) = collect_inventory_json(&conn, options, Some("(current)")) {
                    all_results.extend(results);
                }
            }
        }

        for repo in &repos {
            let db_path = std::path::Path::new(&repo.path).join(".patina/data/patina.db");
            if let Ok(conn) = Connection::open(&db_path) {
                if let Ok(results) = collect_inventory_json(&conn, options, Some(&repo.name)) {
                    all_results.extend(results);
                }
            }
        }

        println!("{}", serde_json::to_string_pretty(&all_results)?);
    } else {
        // Text mode: print each repo's results with headers
        if current_has_db {
            println!("━━━ (current) ━━━\n");
            if let Ok(conn) = Connection::open(DB_PATH) {
                let _ = execute_inventory(&conn, options, Some("(current)"));
            }
            println!();
        }

        for repo in &repos {
            println!("━━━ {} ━━━\n", repo.name);
            let db_path = std::path::Path::new(&repo.path).join(".patina/data/patina.db");
            if let Ok(conn) = Connection::open(&db_path) {
                let _ = execute_inventory(&conn, options, Some(&repo.name));
            } else {
                println!("  (database not found)\n");
            }
            println!();
        }
    }

    Ok(())
}

/// Collect inventory results as JSON (for all_repos mode)
fn collect_inventory_json(
    conn: &Connection,
    options: &AssayOptions,
    repo_name: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let pattern = options.pattern.as_deref().unwrap_or("%");
    let limit = if options.limit > 0 {
        options.limit
    } else {
        1000
    };

    let sql = r#"
        SELECT
            i.path,
            COALESCE(i.line_count, 0) as lines,
            i.size as bytes,
            COALESCE((SELECT COUNT(*) FROM function_facts WHERE file = i.path), 0) as functions,
            COALESCE((SELECT COUNT(*) FROM import_facts WHERE file = i.path), 0) as imports
        FROM index_state i
        WHERE i.path LIKE ?
        ORDER BY lines DESC
        LIMIT ?
    "#;

    let mut stmt = conn.prepare(sql)?;
    let results: Vec<serde_json::Value> = stmt
        .query_map([pattern, &limit.to_string()], |row| {
            Ok(ModuleStats {
                path: row.get(0)?,
                lines: row.get(1)?,
                bytes: row.get(2)?,
                functions: row.get(3)?,
                imports: row.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .map(|m| {
            let mut obj = serde_json::json!({
                "path": m.path,
                "lines": m.lines,
                "bytes": m.bytes,
                "functions": m.functions,
                "imports": m.imports,
            });
            if let Some(name) = repo_name {
                obj["repo"] = serde_json::json!(name);
            }
            obj
        })
        .collect();

    Ok(results)
}

/// Query module inventory with stats
fn execute_inventory(
    conn: &Connection,
    options: &AssayOptions,
    _repo_name: Option<&str>,
) -> Result<()> {
    let pattern = options.pattern.as_deref().unwrap_or("%");
    let limit = if options.limit > 0 {
        options.limit
    } else {
        1000
    };

    // Query modules with aggregated stats
    let sql = r#"
        SELECT
            i.path,
            COALESCE(i.line_count, 0) as lines,
            i.size as bytes,
            COALESCE((SELECT COUNT(*) FROM function_facts WHERE file = i.path), 0) as functions,
            COALESCE((SELECT COUNT(*) FROM import_facts WHERE file = i.path), 0) as imports
        FROM index_state i
        WHERE i.path LIKE ?
        ORDER BY lines DESC
        LIMIT ?
    "#;

    let mut stmt = conn.prepare(sql)?;
    let modules: Vec<ModuleStats> = stmt
        .query_map([pattern, &limit.to_string()], |row| {
            Ok(ModuleStats {
                path: row.get(0)?,
                lines: row.get(1)?,
                bytes: row.get(2)?,
                functions: row.get(3)?,
                imports: row.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Calculate summary
    let total_files = modules.len();
    let total_lines: i64 = modules.iter().map(|m| m.lines).sum();
    let total_functions: i64 = modules.iter().map(|m| m.functions).sum();

    let result = InventoryResult {
        modules,
        summary: InventorySummary {
            total_files,
            total_lines,
            total_functions,
        },
    };

    if options.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("📊 Codebase Inventory\n");
        println!(
            "Summary: {} files, {} lines, {} functions\n",
            result.summary.total_files, result.summary.total_lines, result.summary.total_functions
        );
        println!(
            "{:<50} {:>8} {:>8} {:>8}",
            "Path", "Lines", "Funcs", "Imports"
        );
        println!("{}", "─".repeat(80));
        for m in &result.modules {
            println!(
                "{:<50} {:>8} {:>8} {:>8}",
                truncate(&m.path, 50),
                m.lines,
                m.functions,
                m.imports
            );
        }
    }

    Ok(())
}

/// Query what a module imports
fn execute_imports(conn: &Connection, options: &AssayOptions) -> Result<()> {
    let pattern = options
        .pattern
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--imports requires a module path pattern"))?;
    let limit = if options.limit > 0 {
        options.limit
    } else {
        100
    };

    let sql = r#"
        SELECT import_path, import_kind
        FROM import_facts
        WHERE file LIKE ?
        ORDER BY import_path
        LIMIT ?
    "#;

    let mut stmt = conn.prepare(sql)?;
    let imports: Vec<ImportInfo> = stmt
        .query_map([format!("%{}%", pattern), limit.to_string()], |row| {
            Ok(ImportInfo {
                path: row.get(0)?,
                kind: row.get(1)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if options.json {
        println!("{}", serde_json::to_string_pretty(&imports)?);
    } else {
        println!("📦 Imports matching '{}'\n", pattern);
        println!("{:<60} {:>10}", "Import Path", "Kind");
        println!("{}", "─".repeat(72));
        for i in &imports {
            println!("{:<60} {:>10}", truncate(&i.path, 60), i.kind);
        }
        println!("\nFound {} imports", imports.len());
    }

    Ok(())
}

/// Query what modules import a given module
fn execute_importers(conn: &Connection, options: &AssayOptions) -> Result<()> {
    let pattern = options
        .pattern
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--importers requires a module name pattern"))?;
    let limit = if options.limit > 0 {
        options.limit
    } else {
        100
    };

    let sql = r#"
        SELECT file, imported_names
        FROM import_facts
        WHERE import_path LIKE ?
        ORDER BY file
        LIMIT ?
    "#;

    let mut stmt = conn.prepare(sql)?;
    let importers: Vec<(String, String)> = stmt
        .query_map([format!("%{}%", pattern), limit.to_string()], |row| {
            Ok((
                row.get(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if options.json {
        let result: Vec<_> = importers
            .iter()
            .map(|(file, names)| serde_json::json!({"file": file, "names": names}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("🔗 Modules importing '{}'\n", pattern);
        println!("{:<50} Imported Names", "File");
        println!("{}", "─".repeat(80));
        for (file, names) in &importers {
            println!("{:<50} {}", truncate(file, 50), truncate(names, 30));
        }
        println!("\nFound {} importers", importers.len());
    }

    Ok(())
}

/// Query functions
fn execute_functions(conn: &Connection, options: &AssayOptions) -> Result<()> {
    let limit = if options.limit > 0 {
        options.limit
    } else {
        100
    };

    let (sql, params): (&str, Vec<String>) = if let Some(pattern) = &options.pattern {
        (
            r#"
            SELECT name, file, is_public, is_async, parameters, return_type
            FROM function_facts
            WHERE name LIKE ? OR file LIKE ?
            ORDER BY file, name
            LIMIT ?
            "#,
            vec![
                format!("%{}%", pattern),
                format!("%{}%", pattern),
                limit.to_string(),
            ],
        )
    } else {
        (
            r#"
            SELECT name, file, is_public, is_async, parameters, return_type
            FROM function_facts
            ORDER BY file, name
            LIMIT ?
            "#,
            vec![limit.to_string()],
        )
    };

    let mut stmt = conn.prepare(sql)?;
    let functions: Vec<FunctionInfo> = if options.pattern.is_some() {
        stmt.query_map([&params[0], &params[1], &params[2]], |row| {
            Ok(FunctionInfo {
                name: row.get(0)?,
                file: row.get(1)?,
                is_public: row.get(2)?,
                is_async: row.get(3)?,
                parameters: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                return_type: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map([&params[0]], |row| {
            Ok(FunctionInfo {
                name: row.get(0)?,
                file: row.get(1)?,
                is_public: row.get(2)?,
                is_async: row.get(3)?,
                parameters: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                return_type: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect()
    };

    if options.json {
        println!("{}", serde_json::to_string_pretty(&functions)?);
    } else {
        println!(
            "🔧 Functions{}\n",
            options
                .pattern
                .as_ref()
                .map(|p| format!(" matching '{}'", p))
                .unwrap_or_default()
        );
        println!("{:<30} {:<40} {:>5} {:>5}", "Name", "File", "Pub", "Async");
        println!("{}", "─".repeat(84));
        for f in &functions {
            println!(
                "{:<30} {:<40} {:>5} {:>5}",
                truncate(&f.name, 30),
                truncate(&f.file, 40),
                if f.is_public { "✓" } else { "" },
                if f.is_async { "✓" } else { "" }
            );
        }
        println!("\nFound {} functions", functions.len());
    }

    Ok(())
}

/// Query callers of a function
fn execute_callers(conn: &Connection, options: &AssayOptions) -> Result<()> {
    let pattern = options
        .pattern
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--callers requires a function name"))?;
    let limit = if options.limit > 0 {
        options.limit
    } else {
        100
    };

    let sql = r#"
        SELECT caller, callee, file, call_type
        FROM call_graph
        WHERE callee LIKE ?
        ORDER BY file, caller
        LIMIT ?
    "#;

    let mut stmt = conn.prepare(sql)?;
    let callers: Vec<CallInfo> = stmt
        .query_map([format!("%{}%", pattern), limit.to_string()], |row| {
            Ok(CallInfo {
                caller: row.get(0)?,
                callee: row.get(1)?,
                file: row.get(2)?,
                call_type: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if options.json {
        println!("{}", serde_json::to_string_pretty(&callers)?);
    } else {
        println!("📞 Callers of '{}'\n", pattern);
        println!("{:<30} {:<30} {:<20}", "Caller", "Callee", "File");
        println!("{}", "─".repeat(82));
        for c in &callers {
            println!(
                "{:<30} {:<30} {:<20}",
                truncate(&c.caller, 30),
                truncate(&c.callee, 30),
                truncate(&c.file, 20)
            );
        }
        println!("\nFound {} call sites", callers.len());
    }

    Ok(())
}

/// Query callees of a function
fn execute_callees(conn: &Connection, options: &AssayOptions) -> Result<()> {
    let pattern = options
        .pattern
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--callees requires a function name"))?;
    let limit = if options.limit > 0 {
        options.limit
    } else {
        100
    };

    let sql = r#"
        SELECT caller, callee, file, call_type
        FROM call_graph
        WHERE caller LIKE ?
        ORDER BY file, callee
        LIMIT ?
    "#;

    let mut stmt = conn.prepare(sql)?;
    let callees: Vec<CallInfo> = stmt
        .query_map([format!("%{}%", pattern), limit.to_string()], |row| {
            Ok(CallInfo {
                caller: row.get(0)?,
                callee: row.get(1)?,
                file: row.get(2)?,
                call_type: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if options.json {
        println!("{}", serde_json::to_string_pretty(&callees)?);
    } else {
        println!("📤 Callees of '{}'\n", pattern);
        println!("{:<30} {:<30} {:<20}", "Caller", "Callee", "File");
        println!("{}", "─".repeat(82));
        for c in &callees {
            println!(
                "{:<30} {:<30} {:<20}",
                truncate(&c.caller, 30),
                truncate(&c.callee, 30),
                truncate(&c.file, 20)
            );
        }
        println!("\nFound {} call sites", callees.len());
    }

    Ok(())
}

/// Truncate string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a very long string", 10), "a very ..."); // 7 chars + "..."
    }
}
