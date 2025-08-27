use anyhow::{Context, Result};
use clap::Parser;
use patina::pipeline::{analyze_git, detect_language, discover_files, generate_sql, parse_file};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "patina-index")]
#[command(about = "Index repository code into semantic database")]
struct Args {
    /// Repository path to index
    #[arg(default_value = ".")]
    repo: PathBuf,

    /// Output directory for intermediate files
    #[arg(long, default_value = ".patina")]
    output: PathBuf,

    /// Force re-parse all files (ignore cache)
    #[arg(long)]
    force: bool,

    /// Only parse, don't load into database
    #[arg(long)]
    parse_only: bool,

    /// Verbose output
    #[arg(long, short)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Ensure output directory exists
    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("Failed to create output directory: {:?}", args.output))?;
    
    // Create subdirectories for pipeline stages
    let ast_cache = args.output.join("ast_cache");
    std::fs::create_dir_all(&ast_cache)?;
    
    println!("Indexing repository: {:?}", args.repo);
    println!("Output directory: {:?}", args.output);
    
    // Phase 1: Discovery and Git analysis
    if args.verbose {
        println!("Phase 1: Discovering files and analyzing Git history...");
    }
    
    let files = discover_files(&args.repo)?;
    let git_metrics = analyze_git(&args.repo)?;
    
    println!("Found {} files to index", files.len());
    
    // Save git metrics
    let git_metrics_path = args.output.join("git_metrics.json");
    std::fs::write(
        &git_metrics_path,
        serde_json::to_string_pretty(&git_metrics)?
    )?;
    
    // Phase 2: Parse files to intermediate format
    if args.verbose {
        println!("Phase 2: Parsing files to intermediate format...");
    }
    
    let mut parsed_count = 0;
    let mut skipped_count = 0;
    
    for file in &files {
        let cache_path = get_cache_path(&ast_cache, &args.repo, file);
        
        // Check if we need to parse this file
        if !args.force && should_skip_parse(file, &cache_path)? {
            if args.verbose {
                println!("  Skipping (cached): {}", file.display());
            }
            skipped_count += 1;
            continue;
        }
        
        if args.verbose {
            println!("  Parsing: {}", file.display());
        }
        
        // Parse based on language
        let ast_data = parse_file(file)?;
        
        // Ensure cache directory exists
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Write to cache
        std::fs::write(
            &cache_path,
            serde_json::to_string_pretty(&ast_data)?
        )?;
        
        parsed_count += 1;
    }
    
    println!("Parsed {} files, skipped {} cached files", parsed_count, skipped_count);
    
    if args.parse_only {
        println!("Parse complete (--parse-only specified, skipping database load)");
        return Ok(());
    }
    
    // Phase 3: Generate SQL from intermediate format
    if args.verbose {
        println!("Phase 3: Generating SQL from parsed data...");
    }
    
    let sql_path = args.output.join("load.sql");
    generate_sql(&ast_cache, &sql_path)?;
    
    println!("Generated SQL: {:?}", sql_path);
    
    // Phase 4: Load into DuckDB
    if args.verbose {
        println!("Phase 4: Loading data into DuckDB...");
    }
    
    let db_path = args.output.join("semantic.db");
    load_into_duckdb(&sql_path, &db_path)?;
    
    println!("Database ready: {:?}", db_path);
    
    Ok(())
}


fn get_cache_path(ast_cache: &Path, repo: &Path, file: &Path) -> PathBuf {
    let relative = file.strip_prefix(repo).unwrap_or(file);
    let mut cache_path = ast_cache.join(relative);
    let extension = format!("{}.json", cache_path.extension().unwrap_or_default().to_str().unwrap_or(""));
    cache_path.set_extension(extension);
    cache_path
}

fn should_skip_parse(source: &Path, cache: &Path) -> Result<bool> {
    if !cache.exists() {
        return Ok(false);
    }
    
    let source_modified = std::fs::metadata(source)?.modified()?;
    let cache_modified = std::fs::metadata(cache)?.modified()?;
    
    Ok(cache_modified > source_modified)
}


fn load_into_duckdb(sql_path: &Path, db_path: &Path) -> Result<()> {
    // TODO: Implement DuckDB loading
    println!("TODO: Load SQL from {:?} into {:?}", sql_path, db_path);
    Ok(())
}