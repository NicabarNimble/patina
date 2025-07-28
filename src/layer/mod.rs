use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    Core,
    Topic(String),
    Project(String),
}

#[derive(Debug)]
pub struct Pattern {
    pub name: String,
    pub pattern_type: PatternType,
    pub content: String,
}

pub struct Layer {
    root_path: PathBuf,
}

impl Layer {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
        }
    }
    
    /// Initialize layer directory structure if it doesn't exist
    pub fn init(&self) -> Result<()> {
        let dirs = [
            self.root_path.join("core"),
            self.root_path.join("topics"),
            self.root_path.join("projects"),
            self.root_path.join("adapters"),
        ];
        
        for dir in &dirs {
            fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }
        
        Ok(())
    }
    
    /// Store a pattern in the appropriate location
    pub fn store_pattern(&self, pattern: &Pattern) -> Result<()> {
        let path = self.get_pattern_path(&pattern.pattern_type, &pattern.name)?;
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory for: {}", path.display()))?;
        }
        
        fs::write(&path, &pattern.content)
            .with_context(|| format!("Failed to write pattern to: {}", path.display()))?;
        
        Ok(())
    }
    
    /// Get a specific pattern by type and name
    pub fn get_pattern(&self, pattern_type: &PatternType, name: &str) -> Result<Pattern> {
        let path = self.get_pattern_path(pattern_type, name)?;
        
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read pattern from: {}", path.display()))?;
        
        Ok(Pattern {
            name: name.to_string(),
            pattern_type: pattern_type.clone(),
            content,
        })
    }
    
    /// Get all patterns of a specific type
    pub fn get_patterns(&self, pattern_type: &PatternType) -> Result<Vec<Pattern>> {
        let dir = self.get_pattern_dir(pattern_type)?;
        
        if !dir.exists() {
            return Ok(vec![]);
        }
        
        let mut patterns = Vec::new();
        
        // For topics and projects, we need to handle subdirectories
        match pattern_type {
            PatternType::Core => {
                for entry in fs::read_dir(&dir)? {
                    let entry = entry?;
                    if entry.path().extension().map_or(false, |ext| ext == "md") {
                        let name = entry.path().file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or_default()
                            .to_string();
                        
                        if let Ok(pattern) = self.get_pattern(pattern_type, &name) {
                            patterns.push(pattern);
                        }
                    }
                }
            }
            PatternType::Topic(_) => {
                // List all topics
                for topic_entry in fs::read_dir(&dir)? {
                    let topic_entry = topic_entry?;
                    if topic_entry.path().is_dir() {
                        let topic_name = topic_entry.file_name().to_string_lossy().to_string();
                        let topic_dir = topic_entry.path();
                        
                        for file_entry in fs::read_dir(&topic_dir)? {
                            let file_entry = file_entry?;
                            if file_entry.path().extension().map_or(false, |ext| ext == "md") {
                                let file_name = file_entry.path().file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or_default()
                                    .to_string();
                                
                                let pattern_type = PatternType::Topic(topic_name.clone());
                                if let Ok(pattern) = self.get_pattern(&pattern_type, &file_name) {
                                    patterns.push(pattern);
                                }
                            }
                        }
                    }
                }
            }
            PatternType::Project(_) => {
                // List all projects
                for project_entry in fs::read_dir(&dir)? {
                    let project_entry = project_entry?;
                    if project_entry.path().is_dir() {
                        let project_name = project_entry.file_name().to_string_lossy().to_string();
                        let project_dir = project_entry.path();
                        
                        for file_entry in fs::read_dir(&project_dir)? {
                            let file_entry = file_entry?;
                            if file_entry.path().extension().map_or(false, |ext| ext == "md") {
                                let file_name = file_entry.path().file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or_default()
                                    .to_string();
                                
                                let pattern_type = PatternType::Project(project_name.clone());
                                if let Ok(pattern) = self.get_pattern(&pattern_type, &file_name) {
                                    patterns.push(pattern);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(patterns)
    }
    
    /// Get all patterns across all types for context generation
    pub fn get_all_patterns(&self) -> Result<Vec<Pattern>> {
        let mut all_patterns = Vec::new();
        
        // Get core patterns
        all_patterns.extend(self.get_patterns(&PatternType::Core)?);
        
        // Get all topic patterns
        all_patterns.extend(self.get_patterns(&PatternType::Topic(String::new()))?);
        
        // Get all project patterns
        all_patterns.extend(self.get_patterns(&PatternType::Project(String::new()))?);
        
        Ok(all_patterns)
    }
    
    /// Get the directory path for a pattern type
    fn get_pattern_dir(&self, pattern_type: &PatternType) -> Result<PathBuf> {
        let dir = match pattern_type {
            PatternType::Core => self.root_path.join("core"),
            PatternType::Topic(_) => self.root_path.join("topics"),
            PatternType::Project(_) => self.root_path.join("projects"),
        };
        
        Ok(dir)
    }
    
    /// Get the file path for a specific pattern
    fn get_pattern_path(&self, pattern_type: &PatternType, name: &str) -> Result<PathBuf> {
        let path = match pattern_type {
            PatternType::Core => self.root_path.join("core").join(format!("{}.md", name)),
            PatternType::Topic(topic) => self.root_path.join("topics").join(topic).join(format!("{}.md", name)),
            PatternType::Project(project) => self.root_path.join("projects").join(project).join(format!("{}.md", name)),
        };
        
        Ok(path)
    }
}