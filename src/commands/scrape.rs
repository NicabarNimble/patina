use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Execute the scrape command to extract reality from code
pub fn execute(init: bool, reconcile: bool, query: Option<String>) -> Result<()> {
    if init {
        initialize_database()?;
    } else if reconcile {
        reconcile_patterns()?;
    } else if let Some(q) = query {
        run_query(&q)?;
    } else {
        extract_reality()?;
    }

    Ok(())
}

/// Initialize DuckDB database with semantic reality schema
fn initialize_database() -> Result<()> {
    println!("🗄️  Initializing semantic reality database...");

    // Create .patina directory if it doesn't exist
    std::fs::create_dir_all(".patina")?;

    // Initialize DuckDB with schema
    let schema = r#"
-- Code symbols extracted from AST
CREATE TABLE IF NOT EXISTS code_symbols (
    file TEXT NOT NULL,
    symbol TEXT NOT NULL,
    type TEXT NOT NULL,  -- function, struct, trait, impl
    line_count INTEGER,
    ast_hash TEXT,
    git_commit TEXT,
    first_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    survival_days INTEGER DEFAULT 0,
    PRIMARY KEY (file, symbol)
);

-- Pattern implementations found in code
CREATE TABLE IF NOT EXISTS pattern_implementations (
    pattern_id TEXT NOT NULL,
    file TEXT NOT NULL,
    symbol TEXT NOT NULL,
    compliance REAL CHECK (compliance >= 0 AND compliance <= 1),
    evidence TEXT,  -- Why we think this implements the pattern
    verified_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (pattern_id, file, symbol)
);

-- References between patterns
CREATE TABLE IF NOT EXISTS pattern_references (
    from_pattern TEXT NOT NULL,
    to_pattern TEXT NOT NULL,
    reference_type TEXT NOT NULL,  -- extends, implements, contradicts, mentions
    context TEXT,
    discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (from_pattern, to_pattern, reference_type)
);

-- Git survival metrics
CREATE TABLE IF NOT EXISTS git_metrics (
    file TEXT PRIMARY KEY,
    first_commit TEXT,
    last_commit TEXT,
    commit_count INTEGER,
    survival_days INTEGER,
    churn_rate REAL,  -- Lines changed / total lines
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Pattern documentation vs reality
CREATE TABLE IF NOT EXISTS pattern_claims (
    pattern_id TEXT NOT NULL,
    claim_type TEXT NOT NULL,  -- line_limit, structure, dependency
    claimed_value TEXT,
    actual_value TEXT,
    violations INTEGER DEFAULT 0,
    last_checked TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (pattern_id, claim_type)
);
"#;

    // Execute schema creation
    let output = Command::new("duckdb")
        .arg(".patina/semantic_reality.db")
        .arg("-c")
        .arg(schema)
        .output()
        .context("Failed to initialize DuckDB. Is duckdb installed?")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create database schema: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("✅ Database initialized at .patina/semantic_reality.db");
    println!("\nNext steps:");
    println!("  1. Run 'patina scrape' to extract reality from code");
    println!("  2. Run 'patina scrape --reconcile' to compare docs vs reality");
    println!("  3. Run 'patina scrape --query \"SELECT ...\"' to explore data");

    Ok(())
}

/// Extract facts from Git history and code structure
fn extract_reality() -> Result<()> {
    println!("🔍 Extracting reality from code...\n");

    // Step 1: Extract Git survival metrics
    extract_git_metrics()?;

    // Step 2: Extract pattern references from markdown
    extract_pattern_references()?;

    // Step 3: Analyze code structure (basic for now, tree-sitter later)
    analyze_code_structure()?;

    // Step 4: Show summary
    show_extraction_summary()?;

    Ok(())
}

/// Extract Git survival metrics for quality assessment
fn extract_git_metrics() -> Result<()> {
    println!("📊 Analyzing Git history...");

    // Get all Rust files with their history
    let rust_files = Command::new("git")
        .args(["ls-files", "*.rs", "src/**/*.rs"])
        .output()
        .context("Failed to list Git files")?;

    if !rust_files.status.success() {
        anyhow::bail!("Failed to get file list from Git");
    }

    let files = String::from_utf8_lossy(&rust_files.stdout);
    let file_count = files.lines().count();

    // For each file, get survival metrics
    let mut metrics_sql = String::from("BEGIN TRANSACTION;\n");
    metrics_sql.push_str("DELETE FROM git_metrics;\n");

    for file in files.lines() {
        if file.is_empty() {
            continue;
        }

        // Get first and last commit for this file
        let log_output = Command::new("git")
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

    // Execute the SQL
    Command::new("duckdb")
        .arg(".patina/semantic_reality.db")
        .arg("-c")
        .arg(&metrics_sql)
        .output()
        .context("Failed to insert Git metrics")?;

    println!("  ✓ Analyzed {} files", file_count);

    Ok(())
}

/// Extract references between patterns from markdown files
fn extract_pattern_references() -> Result<()> {
    println!("🔗 Extracting pattern references...");

    // Find all pattern markdown files
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

        // Extract pattern ID from filename
        let pattern_id = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Read file and look for references
        if let Ok(content) = std::fs::read_to_string(file) {
            // Look for explicit references in YAML frontmatter
            if let Some(refs_line) = content.lines().find(|l| l.starts_with("references:")) {
                if let Some(refs) = refs_line.strip_prefix("references:") {
                    let refs = refs.trim().trim_start_matches('[').trim_end_matches(']');
                    for reference in refs.split(',') {
                        let reference = reference.trim().trim_matches('"').trim_matches('\'');
                        if !reference.is_empty() {
                            references_sql.push_str(&format!(
                                "INSERT INTO pattern_references (from_pattern, to_pattern, reference_type, context) VALUES ('{}', '{}', 'references', 'explicit reference in frontmatter');\n",
                                pattern_id, reference
                            ));
                        }
                    }
                }
            }

            // Look for mentions in content
            for mentioned in &[
                "dependable-rust",
                "eternal-tool",
                "pattern-selection",
                "modular-architecture",
            ] {
                if content.contains(mentioned) && mentioned != &pattern_id {
                    references_sql.push_str(&format!(
                        "INSERT OR IGNORE INTO pattern_references (from_pattern, to_pattern, reference_type, context) VALUES ('{}', '{}', 'mentions', 'mentioned in content');\n",
                        pattern_id, mentioned
                    ));
                }
            }
        }
    }

    references_sql.push_str("COMMIT;\n");

    // Execute the SQL
    Command::new("duckdb")
        .arg(".patina/semantic_reality.db")
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

/// Basic code structure analysis (will be enhanced with tree-sitter)
fn analyze_code_structure() -> Result<()> {
    println!("🏗️  Analyzing code structure...");

    // For now, do basic line counting and function detection
    let rust_files = Command::new("find")
        .args(["src", "-name", "*.rs", "-type", "f"])
        .output()
        .context("Failed to find Rust files")?;

    if !rust_files.status.success() {
        anyhow::bail!("Failed to list Rust files");
    }

    let files = String::from_utf8_lossy(&rust_files.stdout);
    let mut symbols_sql = String::from("BEGIN TRANSACTION;\n");
    symbols_sql.push_str("DELETE FROM code_symbols;\n");

    for file in files.lines() {
        if file.is_empty() {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(file) {
            let line_count = content.lines().count();

            // Basic function detection (will be replaced with tree-sitter)
            for line in content.lines() {
                if let Some(fn_match) = extract_function_name(line) {
                    symbols_sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_symbols (file, symbol, type, line_count) VALUES ('{}', '{}', 'function', {});\n",
                        file, fn_match, line_count
                    ));
                } else if let Some(struct_match) = extract_struct_name(line) {
                    symbols_sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_symbols (file, symbol, type, line_count) VALUES ('{}', '{}', 'struct', {});\n",
                        file, struct_match, line_count
                    ));
                }
            }
        }
    }

    symbols_sql.push_str("COMMIT;\n");

    // Execute the SQL
    Command::new("duckdb")
        .arg(".patina/semantic_reality.db")
        .arg("-c")
        .arg(&symbols_sql)
        .output()
        .context("Failed to insert code symbols")?;

    // Update survival days in symbols from git_metrics
    let update_sql = r#"
UPDATE code_symbols 
SET survival_days = (
    SELECT survival_days 
    FROM git_metrics 
    WHERE git_metrics.file = code_symbols.file
)
WHERE EXISTS (
    SELECT 1 FROM git_metrics 
    WHERE git_metrics.file = code_symbols.file
);
"#;

    Command::new("duckdb")
        .arg(".patina/semantic_reality.db")
        .arg("-c")
        .arg(update_sql)
        .output()
        .context("Failed to update survival days")?;

    println!("  ✓ Analyzed {} Rust files", files.lines().count());

    Ok(())
}

/// Extract function name from a line of Rust code
fn extract_function_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("pub fn ") {
        trimmed
            .strip_prefix("pub fn ")
            .and_then(|s| s.split('(').next())
            .map(|s| s.trim().to_string())
    } else if trimmed.starts_with("fn ") {
        trimmed
            .strip_prefix("fn ")
            .and_then(|s| s.split('(').next())
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Extract struct name from a line of Rust code
fn extract_struct_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("pub struct ") {
        trimmed
            .strip_prefix("pub struct ")
            .and_then(|s| {
                s.split(|c: char| c.is_whitespace() || c == '{' || c == '<')
                    .next()
            })
            .map(|s| s.trim().to_string())
    } else if trimmed.starts_with("struct ") {
        trimmed
            .strip_prefix("struct ")
            .and_then(|s| {
                s.split(|c: char| c.is_whitespace() || c == '{' || c == '<')
                    .next()
            })
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Show summary of extracted data
fn show_extraction_summary() -> Result<()> {
    println!("\n📈 Extraction Summary:");

    // Query summary statistics - focus on actual metrics not time
    let summary_query = r#"
SELECT 
    'Total files analyzed' as metric,
    COUNT(DISTINCT file) as value
FROM git_metrics
UNION ALL
SELECT 
    'Files with 10+ commits' as metric,
    COUNT(*) as value
FROM git_metrics
WHERE commit_count >= 10
UNION ALL
SELECT 
    'Total functions found' as metric,
    COUNT(*) as value
FROM code_symbols
WHERE type = 'function'
UNION ALL
SELECT 
    'Total structs found' as metric,
    COUNT(*) as value
FROM code_symbols
WHERE type = 'struct'
UNION ALL
SELECT 
    'Pattern references' as metric,
    COUNT(*) as value
FROM pattern_references
UNION ALL
SELECT 
    'Avg commits per file' as metric,
    CAST(AVG(commit_count) AS INTEGER) as value
FROM git_metrics;
"#;

    let output = Command::new("duckdb")
        .arg(".patina/semantic_reality.db")
        .arg("-c")
        .arg(summary_query)
        .output()
        .context("Failed to query summary")?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    Ok(())
}

/// Reconcile documented patterns with code reality
fn reconcile_patterns() -> Result<()> {
    println!("🔍 Reconciling documentation with reality...\n");

    // Check line count claims vs reality - focus on commit frequency not age
    let line_check_query = r#"
SELECT 
    cs.file,
    cs.symbol,
    cs.line_count,
    gm.commit_count
FROM code_symbols cs
JOIN git_metrics gm ON cs.file = gm.file
WHERE cs.line_count > 150
  AND cs.type = 'function'
  AND gm.commit_count > 5  -- Files that have been modified multiple times
ORDER BY cs.line_count DESC
LIMIT 10;
"#;

    let output = Command::new("duckdb")
        .arg(".patina/semantic_reality.db")
        .arg("-c")
        .arg(line_check_query)
        .output()
        .context("Failed to query line violations")?;

    if output.status.success() {
        let results = String::from_utf8_lossy(&output.stdout);
        if results.trim().lines().count() > 1 {
            // Has header + data
            println!("⚠️  CONFLICT: dependable-rust.md claims '≤150 lines'");
            println!("📊 REALITY: These functions exceed 150 lines but are frequently modified:\n");
            println!("{}", results);

            println!("\n❓ How should we reconcile this?");
            println!("  1. Update pattern to match reality (~300 lines OK for commands)");
            println!("  2. Mark pattern as 'aspirational' not 'enforced'");
            println!("  3. Mark violating files for refactoring");
            println!("  4. Deprecate this pattern constraint");
            println!("\n(In a real implementation, this would be interactive)");
        } else {
            println!("✅ No conflicts found between documentation and reality!");
        }
    }

    Ok(())
}

/// Run a custom query against the semantic reality database
fn run_query(query: &str) -> Result<()> {
    let output = Command::new("duckdb")
        .arg(".patina/semantic_reality.db")
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
