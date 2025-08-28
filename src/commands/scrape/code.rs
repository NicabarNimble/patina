// ============================================================================
// SEMANTIC CODE EXTRACTION PIPELINE
// ============================================================================
//! # Code → Knowledge ETL Pipeline
//! 
//! Transforms source code into queryable semantic knowledge using tree-sitter.
//! 
//! ## Purpose
//! This module implements a pure ETL (Extract, Transform, Load) pipeline:
//! - **Extract**: Read source files and git history
//! - **Transform**: Parse ASTs, calculate complexity, detect patterns
//! - **Load**: Store in DuckDB for querying via Ask command
//!
//! ## Database Tables
//! - `code_fingerprints`: AST patterns and complexity metrics
//! - `function_facts`: Behavioral signals (async, unsafe, mutability)
//! - `git_metrics`: Code survival and evolution tracking
//! - `call_graph`: Function dependency relationships
//! - `documentation`: Extracted doc comments with keywords
//!
//! ## Supported Languages
//! Rust, Go, Python, JavaScript, TypeScript, Solidity
//!
//! ## Usage
//! ```bash
//! patina scrape code           # Index current directory
//! patina scrape code --force   # Rebuild from scratch
//! patina scrape code --repo x  # Index layer/dust/repos/x
//! ```

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::ScrapeConfig;

// ============================================================================
// CHAPTER 1: PUBLIC INTERFACE
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
    
    // Create with schema
    create_database_with_schema(&config.db_path)?;
    
    println!("✅ Database initialized with 16KB blocks at {}", config.db_path);
    println!("\nNext steps:");
    println!("  1. Run 'patina scrape code' to index your codebase");
    println!("  2. Run 'patina scrape code --query \"SELECT ...\"' to explore");
    
    Ok(())
}

/// Extract semantic information from codebase
pub fn extract(config: &ScrapeConfig) -> Result<()> {
    println!("🔍 Starting semantic extraction...\n");
    
    let work_dir = determine_work_directory(config)?;
    
    if config.force {
        initialize(config)?;
    }
    
    // Run the ETL pipeline
    run_pipeline(&config.db_path, &work_dir)?;
    
    Ok(())
}

/// Query the knowledge database (temporary - should move to Ask)
pub fn query(config: &ScrapeConfig, sql: &str) -> Result<()> {
    let output = Command::new("duckdb")
        .arg(&config.db_path)
        .arg("-c")
        .arg(sql)
        .output()
        .context("Failed to execute query")?;
    
    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        anyhow::bail!("Query failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(())
}

// ============================================================================
// CHAPTER 2: ETL PIPELINE ORCHESTRATION
// ============================================================================

fn run_pipeline(db_path: &str, work_dir: &Path) -> Result<()> {
    // Phase 1: Git metrics for code survival analysis
    println!("📊 Phase 1: Analyzing git history...");
    extract_and_load_git_metrics(db_path, work_dir)?;
    
    // Phase 2: Pattern references from documentation
    println!("🔗 Phase 2: Extracting pattern references...");
    extract_and_load_pattern_references(db_path, work_dir)?;
    
    // Phase 3: Semantic analysis with tree-sitter
    println!("🧠 Phase 3: Parsing and analyzing code...");
    extract_and_load_semantic_data(db_path, work_dir)?;
    
    // Phase 4: Summary
    println!("\n📈 Phase 4: Generating summary...");
    show_extraction_summary(db_path)?;
    
    Ok(())
}

// ============================================================================
// CHAPTER 3: EXTRACTION - Git Metrics
// ============================================================================

fn extract_and_load_git_metrics(db_path: &str, work_dir: &Path) -> Result<()> {
    println!("📊 Analyzing Git history...");

    let rust_files = Command::new("git")
        .current_dir(work_dir)
        .args(["ls-files", "*.rs", "src/**/*.rs"])
        .output()
        .context("Failed to list Git files")?;

    if !rust_files.status.success() {
        anyhow::bail!("Failed to get file list from Git");
    }

    let files = String::from_utf8_lossy(&rust_files.stdout);
    let file_count = files.lines().count();

    let mut metrics_sql = String::from("BEGIN TRANSACTION;\n");
    metrics_sql.push_str("DELETE FROM git_metrics;\n");

    for file in files.lines() {
        if file.is_empty() {
            continue;
        }

        // Get commit history for this file
        let log_output = Command::new("git")
            .current_dir(work_dir)
            .args(["log", "--format=%H %ai", "--follow", "--", file])
            .output()?;

        if log_output.status.success() {
            let log = String::from_utf8_lossy(&log_output.stdout);
            let commits: Vec<&str> = log.lines().collect();

            if !commits.is_empty() {
                let first = commits
                    .last()
                    .unwrap_or(&"")
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                let last = commits
                    .first()
                    .unwrap_or(&"")
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                let count = commits.len();

                // Calculate survival days
                let first_date = Command::new("git")
                    .current_dir(work_dir)
                    .args(["show", "-s", "--format=%at", first])
                    .output()?;

                if first_date.status.success() {
                    let timestamp = String::from_utf8_lossy(&first_date.stdout)
                        .trim()
                        .parse::<i64>()
                        .unwrap_or(0);

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs() as i64;

                    let survival_days = (now - timestamp) / 86400;

                    metrics_sql.push_str(&format!(
                        "INSERT INTO git_metrics (file, first_commit, last_commit, commit_count, survival_days) VALUES ('{}', '{}', '{}', {}, {});\n",
                        file, first, last, count, survival_days
                    ));
                }
            }
        }
    }

    metrics_sql.push_str("COMMIT;\n");

    Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(&metrics_sql)
        .output()
        .context("Failed to insert Git metrics")?;

    println!("  ✓ Analyzed {} files", file_count);
    Ok(())
}

// ============================================================================
// CHAPTER 4: EXTRACTION - Pattern References
// ============================================================================

fn extract_and_load_pattern_references(db_path: &str, work_dir: &Path) -> Result<()> {
    println!("🔗 Extracting pattern references...");

    let pattern_files = Command::new("find")
        .current_dir(work_dir)
        .args(["layer", "-name", "*.md", "-type", "f"])
        .output()
        .context("Failed to find pattern files")?;

    if !pattern_files.status.success() {
        anyhow::bail!("Failed to list pattern files");
    }

    let files = String::from_utf8_lossy(&pattern_files.stdout);
    let mut references_sql = String::from("BEGIN TRANSACTION;\n");
    references_sql.push_str("DELETE FROM pattern_references;\n");

    for file in files.lines() {
        if file.is_empty() {
            continue;
        }

        let pattern_id = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let file_path = work_dir.join(file);
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            // Look for references in YAML frontmatter
            if let Some(refs_line) = content.lines().find(|l| l.starts_with("references:")) {
                if let Some(refs) = refs_line.strip_prefix("references:") {
                    let refs = refs.trim().trim_start_matches('[').trim_end_matches(']');
                    for reference in refs.split(',') {
                        let reference = reference.trim().trim_matches('"').trim_matches('\'');
                        if !reference.is_empty() {
                            references_sql.push_str(&format!(
                                "INSERT INTO pattern_references (from_pattern, to_pattern, reference_type, context) VALUES ('{}', '{}', 'references', 'frontmatter');\n",
                                pattern_id, reference
                            ));
                        }
                    }
                }
            }
        }
    }

    references_sql.push_str("COMMIT;\n");

    Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(&references_sql)
        .output()
        .context("Failed to insert pattern references")?;

    println!(
        "  ✓ Extracted references from {} patterns",
        files.lines().count()
    );
    Ok(())
}

// ============================================================================
// CHAPTER 5: EXTRACTION - Semantic Data
// ============================================================================

fn extract_and_load_semantic_data(db_path: &str, work_dir: &Path) -> Result<()> {
    extract_fingerprints(db_path, work_dir, false)
}

/// Extract semantic fingerprints with tree-sitter
fn extract_fingerprints(db_path: &str, work_dir: &Path, force: bool) -> Result<()> {
    println!("🧠 Generating semantic fingerprints and extracting truth data...");

    use crate::commands::scrape::code::languages::{create_parser_for_path, Language};
    use crate::commands::incremental;
    use ignore::WalkBuilder;
    use std::collections::HashMap;
    use std::time::SystemTime;

    // Find all supported language files
    let mut all_files = Vec::new();

    // Track skipped files by extension
    let mut skipped_files: HashMap<String, (usize, usize, String)> = HashMap::new(); // ext -> (count, bytes, example_path)

    // Use ignore crate to walk files, respecting .gitignore
    let walker = WalkBuilder::new(work_dir)
        .hidden(false) // Don't process hidden files
        .git_ignore(true) // Respect .gitignore
        .git_global(true) // Respect global gitignore
        .git_exclude(true) // Respect .git/info/exclude
        .ignore(true) // Respect .ignore files
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
            continue;
        }

        // Get relative path for storage
        let relative_path = path.strip_prefix(work_dir).unwrap_or(path);
        let relative_path_str = relative_path.to_string_lossy();

        // Skip if path starts with dot (hidden)
        if relative_path_str.starts_with('.') {
            continue;
        }

        // Determine language from extension
        let language = Language::from_path(path);

        match language {
            Language::Rust
            | Language::Go
            | Language::Solidity
            | Language::Python
            | Language::JavaScript
            | Language::JavaScriptJSX
            | Language::TypeScript
            | Language::TypeScriptTSX => {
                // Supported language - add to processing list with relative path
                all_files.push((format!("./{}", relative_path_str), language));
            }
            Language::Unknown => {
                // Track skipped file
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    // Get file size
                    let file_size = entry.metadata().ok().map(|m| m.len() as usize).unwrap_or(0);

                    let entry = skipped_files.entry(ext.to_string()).or_insert((
                        0,
                        0,
                        relative_path_str.to_string(),
                    ));
                    entry.0 += 1; // count
                    entry.1 += file_size; // bytes
                                          // Keep first example path
                }
            }
        }
    }

    if all_files.is_empty() {
        println!("  ⚠️  No supported language files found");
        return Ok(());
    }

    // Print summary of found files
    print_file_summary(&all_files);

    // Build map of current files with mtimes for incremental updates
    let current_files = build_file_map(work_dir, &all_files)?;

    // Handle incremental vs full index
    let files_to_process = if force {
        println!("  ⚡ Force flag set - performing full re-index");
        cleanup_for_full_reindex(db_path)?;
        all_files
    } else {
        handle_incremental_update(db_path, &current_files, &all_files)?
    };

    if files_to_process.is_empty() {
        println!("  ✅ All files up to date");
        return Ok(());
    }

    // Process files and extract semantic data
    process_files_batch(db_path, work_dir, files_to_process)?;

    // Save and report skipped files
    if !skipped_files.is_empty() {
        save_and_report_skipped_files(db_path, &skipped_files)?;
    }

    Ok(())
}

// Helper functions for semantic extraction
fn print_file_summary(all_files: &[(String, languages::Language)]) {
    use languages::Language;
    
    println!(
        "  📂 Found {} files ({} Rust, {} Go, {} Solidity, {} Python, {} JS, {} JSX, {} TS, {} TSX)",
        all_files.len(),
        all_files.iter().filter(|(_, l)| *l == Language::Rust).count(),
        all_files.iter().filter(|(_, l)| *l == Language::Go).count(),
        all_files.iter().filter(|(_, l)| *l == Language::Solidity).count(),
        all_files.iter().filter(|(_, l)| *l == Language::Python).count(),
        all_files.iter().filter(|(_, l)| *l == Language::JavaScript).count(),
        all_files.iter().filter(|(_, l)| *l == Language::JavaScriptJSX).count(),
        all_files.iter().filter(|(_, l)| *l == Language::TypeScript).count(),
        all_files.iter().filter(|(_, l)| *l == Language::TypeScriptTSX).count()
    );
}

fn build_file_map(work_dir: &Path, all_files: &[(String, languages::Language)]) -> Result<HashMap<PathBuf, i64>> {
    use std::collections::HashMap;
    use std::time::SystemTime;
    
    let mut current_files = HashMap::new();
    for (file_str, _) in all_files {
        let file_path = work_dir.join(file_str);
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                let mtime = modified
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                current_files.insert(PathBuf::from(file_str), mtime);
            }
        }
    }
    Ok(current_files)
}

fn cleanup_for_full_reindex(db_path: &str) -> Result<()> {
    Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg("DELETE FROM code_fingerprints; DELETE FROM code_search; DELETE FROM index_state;")
        .output()?;
    Ok(())
}

fn handle_incremental_update(
    db_path: &str,
    current_files: &HashMap<PathBuf, i64>,
    all_files: &[(String, languages::Language)]
) -> Result<Vec<(String, languages::Language)>> {
    use crate::commands::incremental;
    
    // Detect changes for incremental update
    let changes = incremental::detect_changes(db_path, current_files)?;
    incremental::print_change_summary(&changes);

    // If no changes, we're done!
    if changes.is_empty() {
        return Ok(Vec::new());
    }

    // Clean up changed files
    incremental::cleanup_changed_files(db_path, &changes)?;

    // Build list of files to process
    let mut files_to_process = Vec::new();
    for path in changes.new_files.iter().chain(changes.modified_files.iter()) {
        let path_str = path.to_string_lossy().to_string();
        if let Some((_, lang)) = all_files.iter().find(|(f, _)| f == &path_str) {
            files_to_process.push((path_str, *lang));
        }
    }
    
    Ok(files_to_process)
}

fn process_files_batch(
    db_path: &str,
    work_dir: &Path,
    files_to_process: Vec<(String, languages::Language)>
) -> Result<()> {
    use languages::{create_parser_for_path, Language};
    use std::time::SystemTime;
    
    let mut sql = String::from("BEGIN TRANSACTION;\n");
    let mut symbol_count = 0;
    let mut current_lang = Language::Unknown;
    let mut parser: Option<tree_sitter::Parser> = None;
    let mut batch_count = 0;

    // Process only new and modified files
    for (file, language) in files_to_process {
        // Check if file needs reindexing (mtime-based incremental)
        let file_path = work_dir.join(&file);

        // Create parser for this specific file path
        // This correctly handles TSX vs TS and JSX vs JS distinctions
        if language != current_lang {
            parser = Some(create_parser_for_path(&file_path)?);
            current_lang = language;
        }
        let metadata = std::fs::metadata(&file_path)?;
        let mtime = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64;

        // Parse and fingerprint
        let content = std::fs::read_to_string(&file_path)?;
        if let Some(ref mut p) = parser {
            if let Some(tree) = p.parse(&content, None) {
                let mut cursor = tree.walk();
                let mut context = ParseContext::new();
                symbol_count += process_ast_node(
                    &mut cursor,
                    content.as_bytes(),
                    &file,
                    &mut sql,
                    language,
                    &mut context,
                );

                // Flush call graph entries for this file
                context.flush_to_sql(&file, &mut sql);

                // Record index state
                sql.push_str(&format!(
                    "INSERT INTO index_state (path, mtime) VALUES ('{}', {});\n",
                    file, mtime
                ));
            }
        }

        // Batch execute every 10 files to avoid command line limits
        batch_count += 1;
        if batch_count >= 10 {
            sql.push_str("COMMIT;\n");

            // Use stdin to avoid command line length limits
            let mut child = Command::new("duckdb")
                .arg(db_path)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to start DuckDB")?;

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin
                    .write_all(sql.as_bytes())
                    .context("Failed to write SQL")?;
            }

            let output = child
                .wait_with_output()
                .context("Failed to execute batch")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("DuckDB error: {}", stderr);
            }
            sql = String::from("BEGIN TRANSACTION;\n");
            batch_count = 0;
        }
    }

    // Execute final batch
    if batch_count > 0 {
        sql.push_str("COMMIT;\n");

        // Use stdin to avoid command line length limits
        let mut child = Command::new("duckdb")
            .arg(db_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start DuckDB")?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(sql.as_bytes())
                .context("Failed to write SQL")?;
        }

        let output = child
            .wait_with_output()
            .context("Failed to execute batch")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Final batch had issues: {}", stderr);
        }
    }

    println!("  ✓ Fingerprinted {} symbols", symbol_count);
    Ok(())
}

fn save_and_report_skipped_files(
    db_path: &str,
    skipped_files: &HashMap<String, (usize, usize, String)>
) -> Result<()> {
    save_skipped_files(db_path, skipped_files)?;
    report_skipped_files(skipped_files);
    Ok(())
}

/// Save skipped files to database
fn save_skipped_files(
    db_path: &str,
    skipped: &HashMap<String, (usize, usize, String)>,
) -> Result<()> {
    let mut sql = String::from("BEGIN TRANSACTION;\n");
    sql.push_str("DELETE FROM skipped_files;\n");

    for (ext, (count, bytes, example)) in skipped {
        // Map common extensions to language names
        let lang_name = match ext.as_str() {
            "py" => "Python",
            "js" => "JavaScript",
            "ts" => "TypeScript",
            "jsx" => "React JSX",
            "tsx" => "React TSX",
            "java" => "Java",
            "c" => "C",
            "cpp" | "cc" | "cxx" => "C++",
            "h" | "hpp" => "C/C++ Header",
            "cs" => "C#",
            "rb" => "Ruby",
            "php" => "PHP",
            "swift" => "Swift",
            "kt" => "Kotlin",
            "scala" => "Scala",
            "ml" => "OCaml",
            "hs" => "Haskell",
            "ex" | "exs" => "Elixir",
            "clj" => "Clojure",
            "vue" => "Vue",
            "svelte" => "Svelte",
            "lua" => "Lua",
            "r" => "R",
            "jl" => "Julia",
            "zig" => "Zig",
            "nim" => "Nim",
            "dart" => "Dart",
            "sh" | "bash" => "Shell",
            "yaml" | "yml" => "YAML",
            "json" => "JSON",
            "toml" => "TOML",
            "xml" => "XML",
            "md" => "Markdown",
            _ => "",
        };

        sql.push_str(&format!(
            "INSERT INTO skipped_files (extension, file_count, total_bytes, example_path, common_name) VALUES ('{}', {}, {}, '{}', '{}');\n",
            ext, count, bytes, example.replace('\'', "''"), lang_name
        ));
    }

    sql.push_str("COMMIT;\n");

    // Execute via stdin
    let mut child = Command::new("duckdb")
        .arg(db_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB")?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(sql.as_bytes())
            .context("Failed to write SQL")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to save skipped files")?;
    if !output.status.success() {
        eprintln!("Warning: Failed to save skipped files stats");
    }

    Ok(())
}

/// Report skipped files to user
fn report_skipped_files(skipped: &HashMap<String, (usize, usize, String)>) {
    // Sort by file count descending
    let mut sorted: Vec<_> = skipped.iter().collect();
    sorted.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    println!("\n⚠️  Skipped files (no parser available):");

    // Show top 5 most common extensions
    for (ext, (count, bytes, _)) in sorted.iter().take(5) {
        let size_mb = *bytes as f64 / 1_048_576.0;
        println!("   {} .{} files ({:.1} MB)", count, ext, size_mb);
    }

    if sorted.len() > 5 {
        let remaining: usize = sorted.iter().skip(5).map(|(_, (c, _, _))| c).sum();
        println!("   {} files with other extensions", remaining);
    }

    // Suggest adding parsers for common languages
    let suggestions: Vec<&str> = sorted
        .iter()
        .filter_map(|(ext, (count, _, _))| {
            if *count > 10 {
                match ext.as_str() {
                    "py" => Some("Python"),
                    "js" | "ts" | "jsx" | "tsx" => Some("JavaScript/TypeScript"),
                    "java" => Some("Java"),
                    "c" | "cpp" | "h" => Some("C/C++"),
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if !suggestions.is_empty() {
        println!(
            "\n💡 Consider adding parsers for: {}",
            suggestions.join(", ")
        );
    }
}

// ============================================================================
// CHAPTER 6: DATABASE OPERATIONS
// ============================================================================

fn create_database_with_schema(db_path: &str) -> Result<()> {
    let schema_sql = schema::generate_complete_schema(db_path);
    
    // Execute via stdin to avoid command line escaping issues
    let mut child = Command::new("duckdb")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB. Is duckdb installed?")?;
    
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(schema_sql.as_bytes())?;
    }
    
    let output = child.wait_with_output()?;
    
    if !output.status.success() {
        anyhow::bail!(
            "Failed to create database: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    
    Ok(())
}

fn show_extraction_summary(db_path: &str) -> Result<()> {
    println!("\n📈 Summary:");

    let summary_query = r#"
SELECT 
    'Functions indexed' as metric,
    COUNT(*) as value
FROM code_fingerprints
WHERE kind = 'function'
UNION ALL
SELECT 
    'Average complexity' as metric,
    CAST(AVG(complexity) AS INTEGER) as value
FROM code_fingerprints
WHERE kind = 'function'
UNION ALL
SELECT 
    'Unique patterns' as metric,
    COUNT(DISTINCT pattern) as value
FROM code_fingerprints
UNION ALL
SELECT 
    'Files with 10+ commits' as metric,
    COUNT(*) as value
FROM git_metrics
WHERE commit_count >= 10
UNION ALL
SELECT
    'Languages skipped' as metric,
    COUNT(*) as value
FROM skipped_files
WHERE file_count > 0;
"#;

    let output = Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(summary_query)
        .output()
        .context("Failed to query summary")?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    // Show database size and block info
    let size_query = "PRAGMA database_size;";
    let size_output = Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(size_query)
        .output()?;

    if size_output.status.success() {
        println!("\n💾 Database info:");
        println!("{}", String::from_utf8_lossy(&size_output.stdout));
    }

    // Also show file size
    if let Ok(metadata) = std::fs::metadata(db_path) {
        let size_kb = metadata.len() / 1024;
        println!("📁 File size: {}KB", size_kb);
    }

    Ok(())
}

// ============================================================================
// CHAPTER 7: UTILITIES
// ============================================================================

fn determine_work_directory(config: &ScrapeConfig) -> Result<PathBuf> {
    // Extract repo name from db_path if it's in layer/dust/repos/
    if config.db_path.contains("layer/dust/repos/") {
        let repo_name = config.db_path
            .strip_prefix("layer/dust/repos/")
            .and_then(|s| s.strip_suffix(".db"))
            .context("Invalid repo database path")?;
        
        let repo_dir = PathBuf::from("layer/dust/repos").join(repo_name);
        if !repo_dir.exists() {
            anyhow::bail!("Repository '{}' not found. Clone it first to layer/dust/repos/", repo_name);
        }
        
        Ok(std::env::current_dir()?.join(repo_dir))
    } else {
        Ok(std::env::current_dir()?)
    }
}

/// Escape SQL strings
fn escape_sql(s: &str) -> String {
    s.replace('\'', "''")
}

/// Context for tracking state during AST traversal
struct ParseContext {
    current_function: Option<String>,
    call_graph_entries: Vec<(String, String, String, i32)>, // (caller, callee, call_type, line)
}

impl ParseContext {
    fn new() -> Self {
        Self {
            current_function: None,
            call_graph_entries: Vec::new(),
        }
    }

    fn enter_function(&mut self, name: String) {
        self.current_function = Some(name);
    }

    fn exit_function(&mut self) {
        self.current_function = None;
    }

    fn add_call(&mut self, callee: String, call_type: String, line: i32) {
        if let Some(ref caller) = self.current_function {
            self.call_graph_entries
                .push((caller.clone(), callee, call_type, line));
        }
    }

    fn flush_to_sql(&mut self, file_path: &str, sql: &mut String) {
        for (caller, callee, call_type, line) in &self.call_graph_entries {
            sql.push_str(&format!(
                "INSERT INTO call_graph (caller, callee, file, call_type, line_number) VALUES ('{}', '{}', '{}', '{}', {});\n",
                escape_sql(caller),
                escape_sql(callee),
                file_path,
                call_type,
                line
            ));
        }
        self.call_graph_entries.clear();
    }
}

// ============================================================================
// MODULE: Database Schema
// ============================================================================

mod schema {
    pub fn generate_complete_schema(db_path: &str) -> String {
        format!(
            r#"
-- Attach with minimal block size (16KB instead of default 256KB)
ATTACH '{}' AS knowledge (BLOCK_SIZE 16384);
USE knowledge;

{}

-- Git survival metrics for quality assessment
CREATE TABLE IF NOT EXISTS git_metrics (
    file VARCHAR PRIMARY KEY,
    first_commit VARCHAR,
    last_commit VARCHAR,
    commit_count INTEGER,
    survival_days INTEGER
);

-- Pattern references extracted from documentation
CREATE TABLE IF NOT EXISTS pattern_references (
    from_pattern VARCHAR NOT NULL,
    to_pattern VARCHAR NOT NULL,
    reference_type VARCHAR NOT NULL,
    context VARCHAR,
    PRIMARY KEY (from_pattern, to_pattern, reference_type)
);
"#,
            db_path,
            all_tables_schema()
        )
    }
    
    fn all_tables_schema() -> &'static str {
        r#"
-- Compact fingerprint storage (columnar for SIMD)
CREATE TABLE IF NOT EXISTS code_fingerprints (
    path VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    kind VARCHAR NOT NULL,  -- function, struct, trait, impl
    pattern UINTEGER,       -- AST shape hash
    imports UINTEGER,       -- Dependency hash  
    complexity USMALLINT,   -- Cyclomatic complexity
    flags USMALLINT,        -- Feature bitmask
    PRIMARY KEY (path, name, kind)
);

-- Full-text search for actual code search
CREATE TABLE IF NOT EXISTS code_search (
    path VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    signature VARCHAR,      -- Function/struct signature
    context VARCHAR,        -- Surrounding code snippet
    PRIMARY KEY (path, name)
);

-- Type vocabulary: The domain language (compiler-verified truth)
CREATE TABLE IF NOT EXISTS type_vocabulary (
    file VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    definition TEXT,        -- 'type NodeId = u32' or 'struct User { ... }'
    kind VARCHAR,          -- 'type_alias', 'struct', 'enum', 'const'
    visibility VARCHAR,     -- 'pub', 'pub(crate)', 'private'
    usage_count INTEGER DEFAULT 0,
    PRIMARY KEY (file, name)
);

-- Function facts: Behavioral signals without interpretation
CREATE TABLE IF NOT EXISTS function_facts (
    file VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    takes_mut_self BOOLEAN,     -- Thread safety signal
    takes_mut_params BOOLEAN,   -- Mutation indicator
    returns_result BOOLEAN,     -- Error handling
    returns_option BOOLEAN,     -- Nullability
    is_async BOOLEAN,          -- Concurrency
    is_unsafe BOOLEAN,         -- Safety requirements
    is_public BOOLEAN,         -- API surface
    parameter_count INTEGER,
    generic_count INTEGER,      -- Complexity indicator
    parameters TEXT,            -- Parameter names and types
    return_type TEXT,           -- Return type signature
    PRIMARY KEY (file, name)
);

-- Import facts: Navigation and dependencies
CREATE TABLE IF NOT EXISTS import_facts (
    importer_file VARCHAR NOT NULL,
    imported_item VARCHAR NOT NULL,
    imported_from VARCHAR,      -- Source module/crate
    is_external BOOLEAN,       -- External crate?
    import_kind VARCHAR,        -- 'use', 'mod', 'extern'
    PRIMARY KEY (importer_file, imported_item)
);

-- Documentation: Searchable docs with keywords for LLM context retrieval
CREATE TABLE IF NOT EXISTS documentation (
    file VARCHAR NOT NULL,
    symbol_name VARCHAR NOT NULL,
    symbol_type VARCHAR,        -- 'function', 'struct', 'module', 'field'
    line_number INTEGER,
    doc_raw TEXT,              -- Original with comment markers
    doc_clean TEXT,            -- Cleaned text for display
    doc_summary VARCHAR,       -- First sentence (fast preview)
    keywords VARCHAR[],        -- Extracted keywords for search
    doc_length INTEGER,        -- Character count
    has_examples BOOLEAN,      -- Contains code blocks
    has_params BOOLEAN,        -- Documents parameters
    parent_symbol VARCHAR,     -- For nested items (methods in impl blocks)
    PRIMARY KEY (file, symbol_name)
);

-- Call graph: Function relationships for context traversal
CREATE TABLE IF NOT EXISTS call_graph (
    caller VARCHAR NOT NULL,
    callee VARCHAR NOT NULL,
    file VARCHAR NOT NULL,
    call_type VARCHAR,         -- 'direct', 'method', 'async', 'callback'
    line_number INTEGER        -- Where the call happens
);

CREATE INDEX IF NOT EXISTS idx_caller ON call_graph(caller);
CREATE INDEX IF NOT EXISTS idx_callee ON call_graph(callee);

-- Behavioral hints: Code smell detection (facts only)
CREATE TABLE IF NOT EXISTS behavioral_hints (
    file VARCHAR NOT NULL,
    function VARCHAR NOT NULL,
    calls_unwrap INTEGER DEFAULT 0,     -- Count of .unwrap()
    calls_expect INTEGER DEFAULT 0,     -- Count of .expect()
    has_panic_macro BOOLEAN,           -- Contains panic!()
    has_todo_macro BOOLEAN,            -- Contains todo!()
    has_unsafe_block BOOLEAN,          -- Contains unsafe {}
    has_mutex BOOLEAN,                 -- Thread synchronization
    has_arc BOOLEAN,                   -- Shared ownership
    PRIMARY KEY (file, function)
);

-- Index metadata for incremental updates
CREATE TABLE IF NOT EXISTS index_state (
    path VARCHAR PRIMARY KEY,
    mtime BIGINT NOT NULL,
    hash VARCHAR,           -- File content hash
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Track files we skipped due to missing language support
CREATE TABLE IF NOT EXISTS skipped_files (
    extension VARCHAR PRIMARY KEY,
    file_count INTEGER DEFAULT 0,
    total_bytes INTEGER DEFAULT 0,
    example_path VARCHAR,
    common_name VARCHAR     -- e.g., "Python", "TypeScript"
);

-- Create indexes for fast lookups
CREATE INDEX IF NOT EXISTS idx_fingerprint_pattern ON code_fingerprints(pattern);
CREATE INDEX IF NOT EXISTS idx_fingerprint_complexity ON code_fingerprints(complexity);
CREATE INDEX IF NOT EXISTS idx_fingerprint_flags ON code_fingerprints(flags);
CREATE INDEX IF NOT EXISTS idx_type_vocabulary_kind ON type_vocabulary(kind);
CREATE INDEX IF NOT EXISTS idx_function_facts_public ON function_facts(is_public);
CREATE INDEX IF NOT EXISTS idx_import_facts_external ON import_facts(is_external);
CREATE INDEX IF NOT EXISTS idx_documentation_symbol ON documentation(symbol_name);
CREATE INDEX IF NOT EXISTS idx_documentation_type ON documentation(symbol_type);
"#
    }
}

// ============================================================================
// MODULE: Language Support
// ============================================================================

pub(crate) mod languages {
    use anyhow::{Context, Result};
    use std::path::Path;
    use tree_sitter::Parser;

    /// Supported programming languages
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Language {
        Rust,
        Go,
        Solidity,
        Python,
        JavaScript,
        JavaScriptJSX, // .jsx files
        TypeScript,
        TypeScriptTSX, // .tsx files
        Unknown,
    }

    impl Language {
        /// Detect language from file extension
        pub fn from_path(path: &Path) -> Self {
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("rs") => Language::Rust,
                Some("go") => Language::Go,
                Some("sol") => Language::Solidity,
                Some("py") => Language::Python,
                Some("js") | Some("mjs") => Language::JavaScript,
                Some("jsx") => Language::JavaScriptJSX,
                Some("ts") => Language::TypeScript,
                Some("tsx") => Language::TypeScriptTSX,
                _ => Language::Unknown,
            }
        }

        /// Convert to patina_metal::Metal enum
        pub fn to_metal(self) -> Option<patina_metal::Metal> {
            match self {
                Language::Rust => Some(patina_metal::Metal::Rust),
                Language::Go => Some(patina_metal::Metal::Go),
                Language::Solidity => Some(patina_metal::Metal::Solidity),
                Language::Python => Some(patina_metal::Metal::Python),
                Language::JavaScript | Language::JavaScriptJSX => {
                    Some(patina_metal::Metal::JavaScript)
                }
                Language::TypeScript | Language::TypeScriptTSX => {
                    Some(patina_metal::Metal::TypeScript)
                }
                Language::Unknown => None,
            }
        }
    }

    /// Create a parser for a specific file path, handling TypeScript's tsx vs ts distinction
    pub fn create_parser_for_path(path: &Path) -> Result<Parser> {
        let language = Language::from_path(path);
        let metal = language
            .to_metal()
            .ok_or_else(|| anyhow::anyhow!("Unsupported language: {:?}", language))?;

        // Use the extension-aware method for TypeScript to get the right parser
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ts_lang = metal
            .tree_sitter_language_for_ext(ext)
            .ok_or_else(|| anyhow::anyhow!("No parser available for {:?}", language))?;

        let mut parser = Parser::new();
        parser
            .set_language(&ts_lang)
            .context("Failed to set language")?;

        Ok(parser)
    }
}

// ============================================================================
// MODULE: Fingerprinting
// ============================================================================

pub(crate) mod fingerprint {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use tree_sitter::Node;

    /// Compact 16-byte fingerprint for code patterns
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Fingerprint {
        pub pattern: u32,    // AST shape hash
        pub imports: u32,    // Dependency hash
        pub complexity: u16, // Cyclomatic complexity
        pub flags: u16,      // Feature flags
    }

    impl Fingerprint {
        /// Generate fingerprint from tree-sitter AST node
        pub fn from_ast(node: Node, source: &[u8]) -> Self {
            let pattern = hash_ast_shape(node, source);
            let imports = hash_imports(node, source);
            let complexity = calculate_complexity(node) as u16;
            let flags = detect_features(node, source);

            Self {
                pattern,
                imports,
                complexity,
                flags,
            }
        }
    }

    /// Hash the AST structure (types only, not content)
    fn hash_ast_shape(node: Node, _source: &[u8]) -> u32 {
        let mut hasher = DefaultHasher::new();
        hash_node_shape(&mut hasher, node);
        (hasher.finish() & 0xFFFFFFFF) as u32
    }

    fn hash_node_shape(hasher: &mut impl Hasher, node: Node) {
        // Hash node type (structure, not content)
        node.kind().hash(hasher);

        // Hash child structure recursively
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                hash_node_shape(hasher, cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    /// Hash imports/dependencies
    fn hash_imports(node: Node, source: &[u8]) -> u32 {
        let mut hasher = DefaultHasher::new();
        let mut cursor = node.walk();

        find_imports(&mut cursor, source, &mut hasher);
        (hasher.finish() & 0xFFFFFFFF) as u32
    }

    fn find_imports(cursor: &mut tree_sitter::TreeCursor, source: &[u8], hasher: &mut impl Hasher) {
        let node = cursor.node();

        if node.kind() == "use_declaration" {
            if let Ok(text) = node.utf8_text(source) {
                text.hash(hasher);
            }
        }

        if cursor.goto_first_child() {
            loop {
                find_imports(cursor, source, hasher);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Calculate cyclomatic complexity
    fn calculate_complexity(node: Node) -> usize {
        let mut complexity = 1; // Base complexity
        let mut cursor = node.walk();

        count_branches(&mut cursor, &mut complexity);
        complexity
    }

    fn count_branches(cursor: &mut tree_sitter::TreeCursor, complexity: &mut usize) {
        let node = cursor.node();

        match node.kind() {
            "if_expression" | "match_expression" | "while_expression" | "for_expression" => {
                *complexity += 1;
            }
            "match_arm" => {
                // Each arm adds a branch
                *complexity += 1;
            }
            _ => {}
        }

        if cursor.goto_first_child() {
            loop {
                count_branches(cursor, complexity);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Detect feature flags (async, unsafe, etc.)
    fn detect_features(node: Node, source: &[u8]) -> u16 {
        let mut flags = 0u16;
        let mut cursor = node.walk();

        detect_features_recursive(&mut cursor, source, &mut flags);
        flags
    }

    fn detect_features_recursive(
        cursor: &mut tree_sitter::TreeCursor,
        source: &[u8],
        flags: &mut u16,
    ) {
        let node = cursor.node();

        // Check for various features
        match node.kind() {
            "async" => *flags |= 0x0001,                   // Bit 0: async
            "unsafe_block" | "unsafe" => *flags |= 0x0002, // Bit 1: unsafe
            "macro_invocation" => {
                if let Ok(text) = node.utf8_text(source) {
                    if text.starts_with("panic!") || text.starts_with("unreachable!") {
                        *flags |= 0x0004; // Bit 2: has panic
                    }
                    if text.starts_with("todo!") || text.starts_with("unimplemented!") {
                        *flags |= 0x0008; // Bit 3: has todo
                    }
                }
            }
            "question_mark" => *flags |= 0x0010, // Bit 4: uses ?
            "generic_type" | "generic_function" => *flags |= 0x0020, // Bit 5: generic
            "trait_bounds" => *flags |= 0x0040,  // Bit 6: has trait bounds
            "lifetime" => *flags |= 0x0080,      // Bit 7: has lifetimes
            _ => {}
        }

        if cursor.goto_first_child() {
            loop {
                detect_features_recursive(cursor, source, flags);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

// ============================================================================
// MODULE: AST Processing
// ============================================================================

mod ast_processing {
    // TODO: Move AST processing functions here from original lines 1125-1443
}