// Simple file-based "database" for memory
// No complex SQL, just markdown files that LLMs can read directly

use anyhow::Result;
use std::path::Path;

pub struct MemoryDatabase;

impl MemoryDatabase {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MemoryDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryDatabase {
    pub fn ensure_initialized(&self) -> Result<()> {
        // Create memory directories if they don't exist
        std::fs::create_dir_all(".patina")?;

        // Initialize files if they don't exist
        let files = vec![
            (
                ".patina/lessons.md",
                "# Lessons Learned\n\nFailures and insights from development.\n",
            ),
            (
                ".patina/decisions.md",
                "# Design Decisions\n\nWhy we chose X over Y.\n",
            ),
            (
                ".patina/context.md",
                "# Context Memory\n\nRelevant information for different topics.\n",
            ),
        ];

        for (path, header) in files {
            if !Path::new(path).exists() {
                std::fs::write(path, header)?;
            }
        }

        Ok(())
    }
}
