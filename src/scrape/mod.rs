use anyhow::Result;
use std::path::{Path, PathBuf};

pub mod discovery;
pub mod extraction;
pub mod storage;
pub mod transform;

/// Thin orchestration layer - just coordinates between layers
pub fn execute(
    init: bool,
    query: Option<String>,
    repo: Option<String>,
    force: bool,
) -> Result<()> {
    let (db_path, work_dir) = determine_paths(repo)?;
    
    if init {
        storage::initialize_database(&db_path)?;
    } else if let Some(q) = query {
        storage::run_query(&q, &db_path)?;
    } else {
        orchestrate_extraction(&db_path, &work_dir, force)?;
    }
    Ok(())
}

/// Orchestrate extraction through all layers
fn orchestrate_extraction(db_path: &str, work_dir: &Path, _force: bool) -> Result<()> {
    // Layer 1: Discovery - find files and detect languages
    eprintln!("Discovering files in {:?}", work_dir);
    let files = discovery::find_files(work_dir)?;
    eprintln!("Found {} files to process", files.len());
    
    // Layer 2: Extraction - extract semantic data from files
    eprintln!("Extracting semantic data...");
    let semantic_data = extraction::extract_all(files)?;
    
    // Layer 3: Transformation - transform to database records
    eprintln!("Transforming to database records...");
    let records = transform::to_records(semantic_data)?;
    eprintln!("Generated {} records", records.len());
    
    // Layer 4: Storage - save to database
    eprintln!("Saving to database...");
    storage::save_batch(db_path, records)?;
    
    eprintln!("✓ Extraction complete");
    Ok(())
}

/// Determine database path and working directory
fn determine_paths(repo: Option<String>) -> Result<(String, PathBuf)> {
    use std::env;
    
    let work_dir = if let Some(repo_path) = repo {
        PathBuf::from(repo_path)
    } else {
        env::current_dir()?
    };
    
    // Use .patina subdirectory for database
    let mut db_path = work_dir.clone();
    db_path.push(".patina");
    db_path.push("semantic.db");
    
    // Ensure .patina directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    Ok((db_path.to_string_lossy().to_string(), work_dir))
}