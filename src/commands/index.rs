use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;

use patina::pipeline::{
    analyze_git, 
    discover_files, 
    parse_file, 
    generate_sql, 
    load_into_duckdb
};

/// Execute the index command with repo support like scrape
pub fn execute(
    init: bool, 
    query: Option<String>, 
    repo: Option<String>, 
    force: bool, 
    verbose: bool
) -> Result<()> {
    // Determine paths based on whether we're indexing a repo
    let (output_dir, work_dir, db_name) = if let Some(repo_name) = &repo {
        validate_repo_path(repo_name)?
    } else {
        (
            PathBuf::from(".patina"),
            std::env::current_dir()?,
            "knowledge.db".to_string(),
        )
    };

    if init {
        initialize_database(&output_dir, &db_name)?;
    } else if let Some(q) = query {
        run_query(&q, &output_dir, &db_name)?;
    } else {
        index_codebase(&output_dir, &work_dir, &db_name, force, verbose)?;
    }
    Ok(())
}

/// Validate repo exists and return output directory and working directory
fn validate_repo_path(repo_name: &str) -> Result<(PathBuf, PathBuf, String)> {
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

    // Use same database location as old scrape command for compatibility
    let output_dir = PathBuf::from("layer/dust/repos");
    let work_dir = std::env::current_dir()?.join(&repo_dir);
    let db_name = format!("{}.db", repo_name);

    println!("📦 Indexing repository: {}", repo_name);
    println!("   Database: {}/{}", output_dir.display(), db_name);

    Ok((output_dir, work_dir, db_name))
}

fn initialize_database(output_dir: &Path, db_name: &str) -> Result<()> {
    println!("🗄️  Initializing DuckDB database...\n");
    
    // Ensure output directory exists
    fs::create_dir_all(output_dir)?;
    
    let db_path = output_dir.join(db_name);
    
    // Use DuckDB to initialize
    patina::pipeline::duckdb::initialize_database(&db_path)?;
    
    println!("✓ Database initialized: {}", db_path.display());
    Ok(())
}

fn run_query(query: &str, output_dir: &Path, db_name: &str) -> Result<()> {
    let db_path = output_dir.join(db_name);
    
    if !db_path.exists() {
        anyhow::bail!(
            "Database not found: {}\nRun without --query first to build the index.",
            db_path.display()
        );
    }
    
    println!("🔍 Running query...\n");
    
    // Use DuckDB to execute query
    let (column_names, rows) = patina::pipeline::duckdb::execute_query(&db_path, query)?;
    
    // Print header
    println!("{}", column_names.join(" | "));
    println!("{}", "-".repeat(column_names.join(" | ").len()));
    
    // Print rows
    for row in rows {
        println!("{}", row.join(" | "));
    }
    
    Ok(())
}

fn index_codebase(
    output_dir: &Path, 
    work_dir: &Path, 
    db_name: &str,
    force: bool, 
    verbose: bool
) -> Result<()> {
    println!("🔍 Indexing codebase using new pipeline...\n");
    
    // Ensure output directory exists
    fs::create_dir_all(output_dir)?;
    
    // Create subdirectories for pipeline stages
    let cache_dir = if output_dir.ends_with("repos") {
        // For repo indexing, create cache in a subdirectory
        output_dir.join(format!("{}_cache", db_name.trim_end_matches(".db")))
    } else {
        // For local indexing, use ast_cache subdirectory
        output_dir.join("ast_cache")
    };
    fs::create_dir_all(&cache_dir)?;
    
    // If force flag is set, clear cache and reinitialize database
    if force {
        if verbose {
            println!("   Force flag set - clearing cache and database");
        }
        // Clear cache
        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir)?;
            fs::create_dir_all(&cache_dir)?;
        }
        // Reinitialize database
        initialize_database(output_dir, db_name)?;
    }
    
    // Phase 1: Discovery and Git analysis
    if verbose {
        println!("Phase 1: Discovering files and analyzing Git history...");
    }
    
    let files = discover_files(work_dir)?;
    let git_metrics = analyze_git(work_dir)?;
    
    println!("   Found {} files to index", files.len());
    
    // Save git metrics
    let git_metrics_path = cache_dir.join("git_metrics.json");
    fs::write(
        &git_metrics_path,
        serde_json::to_string_pretty(&git_metrics)?
    )?;
    
    // Phase 2: Parse files to intermediate format
    if verbose {
        println!("\nPhase 2: Parsing files to intermediate format...");
    }
    
    let mut parsed_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;
    
    for file in &files {
        let cache_path = get_cache_path(&cache_dir, work_dir, file);
        
        // Check if we need to parse this file
        if !force && should_skip_parse(file, &cache_path)? {
            if verbose {
                println!("   Skipping (cached): {}", file.display());
            }
            skipped_count += 1;
            continue;
        }
        
        if verbose {
            println!("   Parsing: {}", file.display());
        }
        
        // Parse based on language with error handling
        match parse_file(file) {
            Ok(ast_data) => {
                // Ensure cache directory exists
                if let Some(parent) = cache_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                // Write to cache
                fs::write(
                    &cache_path,
                    serde_json::to_string_pretty(&ast_data)?
                )?;
                
                parsed_count += 1;
            },
            Err(e) => {
                if verbose {
                    println!("   Error parsing {}: {}", file.display(), e);
                }
                error_count += 1;
            }
        }
    }
    
    println!("\n   Parsed: {} files", parsed_count);
    println!("   Cached: {} files", skipped_count);
    if error_count > 0 {
        println!("   Errors: {} files", error_count);
    }
    
    // Phase 3: Generate SQL from intermediate format
    if verbose {
        println!("\nPhase 3: Generating SQL from parsed data...");
    }
    
    let sql_path = cache_dir.join("load.sql");
    generate_sql(&cache_dir, &sql_path)?;
    
    println!("   Generated SQL: {}", sql_path.display());
    
    // Phase 4: Load into DuckDB
    if verbose {
        println!("\nPhase 4: Loading data into DuckDB database...");
    }
    
    let db_path = output_dir.join(db_name);
    load_into_duckdb(&sql_path, &db_path)?;
    
    println!("\n✅ Indexing complete!");
    println!("   Database: {}", db_path.display());
    
    // Show summary
    show_summary(&db_path)?;
    
    Ok(())
}

fn get_cache_path(cache_dir: &Path, repo: &Path, file: &Path) -> PathBuf {
    let relative = file.strip_prefix(repo).unwrap_or(file);
    let mut cache_path = cache_dir.join(relative);
    let extension = format!("{}.json", 
        cache_path.extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("")
    );
    cache_path.set_extension(extension);
    cache_path
}

fn should_skip_parse(source: &Path, cache: &Path) -> Result<bool> {
    if !cache.exists() {
        return Ok(false);
    }
    
    let source_modified = fs::metadata(source)?.modified()?;
    let cache_modified = fs::metadata(cache)?.modified()?;
    
    Ok(cache_modified > source_modified)
}

fn show_summary(db_path: &Path) -> Result<()> {
    let stats = patina::pipeline::duckdb::get_stats(db_path)?;
    println!("\n📊 Summary:");
    for line in stats.lines().skip(1) {  // Skip the "Database Statistics:" header
        println!("  {}", line);
    }
    Ok(())
}