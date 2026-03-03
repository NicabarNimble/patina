// ============================================================================
// SEMANTIC CODE EXTRACTION PIPELINE V2 - MODULAR ARCHITECTURE
// ============================================================================
//! # Code → Knowledge ETL Pipeline
//!
//! A clean, modular refactor of the code extraction pipeline where each
//! language is fully self-contained in its own module file.
//!
//! ## Architecture
//! - Each language gets its own file (rust.rs, go.rs, etc.)
//! - Each language processor returns ExtractedData structs
//! - Database module uses transactions for bulk insert performance
//! - Clean separation of concerns
//!
//! ## Usage
//! ```bash
//! patina scrape code          # Index using modular architecture
//! patina scrape code --force  # Rebuild from scratch
//! ```

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::ScrapeConfig;

// ============================================================================
// LANGUAGE MODULES
// ============================================================================
pub mod database;
pub mod extract_v2;
pub mod extracted_data;
pub mod languages;
pub mod types;

// ============================================================================
// PUBLIC INTERFACE
// ============================================================================

/// Initialize a new knowledge database
pub fn initialize(config: &ScrapeConfig) -> Result<()> {
    println!("🗄️  Initializing optimized knowledge database...");

    // Create parent directory if needed
    if let Some(parent) = Path::new(&config.db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove old database if exists
    if Path::new(&config.db_path).exists() {
        std::fs::remove_file(&config.db_path)?;
    }

    // Initialize with schema
    initialize_database(&config.db_path)?;

    println!("✅ Database initialized at {}", config.db_path);
    Ok(())
}

/// Main entry point for the code command
pub fn run(config: ScrapeConfig) -> Result<super::ScrapeStats> {
    println!("🔄 Extracting semantic code information...");

    let start = std::time::Instant::now();

    // Determine work directory
    let work_dir = std::env::current_dir()?;
    println!("📂 Working directory: {}", work_dir.display());

    // Initialize database if needed
    if !Path::new(&config.db_path).exists() || config.force {
        initialize_database(&config.db_path)?;
    }

    // Always use the new embedded SQLite implementation
    let items_processed = extract_v2::extract_code_metadata_v2(
        &config.db_path,
        &work_dir,
        config.force,
        config.extension_filter.as_ref(),
    )?;

    // Populate FTS5 index for lexical search
    let conn = rusqlite::Connection::open(&config.db_path)?;
    let changed_refs = config.changed_code_files.as_deref();
    let fts_count = if changed_refs.is_some() {
        println!("📝 Updating FTS5 index (incremental)...");
        super::database::populate_fts5(&conn, changed_refs)?
    } else {
        println!("📝 Building FTS5 lexical index...");
        super::database::populate_fts5(&conn, None)?
    };
    println!("   Indexed {} symbols", fts_count);

    // Populate forge FTS5 from eventlog (idempotent — runs after code FTS5)
    let issue_fts = super::forge::populate_fts5_issues(&conn).unwrap_or(0);
    let pr_fts = super::forge::populate_fts5_prs(&conn).unwrap_or(0);
    if issue_fts > 0 || pr_fts > 0 {
        println!("   Indexed {} issues, {} PRs in FTS5", issue_fts, pr_fts);
    }

    // Get database size
    let metadata = std::fs::metadata(&config.db_path)?;
    let database_size_kb = metadata.len() / 1024;

    // Emit measurement: code scrape capture metrics
    let files_parsed: i64 = conn
        .query_row("SELECT COUNT(*) FROM index_state", [], |row| row.get(0))
        .unwrap_or(0);
    let functions_found: i64 = conn
        .query_row("SELECT COUNT(*) FROM function_facts", [], |row| row.get(0))
        .unwrap_or(0);
    let types_found: i64 = conn
        .query_row("SELECT COUNT(*) FROM type_vocabulary", [], |row| row.get(0))
        .unwrap_or(0);
    patina::measure::emit_or_warn(
        "capture",
        "scrape",
        "code",
        &serde_json::json!({
            "files_parsed": files_parsed,
            "functions_found": functions_found,
            "types_found": types_found,
            "fts_symbols": fts_count,
        }),
    );

    // Emit structural metrics for entropy tracking
    let module_count = count_modules(&work_dir);
    let pub_fn_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM function_facts WHERE is_public = 1 AND file LIKE './src/%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let pub_type_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM type_vocabulary WHERE visibility = 'pub' AND file LIKE './src/%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let pub_interface_count = pub_fn_count + pub_type_count;
    let dependency_count = count_dependencies(&work_dir);
    let (coupling_avg, coupling_max) = compute_coupling(&conn);

    patina::measure::emit_or_warn(
        "capture",
        "scrape",
        "structure",
        &serde_json::json!({
            "module_count": module_count,
            "pub_interface_count": pub_interface_count,
            "dependency_count": dependency_count,
            "coupling_avg": coupling_avg,
            "coupling_max": coupling_max,
        }),
    );

    Ok(super::ScrapeStats {
        items_processed,
        time_elapsed: start.elapsed(),
        database_size_kb,
    })
}

// ============================================================================
// INTERNAL IMPLEMENTATION
// ============================================================================

/// Initialize SQLite database with embedded library
fn initialize_database(db_path: &str) -> Result<()> {
    // Create parent directory if needed
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove old database if exists (when force flag is used)
    if Path::new(db_path).exists() {
        std::fs::remove_file(db_path)?;
    }

    // Initialize unified eventlog (git, sessions, code events)
    super::database::initialize(Path::new(db_path))?;

    // Create code-specific materialized views
    let mut db = database::Database::open(db_path)?;
    db.init_schema()?;
    Ok(())
}

// ============================================================================
// STRUCTURAL METRICS
// ============================================================================

/// Count top-level module directories under src/
fn count_modules(work_dir: &Path) -> i64 {
    let src_dir = work_dir.join("src");
    if !src_dir.exists() {
        return 0;
    }
    std::fs::read_dir(&src_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .count() as i64
        })
        .unwrap_or(0)
}

/// Count dependencies from Cargo.toml [dependencies] section
fn count_dependencies(work_dir: &Path) -> i64 {
    let cargo_path = work_dir.join("Cargo.toml");
    let content = match std::fs::read_to_string(&cargo_path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let doc: toml::Value = match content.parse() {
        Ok(d) => d,
        Err(_) => return 0,
    };
    doc.get("dependencies")
        .and_then(|d| d.as_table())
        .map(|t| t.len() as i64)
        .unwrap_or(0)
}

/// Compute cross-module coupling from import_facts.
///
/// For each top-level module under src/, counts how many OTHER top-level
/// modules it imports from via `crate::` paths. Returns (avg, max) fan-out.
fn compute_coupling(conn: &rusqlite::Connection) -> (f64, i64) {
    let mut stmt = match conn.prepare(
        "SELECT file, import_path FROM import_facts \
         WHERE file LIKE './src/%' AND import_path LIKE 'crate::%'",
    ) {
        Ok(s) => s,
        Err(_) => return (0.0, 0),
    };

    let mut module_targets: HashMap<String, HashSet<String>> = HashMap::new();

    let rows = match stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        Ok(r) => r,
        Err(_) => return (0.0, 0),
    };

    for row in rows.flatten() {
        let (file, import_path) = row;

        // Extract source module: ./src/MODULE/... → MODULE (skip root files)
        let src_rest = match file.strip_prefix("./src/") {
            Some(r) => r,
            None => continue,
        };
        if !src_rest.contains('/') {
            continue; // root-level file, not a module directory
        }
        let src_module = match src_rest.split('/').next() {
            Some(m) => m.to_string(),
            None => continue,
        };

        // Extract target module: crate::MODULE::... → MODULE
        let target_module = match import_path.strip_prefix("crate::") {
            Some(rest) => match rest.split("::").next() {
                Some(m) => m.to_string(),
                None => continue,
            },
            None => continue,
        };

        if src_module != target_module {
            module_targets
                .entry(src_module)
                .or_default()
                .insert(target_module);
        }
    }

    if module_targets.is_empty() {
        return (0.0, 0);
    }

    let fan_outs: Vec<usize> = module_targets.values().map(|s| s.len()).collect();
    let sum: usize = fan_outs.iter().sum();
    let avg = sum as f64 / fan_outs.len() as f64;
    let max = fan_outs.iter().copied().max().unwrap_or(0);

    (avg, max as i64)
}
