//! Structural signal computation
//!
//! "Do X": Compute structural signals from code facts

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use super::super::AssayOptions;
use super::truncate;

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
pub fn execute_derive(conn: &Connection, options: &AssayOptions) -> Result<()> {
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
        println!("Structural Signals Derived\n");
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
        println!("{}", "-".repeat(82));
        for s in &result.signals {
            println!(
                "{:<45} {:>6} {:>8} {:>10} {:>8.2}",
                truncate(&s.path, 45),
                if s.is_used { "Y" } else { "" },
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

    // Query commit_files + commits tables (works for both project and ref repos)
    // Note: For ref repos, eventlog doesn't have git.commit events (lean storage)
    let result: Result<(i64, String), _> = conn.query_row(
        r#"
        SELECT
            COUNT(DISTINCT cf.sha) as commit_count,
            MAX(c.timestamp) as last_commit
        FROM commit_files cf
        JOIN commits c ON cf.sha = c.sha
        WHERE cf.file_path = ?
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

// ============================================================================
// Moments: Temporal signals from git history
// ============================================================================

/// Moment result
#[derive(Debug, Serialize)]
pub struct Moment {
    pub sha: String,
    pub moment_type: String,
    pub file_count: i64,
    pub timestamp: String,
    pub message: String,
}

/// Moments summary
#[derive(Debug, Serialize)]
pub struct MomentsSummary {
    pub total_commits: i64,
    pub genesis: i64,
    pub big_bang: i64,
    pub major: i64,
    pub breaking: i64,
    pub migration: i64,
    pub rewrite: i64,
}

/// Derive moments from git commits
pub fn execute_derive_moments(conn: &Connection, options: &AssayOptions) -> Result<()> {
    // Ensure moments table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS moments (
            sha TEXT PRIMARY KEY,
            moment_type TEXT NOT NULL,
            file_count INTEGER,
            timestamp TEXT,
            message TEXT
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_moments_type ON moments(moment_type)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_moments_timestamp ON moments(timestamp)",
        [],
    )?;

    // Clear existing moments
    conn.execute("DELETE FROM moments", [])?;

    // Detect moments using SQL
    // Priority order matters: genesis > big_bang > major > breaking/migration/rewrite
    let moments_sql = r#"
        WITH file_counts AS (
            SELECT sha, COUNT(*) as files FROM commit_files GROUP BY sha
        ),
        genesis_sha AS (
            SELECT sha FROM commits ORDER BY timestamp ASC LIMIT 1
        ),
        classified AS (
            SELECT
                c.sha,
                c.message,
                c.timestamp,
                COALESCE(fc.files, 0) as file_count,
                CASE
                    WHEN c.sha = (SELECT sha FROM genesis_sha) THEN 'genesis'
                    WHEN fc.files > 100 THEN 'big_bang'
                    WHEN fc.files > 50 THEN 'major'
                    WHEN LOWER(c.message) LIKE '%breaking%' THEN 'breaking'
                    WHEN LOWER(c.message) LIKE '%rewrite%' THEN 'rewrite'
                    WHEN LOWER(c.message) LIKE '%refactor%' THEN 'rewrite'
                    WHEN LOWER(c.message) LIKE '%migrate%' THEN 'migration'
                    WHEN LOWER(c.message) LIKE '%migration%' THEN 'migration'
                    ELSE NULL
                END as moment_type
            FROM commits c
            LEFT JOIN file_counts fc ON c.sha = fc.sha
        )
        SELECT sha, message, timestamp, file_count, moment_type
        FROM classified
        WHERE moment_type IS NOT NULL
        ORDER BY timestamp ASC
    "#;

    let mut stmt = conn.prepare(moments_sql)?;
    let moments: Vec<Moment> = stmt
        .query_map([], |row| {
            Ok(Moment {
                sha: row.get(0)?,
                message: row.get(1)?,
                timestamp: row.get(2)?,
                file_count: row.get(3)?,
                moment_type: row.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Insert into moments table
    let mut insert_stmt = conn.prepare(
        "INSERT INTO moments (sha, moment_type, file_count, timestamp, message) VALUES (?, ?, ?, ?, ?)",
    )?;

    for m in &moments {
        insert_stmt.execute(rusqlite::params![
            &m.sha,
            &m.moment_type,
            m.file_count,
            &m.timestamp,
            &m.message,
        ])?;
    }

    // Compute summary
    let total_commits: i64 = conn.query_row("SELECT COUNT(*) FROM commits", [], |r| r.get(0))?;
    let count_type = |t: &str| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM moments WHERE moment_type = ?",
            [t],
            |r| r.get(0),
        )
        .unwrap_or(0)
    };

    let summary = MomentsSummary {
        total_commits,
        genesis: count_type("genesis"),
        big_bang: count_type("big_bang"),
        major: count_type("major"),
        breaking: count_type("breaking"),
        migration: count_type("migration"),
        rewrite: count_type("rewrite"),
    };

    if options.json {
        let result = serde_json::json!({
            "moments": moments,
            "summary": summary,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Moments Derived (Temporal Signals)\n");
        println!(
            "Summary: {} commits → {} genesis, {} big_bang, {} major, {} breaking, {} migration, {} rewrite\n",
            summary.total_commits,
            summary.genesis,
            summary.big_bang,
            summary.major,
            summary.breaking,
            summary.migration,
            summary.rewrite,
        );

        if !moments.is_empty() {
            println!("{:<12} {:<10} {:>6} Message", "Type", "SHA", "Files");
            println!("{}", "-".repeat(80));
            for m in &moments {
                println!(
                    "{:<12} {:<10} {:>6} {}",
                    m.moment_type,
                    &m.sha[..10.min(m.sha.len())],
                    m.file_count,
                    truncate(&m.message, 45),
                );
            }
        }
    }

    Ok(())
}

/// Get top contributors for a file
/// Returns (top_contributors, contributor_count)
fn compute_contributors(conn: &Connection, path: &str) -> (Vec<String>, i64) {
    // Normalize path: strip ./ prefix to match git file paths
    let normalized_path = path.trim_start_matches("./");

    // Query commit_files + commits tables (works for both project and ref repos)
    // Note: For ref repos, eventlog doesn't have git.commit events (lean storage)

    // First get the count of distinct contributors
    let contributor_count: i64 = conn
        .query_row(
            r#"
            SELECT COUNT(DISTINCT c.author_name)
            FROM commit_files cf
            JOIN commits c ON cf.sha = c.sha
            WHERE cf.file_path = ?
            "#,
            [normalized_path],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Then get top 3 contributors
    let mut stmt = match conn.prepare(
        r#"
        SELECT c.author_name as author, COUNT(DISTINCT cf.sha) as commits
        FROM commit_files cf
        JOIN commits c ON cf.sha = c.sha
        WHERE cf.file_path = ?
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
