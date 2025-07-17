use super::LLMAdapter;
use anyhow::Result;
use std::path::{Path, PathBuf};
use toml::Value;
use crate::brain::Pattern;
use crate::environment::Environment;

pub struct OpenAIAdapter;

impl LLMAdapter for OpenAIAdapter {
    fn name(&self) -> &'static str {
        "openai"
    }
    
    fn init_project(&self, _project_path: &Path, _design: &Value, _environment: &Environment) -> Result<()> {
        // Placeholder - will implement when needed
        Ok(())
    }
    
    fn generate_context(
        &self,
        _project_path: &Path,
        _project_name: &str,
        _design_content: &str,
        _patterns: &[Pattern],
        _environment: &Environment,
    ) -> Result<()> {
        // Placeholder - will implement when needed
        Ok(())
    }
    
    fn update_context(
        &self,
        _project_path: &Path,
        _project_name: &str,
        _design: &Value,
        _patterns: &[Pattern],
        _environment: &Environment,
    ) -> Result<()> {
        // Placeholder - will implement when needed
        Ok(())
    }
    
    fn get_context_file_path(&self, project_path: &Path) -> PathBuf {
        project_path.join("CONTEXT.md")
    }
}