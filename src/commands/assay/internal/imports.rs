//! Import relationship queries
//!
//! "Do X": Query import relationships between files

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use super::super::AssayOptions;
use super::truncate;

/// Import info
#[derive(Debug, Serialize)]
pub struct ImportInfo {
    pub path: String,
    pub kind: String,
}

/// Query what a module imports
pub fn execute_imports(conn: &Connection, options: &AssayOptions) -> Result<()> {
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
        println!("Imports matching '{}'\n", pattern);
        println!("{:<60} {:>10}", "Import Path", "Kind");
        println!("{}", "-".repeat(72));
        for i in &imports {
            println!("{:<60} {:>10}", truncate(&i.path, 60), i.kind);
        }
        println!("\nFound {} imports", imports.len());
    }

    Ok(())
}

/// Query what modules import a given module
pub fn execute_importers(conn: &Connection, options: &AssayOptions) -> Result<()> {
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
        println!("Modules importing '{}'\n", pattern);
        println!("{:<50} Imported Names", "File");
        println!("{}", "-".repeat(80));
        for (file, names) in &importers {
            println!("{:<50} {}", truncate(file, 50), truncate(names, 30));
        }
        println!("\nFound {} importers", importers.len());
    }

    Ok(())
}
