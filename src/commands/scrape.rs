use crate::commands::incremental;
use anyhow::{Context, Result};
use patina::semantic::extractor;
use patina::semantic::languages::{create_parser, Language};
use patina::semantic::store::{duckdb::DuckDbStore, KnowledgeStore};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Execute the scrape command to build semantic knowledge database
pub fn execute(init: bool, query: Option<String>, repo: Option<String>, force: bool) -> Result<()> {
    // Determine paths based on whether we're scraping a repo
    let (db_path, work_dir) = if let Some(repo_name) = &repo {
        validate_repo_path(repo_name)?
    } else {
        (".patina/knowledge.db".into(), std::env::current_dir()?)
    };

    // Create store instance
    let store = DuckDbStore::new(&db_path);

    if init {
        println!("🗄️  Initializing optimized knowledge database...");
        store.initialize()?;
        println!("  ✓ Database initialized at {}", db_path);
    } else if let Some(q) = query {
        run_query(&q, &store)?;
    } else {
        extract_and_index(&db_path, &work_dir, force)?;
    }
    Ok(())
}

/// Validate repo exists and return database path and working directory
fn validate_repo_path(repo_name: &str) -> Result<(String, PathBuf)> {
    let repo_dir = PathBuf::from("layer/dust/repos").join(repo_name);

    if !repo_dir.exists() {
        anyhow::bail!(
            "Repository '{}' not found in layer/dust/repos/\n\
             Please clone the repository first:\n\
             mkdir -p layer/dust/repos && cd layer/dust/repos\n\
             git clone <url> {}",
            repo_name,
            repo_name
        );
    }

    if !repo_dir.is_dir() {
        anyhow::bail!("'{}' exists but is not a directory", repo_dir.display());
    }

    let db_path = format!("layer/dust/repos/{}.db", repo_name);
    let work_dir = std::env::current_dir()?.join(repo_dir);

    println!("📦 Scraping repository: {}", repo_name);
    println!("📁 Source: {}", work_dir.display());
    println!("💾 Database: {}", db_path);

    Ok((db_path, work_dir))
}

/// Run a query against the database
fn run_query(query: &str, store: &DuckDbStore) -> Result<()> {
    println!("🔍 Running query...\n");
    let output = store.execute_query(query)?;
    println!("{}", output);
    Ok(())
}

/// Extract and index code fingerprints
fn extract_and_index(db_path: &str, work_dir: &Path, force: bool) -> Result<()> {
    // Create store
    let store = DuckDbStore::new(db_path);

    // Extract all the data
    extract_fingerprints(db_path, work_dir, force, &store)?;
    extract_git_metrics(db_path, work_dir)?;
    extract_pattern_references(db_path, work_dir)?;

    // Print summary
    print_summary(db_path)?;

    Ok(())
}

/// Extract semantic fingerprints with tree-sitter
fn extract_fingerprints(
    db_path: &str,
    work_dir: &Path,
    force: bool,
    store: &DuckDbStore,
) -> Result<()> {
    println!("🧠 Generating semantic fingerprints and extracting truth data...");

    use ignore::WalkBuilder;
    use std::time::SystemTime;

    // Find all supported language files
    let mut all_files = Vec::new();

    // Track skipped files by extension
    let mut skipped_files: HashMap<String, (usize, usize, String)> = HashMap::new();

    // Use ignore crate to walk files, respecting .gitignore
    let walker = WalkBuilder::new(work_dir)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
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
                    let file_size = entry.metadata().ok().map(|m| m.len() as usize).unwrap_or(0);
                    let entry = skipped_files.entry(ext.to_string()).or_insert((
                        0,
                        0,
                        relative_path_str.to_string(),
                    ));
                    entry.0 += 1; // count
                    entry.1 += file_size; // bytes
                }
            }
        }
    }

    if all_files.is_empty() {
        println!("  ⚠️  No supported language files found");
        return Ok(());
    }

    println!("  📂 Found {} files", all_files.len());

    // Build map of current files with mtimes
    let mut current_files = HashMap::new();
    for (file_str, _) in &all_files {
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

    // Handle incremental vs full index
    let files_to_process = if force {
        println!("  ⚡ Force flag set - performing full re-index");

        // Clear all existing data for full re-index
        Command::new("duckdb")
            .arg(db_path)
            .arg("-c")
            .arg("DELETE FROM code_fingerprints; DELETE FROM code_search; DELETE FROM index_state;")
            .output()?;

        all_files
    } else {
        // Detect changes for incremental update
        let changes = incremental::detect_changes(db_path, &current_files)?;
        incremental::print_change_summary(&changes);

        // If no changes, we're done!
        if changes.is_empty() {
            return Ok(());
        }

        // Clean up changed files
        incremental::cleanup_changed_files(db_path, &changes)?;

        // Build list of files to process
        let mut files_to_process = Vec::new();
        for path in changes
            .new_files
            .iter()
            .chain(changes.modified_files.iter())
        {
            let path_str = path.to_string_lossy().to_string();
            if let Some((_, lang)) = all_files.iter().find(|(f, _)| f == &path_str) {
                files_to_process.push((path_str, *lang));
            }
        }
        files_to_process
    };

    let mut symbol_count = 0;
    let mut current_lang = Language::Unknown;
    let mut parser: Option<tree_sitter::Parser> = None;

    // Process files
    for (file, language) in files_to_process {
        // Switch parser if language changed
        if language != current_lang {
            parser = Some(create_parser(language)?);
            current_lang = language;
        }

        let file_path = work_dir.join(&file);
        let source = std::fs::read(&file_path)?;

        // Parse the file
        if let Some(ref mut p) = parser {
            if let Some(tree) = p.parse(&source, None) {
                // Process the AST using our new extractor
                let results = extractor::process_tree(&tree, &source, &file, language);

                // Store all results
                store.store_results(&results, &file)?;

                // Count symbols for reporting
                symbol_count += results.functions.len();
                symbol_count += results.types.len();

                // Update index state
                let metadata = std::fs::metadata(&file_path)?;
                let mtime = metadata
                    .modified()?
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs() as i64;

                let update_sql = format!(
                    "INSERT OR REPLACE INTO index_state (path, mtime) VALUES ('{}', {})",
                    file, mtime
                );
                store.execute_query(&update_sql)?;
            }
        }
    }

    println!("  ✓ Extracted {} symbols", symbol_count);

    // Report skipped files if any
    if !skipped_files.is_empty() {
        report_skipped_files(&skipped_files);
        save_skipped_files_stats(db_path, &skipped_files)?;
    }

    Ok(())
}

/// Extract Git metrics for quality assessment
fn extract_git_metrics(db_path: &str, work_dir: &Path) -> Result<()> {
    println!("📊 Extracting Git survival metrics...");

    // Get all indexed files from database
    let files_query = Command::new("duckdb")
        .arg(db_path)
        .arg("-csv")
        .arg("-c")
        .arg("SELECT DISTINCT file FROM code_fingerprints")
        .output()
        .context("Failed to query indexed files")?;

    if !files_query.status.success() {
        anyhow::bail!("Failed to query indexed files from database");
    }

    let files_output = String::from_utf8_lossy(&files_query.stdout);
    let mut metrics_sql = String::from("BEGIN TRANSACTION;\n");
    metrics_sql.push_str("DELETE FROM git_metrics;\n");

    let mut file_count = 0;

    for line in files_output.lines().skip(1) {
        // Skip CSV header
        if line.is_empty() {
            continue;
        }

        let file = line.trim_start_matches("./");

        // Get commit history for this file
        let log = Command::new("git")
            .current_dir(work_dir)
            .args(["log", "--oneline", "--follow", "--", file])
            .output()?;

        if log.status.success() {
            let log_output = String::from_utf8_lossy(&log.stdout);
            let commits: Vec<_> = log_output.lines().filter(|l| !l.is_empty()).collect();

            if !commits.is_empty() {
                file_count += 1;

                // Get first and last commits
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

/// Extract pattern references from markdown
fn extract_pattern_references(db_path: &str, work_dir: &Path) -> Result<()> {
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

/// Save skipped files statistics to database
fn save_skipped_files_stats(
    db_path: &str,
    skipped: &HashMap<String, (usize, usize, String)>,
) -> Result<()> {
    let mut sql = String::from("BEGIN TRANSACTION;\n");
    sql.push_str("DELETE FROM skipped_files;\n");

    for (ext, (count, bytes, example)) in skipped {
        sql.push_str(&format!(
            "INSERT INTO skipped_files (extension, file_count, total_bytes, example_path) VALUES ('{}', {}, {}, '{}');\n",
            ext, count, bytes, example.replace('\'', "''")
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
}

/// Print database summary
fn print_summary(db_path: &str) -> Result<()> {
    println!("\n📈 Database Summary:");

    let summary = Command::new("duckdb")
        .arg(db_path)
        .arg("-csv")
        .arg("-c")
        .arg(
            "SELECT 
                (SELECT COUNT(*) FROM function_facts) as functions,
                (SELECT COUNT(*) FROM documentation) as documented,
                (SELECT COUNT(*) FROM call_graph) as call_relations,
                (SELECT COUNT(*) FROM type_vocabulary) as types,
                (SELECT COUNT(*) FROM code_fingerprints) as fingerprints",
        )
        .output()
        .context("Failed to query summary")?;

    if summary.status.success() {
        let output = String::from_utf8_lossy(&summary.stdout);
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 5 {
                println!("  📝 Functions: {}", parts[0]);
                println!("  📚 Documentation: {}", parts[1]);
                println!("  🔗 Call relations: {}", parts[2]);
                println!("  🏗️  Types: {}", parts[3]);
                println!("  🎯 Fingerprints: {}", parts[4]);
            }
        }
    }

    // Get database size
    if let Ok(metadata) = std::fs::metadata(db_path) {
        let size_mb = metadata.len() as f64 / 1_048_576.0;
        println!("  💾 Database size: {:.1} MB", size_mb);
    }

    Ok(())
}
