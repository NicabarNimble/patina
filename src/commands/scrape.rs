use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::commands::incremental;

/// Execute the scrape command to build semantic knowledge database
pub fn execute(init: bool, query: Option<String>, repo: Option<String>, force: bool) -> Result<()> {
    // Determine paths based on whether we're scraping a repo
    let (db_path, work_dir) = if let Some(repo_name) = &repo {
        validate_repo_path(&repo_name)?
    } else {
        (".patina/knowledge.db".into(), std::env::current_dir()?)
    };
    
    if init {
        initialize_database(&db_path)?;
    } else if let Some(q) = query {
        run_query(&q, &db_path)?;
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
            repo_name, repo_name
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

/// Initialize DuckDB database with lean schema and optimal settings for small size
fn initialize_database(db_path: &str) -> Result<()> {
    println!("🗄️  Initializing optimized knowledge database...");
    
    // Create parent directory if needed
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Remove old database if exists
    if Path::new(db_path).exists() {
        std::fs::remove_file(db_path)?;
    }
    
    // Create with 16KB block size for minimal overhead
    let init_script = format!(r#"
-- Attach with minimal block size (16KB instead of default 256KB)
ATTACH '{db_path}' AS knowledge (BLOCK_SIZE 16384);
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
"#, patina::semantic::fingerprint::generate_schema(), db_path=db_path);

    // Execute via stdin to avoid command line escaping issues
    let mut child = Command::new("duckdb")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB. Is duckdb installed?")?;
        
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(init_script.as_bytes())?;
    }
    
    let output = child.wait_with_output()?;
    
    if !output.status.success() {
        anyhow::bail!(
            "Failed to create database: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("✅ Database initialized with 16KB blocks at {}", db_path);
    println!("\nNext steps:");
    println!("  1. Run 'patina scrape' to index your codebase");
    println!("  2. Run 'patina scrape --query \"SELECT ...\"' to explore");
    
    Ok(())
}

/// Extract and index code with fingerprints + Git metrics
fn extract_and_index(db_path: &str, work_dir: &Path, force: bool) -> Result<()> {
    println!("🔍 Indexing codebase...\n");
    
    // Step 1: Git metrics for quality signals
    extract_git_metrics(db_path, work_dir)?;
    
    // Step 2: Pattern references from docs (only for main repo)
    if db_path.contains(".patina/") {
        extract_pattern_references(db_path, work_dir)?;
    }
    
    // Step 3: Semantic fingerprints with tree-sitter
    extract_fingerprints(db_path, work_dir, force)?;
    
    // Step 4: Show summary
    show_summary(db_path)?;
    
    Ok(())
}

/// Extract Git survival metrics
fn extract_git_metrics(db_path: &str, work_dir: &Path) -> Result<()> {
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
                let first = commits.last().unwrap_or(&"").split_whitespace().next().unwrap_or("");
                let last = commits.first().unwrap_or(&"").split_whitespace().next().unwrap_or("");
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
        
    println!("  ✓ Extracted references from {} patterns", files.lines().count());
    Ok(())
}

/// Extract semantic fingerprints with tree-sitter
fn extract_fingerprints(db_path: &str, work_dir: &Path, force: bool) -> Result<()> {
    println!("🧠 Generating semantic fingerprints...");
    
    use patina::semantic::languages::{Language, create_parser};
    use std::time::SystemTime;
    use std::collections::HashMap;
    
    // Find all supported language files
    let mut all_files = Vec::new();
    
    // Find Rust files
    let rust_files = Command::new("find")
        .current_dir(work_dir)
        .args([".", "-name", "*.rs", "-type", "f"])
        .output()?;
    if rust_files.status.success() {
        for file in String::from_utf8_lossy(&rust_files.stdout).lines() {
            if !file.is_empty() {
                all_files.push((file.to_string(), Language::Rust));
            }
        }
    }
    
    // Find Go files
    let go_files = Command::new("find")
        .current_dir(work_dir)
        .args([".", "-name", "*.go", "-type", "f"])
        .output()?;
    if go_files.status.success() {
        for file in String::from_utf8_lossy(&go_files.stdout).lines() {
            if !file.is_empty() {
                all_files.push((file.to_string(), Language::Go));
            }
        }
    }
    
    // Find Solidity files
    let sol_files = Command::new("find")
        .current_dir(work_dir)
        .args([".", "-name", "*.sol", "-type", "f"])
        .output()?;
    if sol_files.status.success() {
        for file in String::from_utf8_lossy(&sol_files.stdout).lines() {
            if !file.is_empty() {
                all_files.push((file.to_string(), Language::Solidity));
            }
        }
    }
    
    if all_files.is_empty() {
        println!("  ⚠️  No supported language files found");
        return Ok(());
    }
    
    println!("  📂 Found {} files ({} Rust, {} Go, {} Solidity)", 
        all_files.len(),
        all_files.iter().filter(|(_, l)| *l == Language::Rust).count(),
        all_files.iter().filter(|(_, l)| *l == Language::Go).count(),
        all_files.iter().filter(|(_, l)| *l == Language::Solidity).count()
    );
    
    // Build map of current files with mtimes
    let mut current_files = HashMap::new();
    for (file_str, _) in &all_files {
        let file_path = work_dir.join(file_str);
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                let mtime = modified.duration_since(SystemTime::UNIX_EPOCH)
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
        for path in changes.new_files.iter().chain(changes.modified_files.iter()) {
            let path_str = path.to_string_lossy().to_string();
            if let Some((_, lang)) = all_files.iter().find(|(f, _)| f == &path_str) {
                files_to_process.push((path_str, *lang));
            }
        }
        files_to_process
    };
    
    let mut sql = String::from("BEGIN TRANSACTION;\n");
    let mut symbol_count = 0;
    let mut current_lang = Language::Unknown;
    let mut parser: Option<tree_sitter::Parser> = None;
    let mut batch_count = 0;
    
    // Process only new and modified files
    for (file, language) in files_to_process {
        // Switch parser if language changed
        if language != current_lang {
            parser = Some(create_parser(language)?);
            current_lang = language;
        }
        
        // Check if file needs reindexing (mtime-based incremental)
        let file_path = work_dir.join(&file);
        let metadata = std::fs::metadata(&file_path)?;
        let mtime = metadata.modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64;
            
        // TODO: Check index_state to skip unchanged files
        
        // Parse and fingerprint
        let content = std::fs::read_to_string(&file_path)?;
        if let Some(ref mut p) = parser {
            if let Some(tree) = p.parse(&content, None) {
                let mut cursor = tree.walk();
                symbol_count += process_ast_node(&mut cursor, content.as_bytes(), &file, &mut sql, language);
                
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
                stdin.write_all(sql.as_bytes()).context("Failed to write SQL")?;
            }
            
            let output = child.wait_with_output().context("Failed to execute batch")?;
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
            stdin.write_all(sql.as_bytes()).context("Failed to write SQL")?;
        }
        
        let output = child.wait_with_output().context("Failed to insert final batch")?;
        
        if !output.status.success() {
            eprintln!("DuckDB error: {}", String::from_utf8_lossy(&output.stderr));
        }
    }
    
    println!("  ✓ Fingerprinted {} symbols", symbol_count);
    Ok(())
}

/// Process AST nodes and generate fingerprints
fn process_ast_node(
    cursor: &mut tree_sitter::TreeCursor,
    source: &[u8],
    file_path: &str,
    sql: &mut String,
    language: patina::semantic::languages::Language,
) -> usize {
    use patina::semantic::fingerprint::Fingerprint;
    use patina::semantic::languages::Language;
    
    let node = cursor.node();
    let mut count = 0;
    
    // Check if this is a symbol we want to fingerprint
    let kind = match (language, node.kind()) {
        // Rust mappings
        (Language::Rust, "function_item") => "function",
        (Language::Rust, "struct_item") => "struct",
        (Language::Rust, "trait_item") => "trait",
        (Language::Rust, "impl_item") => "impl",
        // Go mappings
        (Language::Go, "function_declaration") => "function",
        (Language::Go, "method_declaration") => "function",
        (Language::Go, "type_spec") => {
            // Check if it's a struct or interface
            if node.child_by_field_name("type").map_or(false, |n| n.kind() == "struct_type") {
                "struct"
            } else if node.child_by_field_name("type").map_or(false, |n| n.kind() == "interface_type") {
                "trait"
            } else {
                ""
            }
        },
        // Solidity mappings
        (Language::Solidity, "function_definition") => "function",
        (Language::Solidity, "contract_declaration") => "struct",
        (Language::Solidity, "interface_declaration") => "trait",
        (Language::Solidity, "library_declaration") => "impl",
        (Language::Solidity, "modifier_definition") => "function",
        (Language::Solidity, "event_definition") => "function",
        _ => {
            // Recurse into children
            if cursor.goto_first_child() {
                loop {
                    count += process_ast_node(cursor, source, file_path, sql, language);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
            return count;
        }
    };
    
    // Skip empty kinds
    if kind.is_empty() {
        return count;
    }
    
    // Extract name and generate fingerprint
    let name_node = match language {
        Language::Go if node.kind() == "type_spec" => {
            // Go type specs have name directly in the node
            node.child_by_field_name("name")
        },
        Language::Solidity => {
            // Solidity has name or identifier field
            node.child_by_field_name("name")
                .or_else(|| node.child_by_field_name("identifier"))
        },
        _ => node.child_by_field_name("name")
    };
    
    if let Some(name_node) = name_node {
        let name = name_node.utf8_text(source).unwrap_or("<unknown>");
        let fingerprint = Fingerprint::from_ast(node, source);
        
        // Get signature for search
        let signature = node.utf8_text(source)
            .unwrap_or("")
            .lines()
            .next()
            .unwrap_or("")
            .replace('\'', "''");
            
        // Insert fingerprint
        sql.push_str(&format!(
            "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', '{}', {}, {}, {}, {});\n",
            file_path, name, kind,
            fingerprint.pattern, fingerprint.imports,
            fingerprint.complexity, fingerprint.flags
        ));
        
        // Insert search data
        sql.push_str(&format!(
            "INSERT OR REPLACE INTO code_search (path, name, signature) VALUES ('{}', '{}', '{}');\n",
            file_path, name, signature
        ));
        
        count += 1;
    }
    
    count
}

/// Show extraction summary
fn show_summary(db_path: &str) -> Result<()> {
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
WHERE commit_count >= 10;
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

/// Run a custom query
fn run_query(query: &str, db_path: &str) -> Result<()> {
    let output = Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(query)
        .output()
        .context("Failed to execute query")?;
        
    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        anyhow::bail!("Query failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(())
}