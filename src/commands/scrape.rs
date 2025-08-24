use crate::commands::incremental;
use anyhow::{Context, Result};
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
    let init_script = format!(
        r#"
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
"#,
        patina::semantic::fingerprint::generate_schema(),
        db_path = db_path
    );

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

    // If force flag is set, reinitialize database to ensure clean state
    if force {
        initialize_database(db_path)?;
    }

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

/// Extract semantic fingerprints with tree-sitter
fn extract_fingerprints(db_path: &str, work_dir: &Path, force: bool) -> Result<()> {
    println!("🧠 Generating semantic fingerprints and extracting truth data...");

    use patina::semantic::languages::{create_parser, Language};
    use std::collections::HashMap;
    use std::time::SystemTime;
    use ignore::WalkBuilder;

    // Find all supported language files
    let mut all_files = Vec::new();
    
    // Track skipped files by extension
    let mut skipped_files: HashMap<String, (usize, usize, String)> = HashMap::new(); // ext -> (count, bytes, example_path)

    // Use ignore crate to walk files, respecting .gitignore
    let walker = WalkBuilder::new(work_dir)
        .hidden(false)  // Don't process hidden files
        .git_ignore(true)  // Respect .gitignore
        .git_global(true)  // Respect global gitignore
        .git_exclude(true)  // Respect .git/info/exclude
        .ignore(true)  // Respect .ignore files
        .build();
    
    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        
        // Skip directories
        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
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
            Language::Rust | Language::Go | Language::Solidity | 
            Language::Python | Language::JavaScript | Language::JavaScriptJSX |
            Language::TypeScript | Language::TypeScriptTSX => {
                // Supported language - add to processing list with relative path
                all_files.push((format!("./{}", relative_path_str), language));
            }
            Language::Unknown => {
                // Track skipped file
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    // Get file size
                    let file_size = entry.metadata()
                        .ok()
                        .map(|m| m.len() as usize)
                        .unwrap_or(0);
                    
                    let entry = skipped_files.entry(ext.to_string()).or_insert((0, 0, relative_path_str.to_string()));
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

    println!(
        "  📂 Found {} files ({} Rust, {} Go, {} Solidity, {} Python, {} JS, {} JSX, {} TS, {} TSX)",
        all_files.len(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::Rust)
            .count(),
        all_files.iter().filter(|(_, l)| *l == Language::Go).count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::Solidity)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::Python)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::JavaScript)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::JavaScriptJSX)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::TypeScript)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::TypeScriptTSX)
            .count()
    );

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
        let mtime = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64;

        // TODO: Check index_state to skip unchanged files

        // Parse and fingerprint
        let content = std::fs::read_to_string(&file_path)?;
        if let Some(ref mut p) = parser {
            if let Some(tree) = p.parse(&content, None) {
                let mut cursor = tree.walk();
                symbol_count +=
                    process_ast_node(&mut cursor, content.as_bytes(), &file, &mut sql, language);

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
            .context("Failed to insert final batch")?;

        if !output.status.success() {
            eprintln!("DuckDB error: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    println!("  ✓ Fingerprinted {} symbols", symbol_count);
    
    // Save and report skipped files
    if !skipped_files.is_empty() {
        save_skipped_files(db_path, &skipped_files)?;
        report_skipped_files(&skipped_files);
    }
    
    Ok(())
}

/// Save skipped files to database
fn save_skipped_files(db_path: &str, skipped: &HashMap<String, (usize, usize, String)>) -> Result<()> {
    use std::process::Command;
    
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
        stdin.write_all(sql.as_bytes()).context("Failed to write SQL")?;
    }
    
    let output = child.wait_with_output().context("Failed to save skipped files")?;
    if !output.status.success() {
        eprintln!("Warning: Failed to save skipped files stats");
    }
    
    Ok(())
}

/// Report skipped files to user
fn report_skipped_files(skipped: &HashMap<String, (usize, usize, String)>) {
    // Sort by file count descending
    let mut sorted: Vec<_> = skipped.iter().collect();
    sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    
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
    let suggestions: Vec<&str> = sorted.iter()
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
        println!("\n💡 Consider adding parsers for: {}", suggestions.join(", "));
    }
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
        (Language::Rust, "type_alias") => "type_alias",
        (Language::Rust, "const_item") => "const",
        (Language::Rust, "use_declaration") => "import",
        // Go mappings
        (Language::Go, "function_declaration") => "function",
        (Language::Go, "method_declaration") => "function",
        (Language::Go, "const_declaration") => "const",
        (Language::Go, "import_declaration") => "import",
        (Language::Go, "type_spec") => {
            // Check if it's a struct or interface
            if node
                .child_by_field_name("type")
                .is_some_and(|n| n.kind() == "struct_type")
            {
                "struct"
            } else if node
                .child_by_field_name("type")
                .is_some_and(|n| n.kind() == "interface_type")
            {
                "trait"
            } else {
                "type_alias"  // Type aliases in Go
            }
        }
        // Solidity mappings
        (Language::Solidity, "function_definition") => "function",
        (Language::Solidity, "contract_declaration") => "struct",
        (Language::Solidity, "interface_declaration") => "trait",
        (Language::Solidity, "library_declaration") => "impl",
        (Language::Solidity, "modifier_definition") => "function",
        (Language::Solidity, "event_definition") => "function",
        // Python mappings
        (Language::Python, "function_definition") => "function",
        (Language::Python, "class_definition") => "struct",
        (Language::Python, "decorated_definition") => {
            // Check if it's a decorated function or class
            if node.child_by_field_name("definition")
                .is_some_and(|n| n.kind() == "function_definition") {
                "function"
            } else if node.child_by_field_name("definition")
                .is_some_and(|n| n.kind() == "class_definition") {
                "struct"
            } else {
                ""
            }
        }
        (Language::Python, "import_statement") => "import",
        (Language::Python, "import_from_statement") => "import",
        // JavaScript/JSX mappings
        (Language::JavaScript | Language::JavaScriptJSX, "function_declaration") => "function",
        (Language::JavaScript | Language::JavaScriptJSX, "function_expression") => "function",
        (Language::JavaScript | Language::JavaScriptJSX, "arrow_function") => "function",
        (Language::JavaScript | Language::JavaScriptJSX, "method_definition") => "function",
        (Language::JavaScript | Language::JavaScriptJSX, "class_declaration") => "struct",
        (Language::JavaScript | Language::JavaScriptJSX, "import_statement") => "import",
        (Language::JavaScript | Language::JavaScriptJSX, "variable_declarator") => {
            // Check if it's a const/let/var with a function value
            if node.child_by_field_name("value")
                .is_some_and(|n| n.kind() == "arrow_function" || n.kind() == "function_expression") {
                "function"
            } else if node.child_by_field_name("value")
                .is_some_and(|n| n.kind() == "class_expression") {
                "struct"
            } else {
                ""
            }
        }
        // TypeScript/TSX mappings
        (Language::TypeScript | Language::TypeScriptTSX, "function_declaration") => "function",
        (Language::TypeScript | Language::TypeScriptTSX, "function_expression") => "function", 
        (Language::TypeScript | Language::TypeScriptTSX, "arrow_function") => "function",
        (Language::TypeScript | Language::TypeScriptTSX, "method_definition") => "function",
        (Language::TypeScript | Language::TypeScriptTSX, "class_declaration") => "struct",
        (Language::TypeScript | Language::TypeScriptTSX, "interface_declaration") => "trait",
        (Language::TypeScript | Language::TypeScriptTSX, "type_alias_declaration") => "type_alias",
        (Language::TypeScript | Language::TypeScriptTSX, "enum_declaration") => "struct",
        (Language::TypeScript | Language::TypeScriptTSX, "import_statement") => "import",
        (Language::TypeScript | Language::TypeScriptTSX, "variable_declarator") => {
            // Check if it's a const/let/var with a function value
            if node.child_by_field_name("value")
                .is_some_and(|n| n.kind() == "arrow_function" || n.kind() == "function_expression") {
                "function"
            } else if node.child_by_field_name("value")
                .is_some_and(|n| n.kind() == "class_expression") {
                "struct"
            } else {
                ""
            }
        }
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

    // Handle imports separately (they don't have a name field)
    if kind == "import" {
        extract_import_fact(node, source, file_path, sql, language);
        count += 1;
        return count;
    }

    // Extract name for other symbols
    let name_node = match language {
        Language::Go if node.kind() == "type_spec" => {
            // Go type specs have name directly in the node
            node.child_by_field_name("name")
        }
        Language::Solidity => {
            // Solidity has name or identifier field
            node.child_by_field_name("name")
                .or_else(|| node.child_by_field_name("identifier"))
        }
        Language::Python => {
            // Python uses name field for functions/classes
            if node.kind() == "decorated_definition" {
                // For decorated definitions, get the name from the inner definition
                node.child_by_field_name("definition")
                    .and_then(|def| def.child_by_field_name("name"))
            } else {
                node.child_by_field_name("name")
            }
        }
        Language::JavaScript | Language::JavaScriptJSX | Language::TypeScript | Language::TypeScriptTSX => {
            // JS/TS uses different field names depending on context
            if node.kind() == "variable_declarator" {
                // For const/let/var declarations, name is in the 'name' field
                node.child_by_field_name("name")
            } else if node.kind() == "method_definition" {
                // Methods have name in 'name' field
                node.child_by_field_name("name")
            } else {
                // Functions, classes use 'name' field
                node.child_by_field_name("name")
            }
        }
        _ => node.child_by_field_name("name"),
    };

    if let Some(name_node) = name_node {
        let name = name_node.utf8_text(source).unwrap_or("<unknown>");
        
        // Extract based on kind
        match kind {
            "function" => {
                // Extract function facts
                extract_function_facts(node, source, file_path, name, sql, language);
                
                // Also generate fingerprint for functions
                let fingerprint = Fingerprint::from_ast(node, source);
                let signature = node
                    .utf8_text(source)
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .replace('\'', "''");

                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', '{}', {}, {}, {}, {});\n",
                    file_path, name, kind,
                    fingerprint.pattern, fingerprint.imports,
                    fingerprint.complexity, fingerprint.flags
                ));

                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO code_search (path, name, signature) VALUES ('{}', '{}', '{}');\n",
                    file_path, name, signature
                ));
            }
            "type_alias" | "struct" | "trait" | "const" => {
                // Extract type vocabulary
                extract_type_definition(node, source, file_path, name, kind, sql, language);
                
                // Also generate fingerprint for structs/traits
                if kind == "struct" || kind == "trait" {
                    let fingerprint = Fingerprint::from_ast(node, source);
                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', '{}', {}, {}, {}, {});\n",
                        file_path, name, kind,
                        fingerprint.pattern, fingerprint.imports,
                        fingerprint.complexity, fingerprint.flags
                    ));
                }
            }
            "impl" => {
                // Keep fingerprinting for impl blocks
                let fingerprint = Fingerprint::from_ast(node, source);
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', '{}', {}, {}, {}, {});\n",
                    file_path, name, kind,
                    fingerprint.pattern, fingerprint.imports,
                    fingerprint.complexity, fingerprint.flags
                ));
            }
            _ => {}
        }

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

/// Extract function facts (truth data only)
fn extract_function_facts(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    sql: &mut String,
    language: patina::semantic::languages::Language,
) {
    use patina::semantic::languages::Language;
    
    // Extract visibility
    let is_public = match language {
        Language::Rust => {
            // Check for pub keyword
            node.children(&mut node.walk())
                .any(|child| child.kind() == "visibility_modifier")
        }
        Language::Go => {
            // In Go, uppercase first letter = public
            name.chars().next().map_or(false, |c| c.is_uppercase())
        }
        Language::Solidity => {
            // Solidity defaults to public
            !node.utf8_text(source).unwrap_or("").contains("private")
        }
        Language::Python => {
            // Python uses convention: _ prefix = private
            !name.starts_with('_')
        }
        Language::JavaScript | Language::JavaScriptJSX | Language::TypeScript | Language::TypeScriptTSX => {
            // JS/TS: export = public, TypeScript can have public/private keywords
            let text = node.utf8_text(source).unwrap_or("");
            text.contains("export") || text.contains("public")
        }
        Language::Unknown => false,
    };
    
    // Extract async
    let is_async = match language {
        Language::Rust => {
            node.children(&mut node.walk())
                .any(|child| child.kind() == "async")
        }
        Language::JavaScript | Language::JavaScriptJSX | Language::TypeScript | Language::TypeScriptTSX => {
            // JS/TS have async functions
            let text = node.utf8_text(source).unwrap_or("");
            text.starts_with("async ") || text.contains(" async ")
        }
        Language::Python => {
            // Python uses async def
            node.kind() == "async_function_definition" || 
            node.utf8_text(source).unwrap_or("").starts_with("async def")
        }
        _ => false, // Go and Solidity don't have async keyword
    };
    
    // Extract unsafe
    let is_unsafe = match language {
        Language::Rust => {
            node.children(&mut node.walk())
                .any(|child| child.kind() == "unsafe")
        }
        _ => false,
    };
    
    // Extract parameters with details
    let params_node = node.child_by_field_name("parameters");
    let (takes_mut_self, takes_mut_params, parameter_count, parameter_list) = if let Some(params) = params_node {
        let mut has_mut_self = false;
        let mut has_mut_params = false;
        let mut param_count = 0;
        let mut param_details = Vec::new();
        
        let params_text = params.utf8_text(source).unwrap_or("");
        
        match language {
            Language::Rust => {
                // Check for &mut self
                if params_text.contains("&mut self") {
                    has_mut_self = true;
                }
                // Check for other mut params
                if params_text.contains("mut ") && !params_text.contains("&mut self") {
                    has_mut_params = true;
                }
                // Extract each parameter
                for child in params.children(&mut params.walk()) {
                    if child.kind() == "parameter" {
                        // Get parameter name and type
                        if let Some(pattern) = child.child_by_field_name("pattern") {
                            let param_name = pattern.utf8_text(source).unwrap_or("").to_string();
                            let param_type = child.child_by_field_name("type")
                                .and_then(|t| t.utf8_text(source).ok())
                                .unwrap_or("")
                                .to_string();
                            param_details.push(format!("{}:{}", param_name, param_type));
                        }
                        param_count += 1;
                    } else if child.kind() == "self_parameter" {
                        param_details.push("self".to_string());
                        param_count += 1;
                    }
                }
            }
            Language::Go => {
                // Extract Go parameters
                for child in params.children(&mut params.walk()) {
                    if child.kind() == "parameter_declaration" {
                        let param_text = child.utf8_text(source).unwrap_or("").to_string();
                        param_details.push(param_text);
                        param_count += 1;
                    }
                }
            }
            Language::Solidity => {
                // Extract Solidity parameters
                for child in params.children(&mut params.walk()) {
                    if child.kind() == "parameter" {
                        let param_text = child.utf8_text(source).unwrap_or("").to_string();
                        param_details.push(param_text);
                        param_count += 1;
                    }
                }
            }
            Language::Python => {
                // Extract Python parameters - simpler approach
                for child in params.children(&mut params.walk()) {
                    // Skip punctuation
                    if child.kind() == "," || child.kind() == "(" || child.kind() == ")" {
                        continue;
                    }
                    
                    // Get any parameter-like text
                    if child.kind().contains("parameter") || child.kind() == "identifier" {
                        let param_text = child.utf8_text(source).unwrap_or("").trim().to_string();
                        if !param_text.is_empty() {
                            param_count += 1;
                            if param_text != "self" { // Skip 'self' in param list but count it
                                param_details.push(param_text);
                            }
                        }
                    }
                }
            }
            Language::JavaScript | Language::JavaScriptJSX | Language::TypeScript | Language::TypeScriptTSX => {
                // Extract JS/TS parameters - they can be formal_parameter, required_parameter, optional_parameter, or just identifier
                for child in params.children(&mut params.walk()) {
                    // Skip punctuation like commas and parentheses
                    if child.kind() == "," || child.kind() == "(" || child.kind() == ")" {
                        continue;
                    }
                    
                    // Get the parameter text for any parameter-like node
                    if child.kind().contains("parameter") || child.kind() == "identifier" {
                        let param_text = child.utf8_text(source).unwrap_or("").trim().to_string();
                        if !param_text.is_empty() {
                            param_details.push(param_text);
                            param_count += 1;
                        }
                    }
                }
            }
            Language::Unknown => {} // Skip for unknown languages
        }
        
        // Create parameter list string (escape for SQL)
        let param_list = if !param_details.is_empty() {
            param_details.join(", ").replace('\'', "''")
        } else {
            String::new()
        };
        
        (has_mut_self, has_mut_params, param_count, param_list)
    } else {
        (false, false, 0, String::new())
    };
    
    // Extract return type with full details
    let (returns_result, returns_option, return_type) = match language {
        Language::Rust => {
            if let Some(return_type_node) = node.child_by_field_name("return_type") {
                let ret_text = return_type_node.utf8_text(source).unwrap_or("");
                let ret_clean = ret_text.replace('\'', "''");
                (ret_text.contains("Result"), ret_text.contains("Option"), ret_clean)
            } else {
                (false, false, String::new())
            }
        }
        Language::Go => {
            if let Some(result) = node.child_by_field_name("result") {
                let ret_text = result.utf8_text(source).unwrap_or("");
                let ret_clean = ret_text.replace('\'', "''");
                (ret_text.contains("error"), false, ret_clean) // Go uses error, not Result/Option
            } else {
                (false, false, String::new())
            }
        }
        Language::TypeScript | Language::TypeScriptTSX => {
            if let Some(return_type_node) = node.child_by_field_name("return_type") {
                let ret_text = return_type_node.utf8_text(source).unwrap_or("");
                let ret_clean = ret_text.replace('\'', "''");
                (false, false, ret_clean) // TypeScript has explicit return types
            } else {
                (false, false, String::new())
            }
        }
        _ => (false, false, String::new()),
    };
    
    // Count generics
    let generic_count = match language {
        Language::Rust => {
            node.child_by_field_name("type_parameters")
                .map(|tp| {
                    tp.children(&mut tp.walk())
                        .filter(|c| c.kind() == "type_identifier" || c.kind() == "lifetime")
                        .count()
                })
                .unwrap_or(0)
        }
        _ => 0, // Go doesn't have generics (until recently), Solidity doesn't
    };
    
    // Insert function facts with parameter and return type details
    sql.push_str(&format!(
        "INSERT OR REPLACE INTO function_facts (file, name, takes_mut_self, takes_mut_params, returns_result, returns_option, is_async, is_unsafe, is_public, parameter_count, generic_count, parameters, return_type) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', '{}');\n",
        escape_sql(file_path),
        escape_sql(name),
        takes_mut_self,
        takes_mut_params,
        returns_result,
        returns_option,
        is_async,
        is_unsafe,
        is_public,
        parameter_count,
        generic_count,
        parameter_list,  // Already escaped with '' replacement
        return_type      // Already escaped with '' replacement
    ));
    
    // Extract behavioral hints
    if language == Language::Rust {
        extract_behavioral_hints(node, source, file_path, name, sql);
    }
}

/// Extract type definitions for vocabulary
fn extract_type_definition(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: &str,
    sql: &mut String,
    language: patina::semantic::languages::Language,
) {
    use patina::semantic::languages::Language;
    
    // Get the full definition (first line for brevity)
    let definition = node.utf8_text(source)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .replace('\'', "''");
    
    // Determine visibility
    let visibility = match language {
        Language::Rust => {
            if node.children(&mut node.walk()).any(|child| {
                if child.kind() == "visibility_modifier" {
                    let vis_text = child.utf8_text(source).unwrap_or("");
                    vis_text.contains("pub(crate)")
                } else {
                    false
                }
            }) {
                "pub(crate)"
            } else if node.children(&mut node.walk()).any(|child| child.kind() == "visibility_modifier") {
                "pub"
            } else {
                "private"
            }
        }
        Language::Go => {
            // In Go, uppercase = public
            if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                "pub"
            } else {
                "private"
            }
        }
        Language::Solidity => "pub", // Most things in Solidity are public by default
        Language::Python => {
            // Python convention: _ prefix = private
            if name.starts_with('_') {
                "private"
            } else {
                "pub"
            }
        }
        Language::JavaScript | Language::JavaScriptJSX | Language::TypeScript | Language::TypeScriptTSX => {
            // JS/TS: look for export keyword
            let text = node.utf8_text(source).unwrap_or("");
            if text.contains("export") {
                "pub"
            } else {
                "private"
            }
        }
        Language::Unknown => "private",
    };
    
    // Insert type vocabulary
    sql.push_str(&format!(
        "INSERT OR REPLACE INTO type_vocabulary (file, name, definition, kind, visibility) VALUES ('{}', '{}', '{}', '{}', '{}');\n",
        escape_sql(file_path),
        escape_sql(name),
        escape_sql(&definition),
        kind,
        visibility
    ));
}

/// Extract import facts
fn extract_import_fact(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    sql: &mut String,
    language: patina::semantic::languages::Language,
) {
    use patina::semantic::languages::Language;
    
    let import_text = node.utf8_text(source).unwrap_or("");
    
    match language {
        Language::Rust => {
            // Parse Rust use statements
            let import_clean = import_text
                .trim_start_matches("use ")
                .trim_end_matches(';');
            
            // Determine if external
            let is_external = !import_clean.starts_with("crate::")
                && !import_clean.starts_with("super::")
                && !import_clean.starts_with("self::");
            
            // Extract the imported item (last part after ::)
            let imported_item = import_clean
                .split("::")
                .last()
                .unwrap_or(import_clean);
            
            // Extract the source module
            let imported_from = if import_clean.contains("::") {
                import_clean
                    .rsplit_once("::")
                    .map(|(from, _)| from)
                    .unwrap_or(import_clean)
            } else {
                import_clean
            };
            
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'use');\n",
                escape_sql(file_path),
                escape_sql(imported_item),
                escape_sql(imported_from),
                is_external
            ));
        }
        Language::Go => {
            // Parse Go imports
            let import_clean = import_text
                .trim_start_matches("import ")
                .trim()
                .trim_matches('"');
            
            let is_external = !import_clean.starts_with(".");
            
            let imported_item = import_clean
                .split('/')
                .last()
                .unwrap_or(import_clean);
            
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                escape_sql(file_path),
                escape_sql(imported_item),
                escape_sql(import_clean),
                is_external
            ));
        }
        Language::Solidity => {
            // Parse Solidity imports
            if let Some(path_match) = import_text.split('"').nth(1) {
                let is_external = path_match.starts_with('@') || path_match.starts_with("http");
                
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                    escape_sql(file_path),
                    escape_sql(path_match),
                    escape_sql(path_match),
                    is_external
                ));
            }
        }
        Language::Python => {
            // Python imports: import x or from x import y
            // Simple extraction - just store the whole import for now
            let import_clean = import_text.trim();
            let is_external = !import_clean.contains("from .");
            
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                escape_sql(file_path),
                escape_sql(import_clean),
                escape_sql(import_clean),
                is_external
            ));
        }
        Language::JavaScript | Language::JavaScriptJSX | Language::TypeScript | Language::TypeScriptTSX => {
            // JS/TS imports: import x from 'y'
            // Simple extraction - just store the module path
            if let Some(module_match) = import_text.split('\'').nth(1).or_else(|| import_text.split('"').nth(1)) {
                let is_external = !module_match.starts_with('.');
                
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                    escape_sql(file_path),
                    escape_sql(module_match),
                    escape_sql(module_match),
                    is_external
                ));
            }
        }
        Language::Unknown => {}, // Skip unknown languages
    }
}

/// Extract behavioral hints (code smells as facts)
fn extract_behavioral_hints(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    function_name: &str,
    sql: &mut String,
) {
    // Only extract for function bodies
    if let Some(body) = node.child_by_field_name("body") {
        let body_text = body.utf8_text(source).unwrap_or("");
        
        // Count unwrap calls
        let calls_unwrap = body_text.matches(".unwrap()").count();
        
        // Count expect calls
        let calls_expect = body_text.matches(".expect(").count();
        
        // Check for panic! macro
        let has_panic_macro = body_text.contains("panic!");
        
        // Check for todo! macro
        let has_todo_macro = body_text.contains("todo!");
        
        // Check for unsafe blocks
        let has_unsafe_block = body_text.contains("unsafe {");
        
        // Check for Mutex usage
        let has_mutex = body_text.contains("Mutex");
        
        // Check for Arc usage
        let has_arc = body_text.contains("Arc<") || body_text.contains("Arc::");
        
        // Only insert if there are any behavioral hints
        if calls_unwrap > 0 || calls_expect > 0 || has_panic_macro || has_todo_macro || has_unsafe_block || has_mutex || has_arc {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO behavioral_hints (file, function, calls_unwrap, calls_expect, has_panic_macro, has_todo_macro, has_unsafe_block, has_mutex, has_arc) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {});\n",
                escape_sql(file_path),
                escape_sql(function_name),
                calls_unwrap,
                calls_expect,
                has_panic_macro,
                has_todo_macro,
                has_unsafe_block,
                has_mutex,
                has_arc
            ));
        }
    }
}

/// Escape SQL strings
fn escape_sql(s: &str) -> String {
    s.replace('\'', "''")
}
