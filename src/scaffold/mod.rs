use anyhow::Result;
use std::path::Path;

pub struct Scaffold {
    templates_path: std::path::PathBuf,
}

impl Scaffold {
    pub fn new(templates_path: impl AsRef<Path>) -> Self {
        Self {
            templates_path: templates_path.as_ref().to_path_buf(),
        }
    }
    
    pub fn create_project(&self, name: &str, llm: &str, dev: &str) -> Result<()> {
        // TODO: Create project structure
        // 1. Create directories
        // 2. Copy templates
        // 3. Set up LLM-specific files
        Ok(())
    }
}