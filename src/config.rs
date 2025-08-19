use anyhow::Result;
use std::path::PathBuf;

/// Configuration for Patina
#[derive(Debug, Clone)]
pub struct Config {
    /// Cache directory for databases and indexes
    pub cache_dir: PathBuf,
    /// Project root directory
    pub project_root: PathBuf,
}

impl Config {
    /// Load configuration
    pub fn load() -> Result<Self> {
        let project_root = std::env::current_dir()?;
        let cache_dir = project_root.join(".patina").join("cache");

        // Ensure cache directory exists
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            project_root,
        })
    }
}
