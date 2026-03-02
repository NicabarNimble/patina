//! Inventory queries
//!
//! "Do X": List files and modules in codebase with stats

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use super::super::AssayOptions;
use super::truncate;

/// Module stats from inventory query
#[derive(Debug, Serialize)]
pub struct ModuleStats {
    pub path: String,
    pub lines: i64,
    pub bytes: i64,
    pub functions: i64,
    pub imports: i64,
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

/// Collect inventory results as JSON (for all_repos mode)
pub fn collect_inventory_json(
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

/// Query inventory across all registered repos and return JSON string
pub fn inventory_all_repos_json(options: &super::super::AssayOptions) -> Result<String> {
    use std::path::Path;

    const DB_PATH: &str = ".patina/local/data/patina.db";

    let repos = crate::commands::repo::list()?;
    let current_has_db = Path::new(DB_PATH).exists();

    if !matches!(options.query_type, super::super::QueryType::Inventory) {
        anyhow::bail!("all_repos mode currently only supports 'inventory' query type");
    }

    let mut all_results: Vec<serde_json::Value> = Vec::new();

    if current_has_db {
        if let Ok(conn) = Connection::open(DB_PATH) {
            if let Ok(results) = collect_inventory_json(&conn, options, Some("(current)")) {
                all_results.extend(results);
            }
        }
    }

    for repo in &repos {
        let db_path = Path::new(&repo.path).join(DB_PATH);
        if let Ok(conn) = Connection::open(&db_path) {
            if let Ok(results) = collect_inventory_json(&conn, options, Some(&repo.name)) {
                all_results.extend(results);
            }
        }
    }

    let total_lines: i64 = all_results.iter().filter_map(|m| m["lines"].as_i64()).sum();
    let total_functions: i64 = all_results
        .iter()
        .filter_map(|m| m["functions"].as_i64())
        .sum();

    let result = serde_json::json!({
        "modules": all_results,
        "summary": {
            "total_files": all_results.len(),
            "total_lines": total_lines,
            "total_functions": total_functions,
            "repos_queried": repos.len() + if current_has_db { 1 } else { 0 }
        }
    });

    Ok(serde_json::to_string_pretty(&result)?)
}

/// Query module inventory and return JSON string
pub fn inventory_json(conn: &Connection, pattern: &str, limit: usize) -> Result<String> {
    let result = inventory_result(conn, pattern, limit)?;
    Ok(serde_json::to_string_pretty(&result)?)
}

/// Query module inventory and return typed result
pub fn inventory_result(conn: &Connection, pattern: &str, limit: usize) -> Result<InventoryResult> {
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

    let total_files = modules.len();
    let total_lines: i64 = modules.iter().map(|m| m.lines).sum();
    let total_functions: i64 = modules.iter().map(|m| m.functions).sum();

    Ok(InventoryResult {
        modules,
        summary: InventorySummary {
            total_files,
            total_lines,
            total_functions,
        },
    })
}

/// Query module inventory with stats
pub fn execute_inventory(
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

    let result = inventory_result(conn, pattern, limit)?;

    if options.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Codebase Inventory\n");
        println!(
            "Summary: {} files, {} lines, {} functions\n",
            result.summary.total_files, result.summary.total_lines, result.summary.total_functions
        );
        println!(
            "{:<50} {:>8} {:>8} {:>8}",
            "Path", "Lines", "Funcs", "Imports"
        );
        println!("{}", "-".repeat(80));
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
