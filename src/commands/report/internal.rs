//! Internal implementation for report command
//!
//! Follows dependable-rust pattern: private module with curated exports.

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::path::Path;

use super::ReportOptions;
use crate::commands::scry::{scry, ScryOptions};

const DB_PATH: &str = ".patina/data/patina.db";

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Serialize)]
pub struct Report {
    pub generated: String,
    pub project_name: String,
    pub summary: Summary,
    pub architecture: Vec<ArchitectureSection>,
    pub largest_modules: Vec<ModuleInfo>,
    pub rag_health: RagHealth,
    pub tool_performance: ToolPerformance,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub total_files: usize,
    pub total_lines: i64,
    pub total_functions: i64,
    pub languages: Vec<LanguageStats>,
}

#[derive(Debug, Serialize)]
pub struct LanguageStats {
    pub language: String,
    pub files: usize,
    pub lines: i64,
}

#[derive(Debug, Serialize)]
pub struct ArchitectureSection {
    pub query: String,
    pub results: Vec<ArchResult>,
    pub empty: bool,
}

#[derive(Debug, Serialize)]
pub struct ArchResult {
    pub source: String,
    pub content: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct ModuleInfo {
    pub path: String,
    pub lines: i64,
    pub functions: i64,
    pub percent_of_total: f32,
}

#[derive(Debug, Serialize)]
pub struct RagHealth {
    pub last_scrape: Option<String>,
    pub total_events: i64,
    pub code_events: i64,
    pub git_events: i64,
    pub session_events: i64,
    pub indexed_files: i64,
    pub has_vectors: bool,
    pub vector_dimensions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolPerformance {
    pub scry_queries_run: usize,
    pub scry_empty_results: usize,
    pub scry_avg_results: f32,
}

// ============================================================================
// Report Generation
// ============================================================================

pub fn generate_report(options: ReportOptions) -> Result<()> {
    println!("📊 Generating project report...\n");

    // Determine database path
    let db_path = match &options.repo {
        Some(name) => crate::commands::repo::get_db_path(name)?,
        None => DB_PATH.to_string(),
    };

    // Check if database exists
    if !Path::new(&db_path).exists() {
        anyhow::bail!(
            "No knowledge database found at {}. Run 'patina scrape' first.",
            db_path
        );
    }

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Get project name
    let project_name = get_project_name(&options)?;

    // Collect data
    println!("  Collecting summary metrics...");
    let summary = collect_summary(&conn)?;

    println!("  Querying architecture via scry...");
    let (architecture, tool_perf) = collect_architecture(&options)?;

    println!("  Collecting largest modules...");
    let largest_modules = collect_largest_modules(&conn, summary.total_lines)?;

    println!("  Checking RAG health...");
    let rag_health = collect_rag_health(&conn)?;

    // Build report
    let report = Report {
        generated: Utc::now().to_rfc3339(),
        project_name,
        summary,
        architecture,
        largest_modules,
        rag_health,
        tool_performance: tool_perf,
    };

    // Output
    if options.json {
        println!("\n{}", serde_json::to_string_pretty(&report)?);
    } else {
        let markdown = render_markdown(&report);

        // Save to file
        let output_path = get_output_path(&options)?;
        save_report(&output_path, &markdown)?;

        println!("\n{}", markdown);
        println!("\n📁 Report saved to: {}", output_path);
    }

    Ok(())
}

// ============================================================================
// Data Collection
// ============================================================================

fn get_project_name(options: &ReportOptions) -> Result<String> {
    if let Some(ref repo) = options.repo {
        return Ok(repo.clone());
    }

    // Try to get from git remote or directory name
    let cwd = std::env::current_dir()?;
    Ok(cwd
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string()))
}

fn collect_summary(conn: &Connection) -> Result<Summary> {
    // Get file counts and line totals by extension
    let sql = r#"
        SELECT
            CASE
                WHEN path LIKE '%.rs' THEN 'Rust'
                WHEN path LIKE '%.ts' THEN 'TypeScript'
                WHEN path LIKE '%.tsx' THEN 'TypeScript'
                WHEN path LIKE '%.js' THEN 'JavaScript'
                WHEN path LIKE '%.jsx' THEN 'JavaScript'
                WHEN path LIKE '%.py' THEN 'Python'
                WHEN path LIKE '%.go' THEN 'Go'
                WHEN path LIKE '%.md' THEN 'Markdown'
                WHEN path LIKE '%.toml' THEN 'TOML'
                WHEN path LIKE '%.yaml' OR path LIKE '%.yml' THEN 'YAML'
                WHEN path LIKE '%.json' THEN 'JSON'
                ELSE 'Other'
            END as language,
            COUNT(*) as file_count,
            COALESCE(SUM(line_count), 0) as line_count
        FROM index_state
        GROUP BY language
        ORDER BY line_count DESC
    "#;

    let mut stmt = conn.prepare(sql)?;
    let languages: Vec<LanguageStats> = stmt
        .query_map([], |row| {
            Ok(LanguageStats {
                language: row.get(0)?,
                files: row.get::<_, i64>(1)? as usize,
                lines: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let total_files: usize = languages.iter().map(|l| l.files).sum();
    let total_lines: i64 = languages.iter().map(|l| l.lines).sum();

    // Get function count
    let total_functions: i64 = conn
        .query_row("SELECT COUNT(*) FROM function_facts", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(Summary {
        total_files,
        total_lines,
        total_functions,
        languages,
    })
}

fn collect_architecture(
    options: &ReportOptions,
) -> Result<(Vec<ArchitectureSection>, ToolPerformance)> {
    let queries = [
        "what are the main architectural components",
        "how is the codebase organized",
        "what are the core abstractions",
    ];

    let mut sections = Vec::new();
    let mut total_results = 0;
    let mut empty_count = 0;

    for query in queries {
        let scry_options = ScryOptions {
            limit: 5,
            min_score: 0.3,
            include_persona: false,
            repo: options.repo.clone(),
            ..Default::default()
        };

        let results = scry(query, &scry_options).unwrap_or_default();

        let is_empty = results.is_empty();
        if is_empty {
            empty_count += 1;
        }
        total_results += results.len();

        let arch_results: Vec<ArchResult> = results
            .into_iter()
            .map(|r| ArchResult {
                source: r.source_id,
                content: truncate_content(&r.content, 150),
                score: r.score,
            })
            .collect();

        sections.push(ArchitectureSection {
            query: query.to_string(),
            results: arch_results,
            empty: is_empty,
        });
    }

    let tool_perf = ToolPerformance {
        scry_queries_run: queries.len(),
        scry_empty_results: empty_count,
        scry_avg_results: total_results as f32 / queries.len() as f32,
    };

    Ok((sections, tool_perf))
}

fn collect_largest_modules(conn: &Connection, total_lines: i64) -> Result<Vec<ModuleInfo>> {
    let sql = r#"
        SELECT
            i.path,
            COALESCE(i.line_count, 0) as lines,
            COALESCE((SELECT COUNT(*) FROM function_facts WHERE file = i.path), 0) as functions
        FROM index_state i
        WHERE i.line_count > 0
        ORDER BY lines DESC
        LIMIT 10
    "#;

    let mut stmt = conn.prepare(sql)?;
    let modules: Vec<ModuleInfo> = stmt
        .query_map([], |row| {
            let lines: i64 = row.get(1)?;
            Ok(ModuleInfo {
                path: row.get(0)?,
                lines,
                functions: row.get(2)?,
                percent_of_total: if total_lines > 0 {
                    (lines as f32 / total_lines as f32) * 100.0
                } else {
                    0.0
                },
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(modules)
}

fn collect_rag_health(conn: &Connection) -> Result<RagHealth> {
    // Get last scrape time
    let last_scrape: Option<String> = conn
        .query_row(
            "SELECT value FROM scrape_meta WHERE key = 'last_processed_git'",
            [],
            |row| row.get(0),
        )
        .ok();

    // Count events by type
    let total_events: i64 = conn
        .query_row("SELECT COUNT(*) FROM eventlog", [], |row| row.get(0))
        .unwrap_or(0);

    let code_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'code.%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let git_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'git.%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let session_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'session.%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Count indexed files
    let indexed_files: i64 = conn
        .query_row("SELECT COUNT(*) FROM index_state", [], |row| row.get(0))
        .unwrap_or(0);

    // Check for vector indices
    let embeddings_dir = Path::new(".patina/data/embeddings");
    let mut vector_dimensions = Vec::new();

    if embeddings_dir.exists() {
        for dim in ["semantic", "temporal", "dependency"] {
            // Look for .usearch files in any model subdirectory
            let pattern = embeddings_dir
                .join("*/projections")
                .join(format!("{}.usearch", dim));
            if let Ok(entries) = glob::glob(pattern.to_string_lossy().as_ref()) {
                if entries.filter_map(|e| e.ok()).next().is_some() {
                    vector_dimensions.push(dim.to_string());
                }
            }
        }
    }

    Ok(RagHealth {
        last_scrape,
        total_events,
        code_events,
        git_events,
        session_events,
        indexed_files,
        has_vectors: !vector_dimensions.is_empty(),
        vector_dimensions,
    })
}

// ============================================================================
// Markdown Rendering
// ============================================================================

fn render_markdown(report: &Report) -> String {
    let mut md = String::new();

    // Header
    md.push_str(&format!(
        "# Project State Report: {}\n\n",
        report.project_name
    ));
    md.push_str(&format!("**Generated:** {}\n", report.generated));
    md.push_str("**By:** patina report v0.1.0\n\n");

    // Summary
    md.push_str("## Summary\n\n");
    md.push_str("| Metric | Value |\n");
    md.push_str("|--------|-------|\n");
    md.push_str(&format!("| Files | {} |\n", report.summary.total_files));
    md.push_str(&format!(
        "| Lines of code | {} |\n",
        report.summary.total_lines
    ));
    md.push_str(&format!(
        "| Functions | {} |\n",
        report.summary.total_functions
    ));
    md.push('\n');

    // Languages breakdown
    if !report.summary.languages.is_empty() {
        md.push_str("### Languages\n\n");
        md.push_str("| Language | Files | Lines |\n");
        md.push_str("|----------|-------|-------|\n");
        for lang in &report.summary.languages {
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                lang.language, lang.files, lang.lines
            ));
        }
        md.push('\n');
    }

    // Architecture
    md.push_str("## Architecture\n\n");
    md.push_str("*Via scry semantic search*\n\n");

    for section in &report.architecture {
        md.push_str(&format!("### Query: \"{}\"\n\n", section.query));
        if section.empty {
            md.push_str("*No results - this may indicate a gap in the knowledge base*\n\n");
        } else {
            for (i, result) in section.results.iter().enumerate() {
                md.push_str(&format!(
                    "{}. **{}** (score: {:.2})\n   {}\n\n",
                    i + 1,
                    result.source,
                    result.score,
                    result.content
                ));
            }
        }
    }

    // Largest modules
    md.push_str("## Largest Modules\n\n");
    md.push_str("| Module | Lines | Functions | % of Total |\n");
    md.push_str("|--------|-------|-----------|------------|\n");
    for module in &report.largest_modules {
        md.push_str(&format!(
            "| {} | {} | {} | {:.1}% |\n",
            truncate_path(&module.path, 50),
            module.lines,
            module.functions,
            module.percent_of_total
        ));
    }
    md.push('\n');

    // RAG Health
    md.push_str("## RAG Index Health\n\n");
    md.push_str("| Metric | Value |\n");
    md.push_str("|--------|-------|\n");
    md.push_str(&format!(
        "| Last scrape | {} |\n",
        report.rag_health.last_scrape.as_deref().unwrap_or("never")
    ));
    md.push_str(&format!(
        "| Total events | {} |\n",
        report.rag_health.total_events
    ));
    md.push_str(&format!(
        "| Code events | {} |\n",
        report.rag_health.code_events
    ));
    md.push_str(&format!(
        "| Git events | {} |\n",
        report.rag_health.git_events
    ));
    md.push_str(&format!(
        "| Session events | {} |\n",
        report.rag_health.session_events
    ));
    md.push_str(&format!(
        "| Indexed files | {} |\n",
        report.rag_health.indexed_files
    ));
    md.push_str(&format!(
        "| Vector indices | {} |\n",
        if report.rag_health.has_vectors {
            report.rag_health.vector_dimensions.join(", ")
        } else {
            "none".to_string()
        }
    ));
    md.push('\n');

    // Tool Performance
    md.push_str("## Tool Performance\n\n");
    md.push_str("| Tool | Metric | Value |\n");
    md.push_str("|------|--------|-------|\n");
    md.push_str(&format!(
        "| scry | Queries run | {} |\n",
        report.tool_performance.scry_queries_run
    ));
    md.push_str(&format!(
        "| scry | Empty results | {} |\n",
        report.tool_performance.scry_empty_results
    ));
    md.push_str(&format!(
        "| scry | Avg results/query | {:.1} |\n",
        report.tool_performance.scry_avg_results
    ));
    md.push('\n');

    md.push_str("---\n\n");
    md.push_str("*Report quality = tool quality. Empty results indicate opportunities to improve patina's understanding.*\n");

    md
}

// ============================================================================
// File Operations
// ============================================================================

fn get_output_path(options: &ReportOptions) -> Result<String> {
    if let Some(ref path) = options.output {
        return Ok(path.clone());
    }

    // Default: layer/surface/reports/YYYY-MM-DD-state.md
    let date = Utc::now().format("%Y-%m-%d");
    let reports_dir = "layer/surface/reports";

    // Create directory if needed
    fs::create_dir_all(reports_dir)?;

    Ok(format!("{}/{}-state.md", reports_dir, date))
}

fn save_report(path: &str, content: &str) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)?;
    Ok(())
}

// ============================================================================
// Utilities
// ============================================================================

fn truncate_content(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ").replace("  ", " ");
    if s.len() <= max_len {
        s
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn truncate_path(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max_len + 3..])
    }
}
