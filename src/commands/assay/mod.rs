//! Assay command - Query codebase structure
//!
//! Complement to scry (semantic search) for exact structural queries:
//! - Module inventory with line counts, function counts
//! - Import/importer relationships
//! - Caller/callee relationships from call graph

mod internal;

use anyhow::{Context, Result};
use internal::truncate;
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
    Derive,
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
        QueryType::Derive => execute_derive(&conn, &options),
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

/// Module signal data
#[derive(Debug, Serialize)]
pub struct ModuleSignal {
    pub path: String,
    pub is_used: bool,
    pub importer_count: i64,
    pub activity_level: String,
    pub last_commit_days: Option<i64>,
    pub top_contributors: Vec<String>,
    pub centrality_score: f64,
    // Phase 1.5: Robust signals
    pub commit_count: i64,
    pub contributor_count: i64,
    pub is_entry_point: bool,
    pub is_test_file: bool,
    pub directory_depth: i64,
    pub file_size_rank: f64,
}

/// Derive result
#[derive(Debug, Serialize)]
pub struct DeriveResult {
    pub signals: Vec<ModuleSignal>,
    pub summary: DeriveSummary,
}

#[derive(Debug, Serialize)]
pub struct DeriveSummary {
    pub total_modules: usize,
    pub used_modules: usize,
    pub dormant_modules: usize,
}

/// Check if a file is an entry point (main.rs, index.ts, __init__.py, mod.rs, etc.)
fn is_entry_point(path: &str) -> bool {
    let filename = path.rsplit('/').next().unwrap_or(path);
    matches!(
        filename,
        "main.rs"
            | "lib.rs"
            | "mod.rs"
            | "index.ts"
            | "index.js"
            | "index.tsx"
            | "index.jsx"
            | "__init__.py"
            | "__main__.py"
            | "main.go"
            | "main.py"
            | "app.py"
            | "app.ts"
            | "app.js"
    )
}

/// Check if a file is a test file
fn is_test_file(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    // Check path components
    path_lower.contains("/test/")
        || path_lower.contains("/tests/")
        || path_lower.contains("/__tests__/")
        || path_lower.contains("/spec/")
        || path_lower.contains("/specs/")
        // Check filename patterns
        || path_lower.ends_with("_test.rs")
        || path_lower.ends_with("_test.go")
        || path_lower.ends_with("_test.py")
        || path_lower.ends_with(".test.ts")
        || path_lower.ends_with(".test.js")
        || path_lower.ends_with(".test.tsx")
        || path_lower.ends_with(".test.jsx")
        || path_lower.ends_with(".spec.ts")
        || path_lower.ends_with(".spec.js")
        || path_lower.ends_with("_spec.rb")
        || path_lower.contains("/test_")
}

/// Compute directory depth from path (count of / separators)
fn compute_directory_depth(path: &str) -> i64 {
    path.trim_start_matches("./").matches('/').count() as i64
}

/// Compute structural signals for all modules
fn execute_derive(conn: &Connection, options: &AssayOptions) -> Result<()> {
    // Ensure module_signals table exists with Phase 1.5 columns
    // Drop and recreate to handle schema migration
    conn.execute("DROP TABLE IF EXISTS module_signals", [])?;
    conn.execute(
        "CREATE TABLE module_signals (
            path TEXT PRIMARY KEY,
            is_used INTEGER,
            importer_count INTEGER,
            activity_level TEXT,
            last_commit_days INTEGER,
            top_contributors TEXT,
            centrality_score REAL,
            staleness_flags TEXT,
            computed_at TEXT,
            -- Phase 1.5: Robust signals
            commit_count INTEGER,
            contributor_count INTEGER,
            is_entry_point INTEGER,
            is_test_file INTEGER,
            directory_depth INTEGER,
            file_size_rank REAL
        )",
        [],
    )?;

    // Get all modules from index_state with their sizes for file_size_rank computation
    let mut modules_stmt = conn.prepare(
        "SELECT path, size FROM index_state WHERE path LIKE '%.rs' OR path LIKE '%.py' OR path LIKE '%.ts' OR path LIKE '%.js' OR path LIKE '%.go'",
    )?;
    let modules_with_sizes: Vec<(String, i64)> = modules_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // Sort sizes for percentile rank computation
    let mut sorted_sizes: Vec<i64> = modules_with_sizes.iter().map(|(_, s)| *s).collect();
    sorted_sizes.sort();
    let total_files = sorted_sizes.len() as f64;

    let mut signals = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for (path, file_size) in &modules_with_sizes {
        // Convert file path to module path pattern for import matching
        // ./src/adapters/claude/mod.rs -> adapters::claude
        // ./src/adapters/templates.rs -> adapters::templates
        let module_path = path
            .trim_start_matches("./")
            .trim_start_matches("src/")
            .trim_end_matches(".rs")
            .trim_end_matches("/mod")
            .replace('/', "::");

        // Compute importer_count: how many files import this module
        let importer_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT file) FROM import_facts WHERE import_path LIKE ?",
                [format!("%{}%", module_path)],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Phase 1.5: is_used now includes is_entry_point
        let entry_point = is_entry_point(path);
        let is_used = importer_count > 0 || entry_point;

        // Compute centrality: degree centrality from call_graph
        // (number of callers + callees for functions in this file)
        let centrality_score: f64 = conn
            .query_row(
                "SELECT CAST(COUNT(*) AS REAL) / 100.0 FROM call_graph WHERE file = ?",
                [path],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        // Compute activity from git commits in eventlog (now includes commit_count)
        let (activity_level, last_commit_days, commit_count) = compute_activity(conn, path);

        // Get top contributors from git events (now includes contributor_count)
        let (top_contributors, contributor_count) = compute_contributors(conn, path);

        // Phase 1.5: Additional signals
        let test_file = is_test_file(path);
        let dir_depth = compute_directory_depth(path);

        // Compute file_size_rank as percentile (0.0 = smallest, 1.0 = largest)
        let file_size_rank = if total_files > 1.0 {
            let position = sorted_sizes
                .iter()
                .position(|&s| s >= *file_size)
                .unwrap_or(0);
            position as f64 / (total_files - 1.0)
        } else {
            0.5 // Single file gets middle rank
        };

        // Insert into module_signals with Phase 1.5 columns
        conn.execute(
            "INSERT INTO module_signals (path, is_used, importer_count, activity_level, last_commit_days, top_contributors, centrality_score, staleness_flags, computed_at, commit_count, contributor_count, is_entry_point, is_test_file, directory_depth, file_size_rank)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                path,
                is_used as i32,
                importer_count,
                &activity_level,
                last_commit_days,
                serde_json::to_string(&top_contributors).unwrap_or_else(|_| "[]".to_string()),
                centrality_score,
                "[]",
                &now,
                commit_count,
                contributor_count,
                entry_point as i32,
                test_file as i32,
                dir_depth,
                file_size_rank,
            ],
        )?;

        signals.push(ModuleSignal {
            path: path.clone(),
            is_used,
            importer_count,
            activity_level: activity_level.clone(),
            last_commit_days,
            top_contributors,
            centrality_score,
            commit_count,
            contributor_count,
            is_entry_point: entry_point,
            is_test_file: test_file,
            directory_depth: dir_depth,
            file_size_rank,
        });
    }

    // Calculate summary
    let total_modules = signals.len();
    let used_modules = signals.iter().filter(|s| s.is_used).count();
    let dormant_modules = signals
        .iter()
        .filter(|s| s.activity_level == "dormant")
        .count();

    let result = DeriveResult {
        signals,
        summary: DeriveSummary {
            total_modules,
            used_modules,
            dormant_modules,
        },
    };

    if options.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("🔬 Structural Signals Derived\n");
        println!(
            "Summary: {} modules, {} used, {} dormant\n",
            result.summary.total_modules,
            result.summary.used_modules,
            result.summary.dormant_modules
        );
        println!(
            "{:<45} {:>6} {:>8} {:>10} {:>8}",
            "Path", "Used", "Imports", "Activity", "Central"
        );
        println!("{}", "─".repeat(82));
        for s in &result.signals {
            println!(
                "{:<45} {:>6} {:>8} {:>10} {:>8.2}",
                truncate(&s.path, 45),
                if s.is_used { "✓" } else { "" },
                s.importer_count,
                s.activity_level,
                s.centrality_score
            );
        }
    }

    Ok(())
}

/// Compute activity level from git commits
/// Returns (activity_level, last_commit_days, commit_count)
fn compute_activity(conn: &Connection, path: &str) -> (String, Option<i64>, i64) {
    // Normalize path: strip ./ prefix to match git file paths
    let normalized_path = path.trim_start_matches("./");

    // Query git.commit events that touch this file using json_each to search the files array
    let result: Result<(i64, String), _> = conn.query_row(
        r#"
        SELECT
            COUNT(DISTINCT e.seq) as commit_count,
            MAX(e.timestamp) as last_commit
        FROM eventlog e, json_each(json_extract(e.data, '$.files')) as f
        WHERE e.event_type = 'git.commit'
          AND json_extract(f.value, '$.path') = ?
        "#,
        [normalized_path],
        |row| {
            Ok((
                row.get(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            ))
        },
    );

    match result {
        Ok((commit_count, last_commit)) => {
            // Calculate days since last commit
            let last_commit_days = if !last_commit.is_empty() {
                chrono::DateTime::parse_from_rfc3339(&last_commit)
                    .ok()
                    .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days())
            } else {
                None
            };

            // Determine activity level based on commits and recency
            let activity_level = match (commit_count, last_commit_days) {
                (0, _) => "dormant",
                (_, Some(days)) if days <= 7 => "high",
                (_, Some(days)) if days <= 30 => "medium",
                (_, Some(days)) if days <= 90 => "low",
                _ => "dormant",
            };

            (activity_level.to_string(), last_commit_days, commit_count)
        }
        Err(_) => ("dormant".to_string(), None, 0),
    }
}

/// Get top contributors for a file
/// Returns (top_contributors, contributor_count)
fn compute_contributors(conn: &Connection, path: &str) -> (Vec<String>, i64) {
    // Normalize path: strip ./ prefix to match git file paths
    let normalized_path = path.trim_start_matches("./");

    // First get the count of distinct contributors
    let contributor_count: i64 = conn
        .query_row(
            r#"
            SELECT COUNT(DISTINCT json_extract(e.data, '$.author_name'))
            FROM eventlog e, json_each(json_extract(e.data, '$.files')) as f
            WHERE e.event_type = 'git.commit'
              AND json_extract(f.value, '$.path') = ?
            "#,
            [normalized_path],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Then get top 3 contributors
    let mut stmt = match conn.prepare(
        r#"
        SELECT json_extract(e.data, '$.author_name') as author, COUNT(DISTINCT e.seq) as commits
        FROM eventlog e, json_each(json_extract(e.data, '$.files')) as f
        WHERE e.event_type = 'git.commit'
          AND json_extract(f.value, '$.path') = ?
        GROUP BY author
        ORDER BY commits DESC
        LIMIT 3
        "#,
    ) {
        Ok(s) => s,
        Err(_) => return (vec![], contributor_count),
    };

    let top_contributors = stmt
        .query_map([normalized_path], |row| row.get(0))
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    (top_contributors, contributor_count)
}

