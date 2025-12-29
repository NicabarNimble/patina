//! Function and call graph queries
//!
//! "Do X": Query function definitions and call relationships

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use super::super::AssayOptions;
use super::truncate;

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

/// Query functions
pub fn execute_functions(conn: &Connection, options: &AssayOptions) -> Result<()> {
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
            "Functions{}\n",
            options
                .pattern
                .as_ref()
                .map(|p| format!(" matching '{}'", p))
                .unwrap_or_default()
        );
        println!("{:<30} {:<40} {:>5} {:>5}", "Name", "File", "Pub", "Async");
        println!("{}", "-".repeat(84));
        for f in &functions {
            println!(
                "{:<30} {:<40} {:>5} {:>5}",
                truncate(&f.name, 30),
                truncate(&f.file, 40),
                if f.is_public { "Y" } else { "" },
                if f.is_async { "Y" } else { "" }
            );
        }
        println!("\nFound {} functions", functions.len());
    }

    Ok(())
}

/// Query callers of a function
pub fn execute_callers(conn: &Connection, options: &AssayOptions) -> Result<()> {
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
        println!("Callers of '{}'\n", pattern);
        println!("{:<30} {:<30} {:<20}", "Caller", "Callee", "File");
        println!("{}", "-".repeat(82));
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
pub fn execute_callees(conn: &Connection, options: &AssayOptions) -> Result<()> {
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
        println!("Callees of '{}'\n", pattern);
        println!("{:<30} {:<30} {:<20}", "Caller", "Callee", "File");
        println!("{}", "-".repeat(82));
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
