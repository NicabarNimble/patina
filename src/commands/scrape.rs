use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Execute the scrape command to build semantic knowledge database
pub fn execute(init: bool, query: Option<String>) -> Result<()> {
    if init {
        initialize_database()?;
    } else if let Some(q) = query {
        run_query(&q)?;
    } else {
        extract_and_index()?;
    }
    Ok(())
}

/// Initialize DuckDB database with lean schema and optimal settings for small size
fn initialize_database() -> Result<()> {
    println!("🗄️  Initializing optimized knowledge database...");
    
    std::fs::create_dir_all(".patina")?;
    
    // Remove old database if exists
    let db_path = ".patina/knowledge.db";
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
fn extract_and_index() -> Result<()> {
    println!("🔍 Indexing codebase...\n");
    
    // Step 1: Git metrics for quality signals
    extract_git_metrics()?;
    
    // Step 2: Pattern references from docs
    extract_pattern_references()?;
    
    // Step 3: Semantic fingerprints with tree-sitter
    extract_fingerprints()?;
    
    // Step 4: Show summary
    show_summary()?;
    
    Ok(())
}

/// Extract Git survival metrics
fn extract_git_metrics() -> Result<()> {
    println!("📊 Analyzing Git history...");
    
    let rust_files = Command::new("git")
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
        .arg(".patina/knowledge.db")
        .arg("-c")
        .arg(&metrics_sql)
        .output()
        .context("Failed to insert Git metrics")?;
        
    println!("  ✓ Analyzed {} files", file_count);
    Ok(())
}

/// Extract pattern references from markdown
fn extract_pattern_references() -> Result<()> {
    println!("🔗 Extracting pattern references...");
    
    let pattern_files = Command::new("find")
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
            
        if let Ok(content) = std::fs::read_to_string(file) {
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
        .arg(".patina/knowledge.db")
        .arg("-c")
        .arg(&references_sql)
        .output()
        .context("Failed to insert pattern references")?;
        
    println!("  ✓ Extracted references from {} patterns", files.lines().count());
    Ok(())
}

/// Extract semantic fingerprints with tree-sitter
fn extract_fingerprints() -> Result<()> {
    println!("🧠 Generating semantic fingerprints...");
    
    use patina::semantic::init_parser;
    use std::time::SystemTime;
    
    let rust_files = Command::new("find")
        .args(["src", "-name", "*.rs", "-type", "f"])
        .output()
        .context("Failed to find Rust files")?;
        
    if !rust_files.status.success() {
        anyhow::bail!("Failed to list Rust files");
    }
    
    let files = String::from_utf8_lossy(&rust_files.stdout);
    let mut parser = init_parser()?;
    
    // Start transaction
    let mut sql = String::from("BEGIN TRANSACTION;\n");
    sql.push_str("DELETE FROM code_fingerprints;\n");
    sql.push_str("DELETE FROM code_search;\n");
    sql.push_str("DELETE FROM index_state;\n");
    
    let mut symbol_count = 0;
    for file in files.lines() {
        if file.is_empty() {
            continue;
        }
        
        // Check if file needs reindexing (mtime-based incremental)
        let metadata = std::fs::metadata(file)?;
        let mtime = metadata.modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64;
            
        // TODO: Check index_state to skip unchanged files
        
        // Parse and fingerprint
        let content = std::fs::read_to_string(file)?;
        if let Some(tree) = parser.parse(&content, None) {
            let mut cursor = tree.walk();
            symbol_count += process_ast_node(&mut cursor, content.as_bytes(), file, &mut sql);
            
            // Record index state
            sql.push_str(&format!(
                "INSERT INTO index_state (path, mtime) VALUES ('{}', {});\n",
                file, mtime
            ));
        }
    }
    
    sql.push_str("COMMIT;\n");
    
    // Execute
    let output = Command::new("duckdb")
        .arg(".patina/knowledge.db")
        .arg("-c")
        .arg(&sql)
        .output()
        .context("Failed to insert fingerprints")?;
        
    if !output.status.success() {
        eprintln!("DuckDB error: {}", String::from_utf8_lossy(&output.stderr));
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
) -> usize {
    use patina::semantic::fingerprint::Fingerprint;
    
    let node = cursor.node();
    let mut count = 0;
    
    // Check if this is a symbol we want to fingerprint
    let kind = match node.kind() {
        "function_item" => "function",
        "struct_item" => "struct",
        "trait_item" => "trait",
        "impl_item" => "impl",
        _ => {
            // Recurse into children
            if cursor.goto_first_child() {
                loop {
                    count += process_ast_node(cursor, source, file_path, sql);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
            return count;
        }
    };
    
    // Extract name and generate fingerprint
    if let Some(name_node) = node.child_by_field_name("name") {
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
fn show_summary() -> Result<()> {
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
        .arg(".patina/knowledge.db")
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
        .arg(".patina/knowledge.db")
        .arg("-c")
        .arg(size_query)
        .output()?;
        
    if size_output.status.success() {
        println!("\n💾 Database info:");
        println!("{}", String::from_utf8_lossy(&size_output.stdout));
    }
    
    // Also show file size
    if let Ok(metadata) = std::fs::metadata(".patina/knowledge.db") {
        let size_kb = metadata.len() / 1024;
        println!("📁 File size: {}KB", size_kb);
    }
    
    Ok(())
}

/// Run a custom query
fn run_query(query: &str) -> Result<()> {
    let output = Command::new("duckdb")
        .arg(".patina/knowledge.db")
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