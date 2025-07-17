pub mod claude;
pub mod openai;
pub mod local;

use anyhow::Result;
use std::path::Path;
use toml::Value;
use crate::brain::Pattern;
use crate::environment::Environment;

/// Trait for LLM-specific implementations
pub trait LLMAdapter {
    /// Get the name of this LLM adapter
    fn name(&self) -> &'static str;
    
    /// Initialize LLM-specific files and directories during project creation
    fn init_project(&self, project_path: &Path, design: &Value, environment: &Environment) -> Result<()>;
    
    /// Generate LLM-specific context from patterns and environment
    fn generate_context(
        &self,
        project_path: &Path,
        project_name: &str,
        design_content: &str,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<()>;
    
    /// Update existing context with latest information
    fn update_context(
        &self,
        project_path: &Path,
        project_name: &str,
        design: &Value,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<()>;
    
    /// Get custom commands for this LLM
    fn get_custom_commands(&self) -> Vec<(&'static str, &'static str)> {
        vec![]
    }
    
    /// Get the main context file path for this LLM
    fn get_context_file_path(&self, project_path: &Path) -> std::path::PathBuf;
}

/// Get an LLM adapter by name
pub fn get_adapter(llm_name: &str) -> Box<dyn LLMAdapter> {
    match llm_name.to_lowercase().as_str() {
        "claude" => Box::new(claude::ClaudeAdapter),
        "openai" | "gpt" => Box::new(openai::OpenAIAdapter),
        "local" | "ollama" => Box::new(local::LocalAdapter),
        _ => Box::new(claude::ClaudeAdapter), // Default to Claude
    }
}