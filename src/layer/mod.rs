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
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory for: {}", path.display())
            })?;
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
                    if entry.path().extension().is_some_and(|ext| ext == "md") {
                        let name = entry
                            .path()
                            .file_stem()
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
                            if file_entry.path().extension().is_some_and(|ext| ext == "md") {
                                let file_name = file_entry
                                    .path()
                                    .file_stem()
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
                            if file_entry.path().extension().is_some_and(|ext| ext == "md") {
                                let file_name = file_entry
                                    .path()
                                    .file_stem()
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
            PatternType::Core => self.root_path.join("core").join(format!("{name}.md")),
            PatternType::Topic(topic) => self
                .root_path
                .join("topics")
                .join(topic)
                .join(format!("{name}.md")),
            PatternType::Project(project) => self
                .root_path
                .join("projects")
                .join(project)
                .join(format!("{name}.md")),
        };

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pattern_type_equality() {
        assert_eq!(PatternType::Core, PatternType::Core);
        assert_eq!(
            PatternType::Topic("testing".to_string()),
            PatternType::Topic("testing".to_string())
        );
        assert_ne!(
            PatternType::Topic("testing".to_string()),
            PatternType::Topic("architecture".to_string())
        );
        assert_ne!(
            PatternType::Core,
            PatternType::Project("patina".to_string())
        );
    }

    #[test]
    fn test_layer_new() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);
        assert_eq!(layer.root_path, temp_dir.path());
    }

    #[test]
    fn test_layer_init_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);

        layer.init().unwrap();

        assert!(temp_dir.path().join("core").exists());
        assert!(temp_dir.path().join("topics").exists());
        assert!(temp_dir.path().join("projects").exists());
        assert!(temp_dir.path().join("adapters").exists());
    }

    #[test]
    fn test_store_and_get_core_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);
        layer.init().unwrap();

        let pattern = Pattern {
            name: "test-pattern".to_string(),
            pattern_type: PatternType::Core,
            content: "# Test Pattern\n\nThis is a test.".to_string(),
        };

        // Store pattern
        layer.store_pattern(&pattern).unwrap();

        // Verify file exists
        let expected_path = temp_dir.path().join("core").join("test-pattern.md");
        assert!(expected_path.exists());

        // Get pattern and verify content
        let retrieved = layer
            .get_pattern(&PatternType::Core, "test-pattern")
            .unwrap();
        assert_eq!(retrieved.name, pattern.name);
        assert_eq!(retrieved.content, pattern.content);
    }

    #[test]
    fn test_store_and_get_topic_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);
        layer.init().unwrap();

        let pattern = Pattern {
            name: "auth-pattern".to_string(),
            pattern_type: PatternType::Topic("security".to_string()),
            content: "# Auth Pattern\n\nSecurity guidelines.".to_string(),
        };

        layer.store_pattern(&pattern).unwrap();

        let retrieved = layer
            .get_pattern(&PatternType::Topic("security".to_string()), "auth-pattern")
            .unwrap();
        assert_eq!(retrieved.name, pattern.name);
        assert_eq!(retrieved.content, pattern.content);
    }

    #[test]
    fn test_get_nonexistent_pattern_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);
        layer.init().unwrap();

        let result = layer.get_pattern(&PatternType::Core, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_patterns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);
        layer.init().unwrap();

        let patterns = layer.get_patterns(&PatternType::Core).unwrap();
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_list_patterns_with_content() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);
        layer.init().unwrap();

        // Add some patterns
        for i in 1..=3 {
            let pattern = Pattern {
                name: format!("pattern-{}", i),
                pattern_type: PatternType::Core,
                content: format!("Content {}", i),
            };
            layer.store_pattern(&pattern).unwrap();
        }

        let patterns = layer.get_patterns(&PatternType::Core).unwrap();
        assert_eq!(patterns.len(), 3);

        // Verify all patterns are retrieved
        let names: Vec<String> = patterns.iter().map(|p| p.name.clone()).collect();
        assert!(names.contains(&"pattern-1".to_string()));
        assert!(names.contains(&"pattern-2".to_string()));
        assert!(names.contains(&"pattern-3".to_string()));
    }

    #[test]
    fn test_get_all_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);
        layer.init().unwrap();

        // Add patterns of different types
        layer
            .store_pattern(&Pattern {
                name: "core-pattern".to_string(),
                pattern_type: PatternType::Core,
                content: "Core content".to_string(),
            })
            .unwrap();

        layer
            .store_pattern(&Pattern {
                name: "topic-pattern".to_string(),
                pattern_type: PatternType::Topic("testing".to_string()),
                content: "Topic content".to_string(),
            })
            .unwrap();

        layer
            .store_pattern(&Pattern {
                name: "project-pattern".to_string(),
                pattern_type: PatternType::Project("patina".to_string()),
                content: "Project content".to_string(),
            })
            .unwrap();

        let all_patterns = layer.get_all_patterns().unwrap();
        assert_eq!(all_patterns.len(), 3);
    }

    #[test]
    fn test_pattern_path_generation() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);

        // Test core pattern path
        let core_path = layer.get_pattern_path(&PatternType::Core, "test").unwrap();
        assert_eq!(core_path, temp_dir.path().join("core").join("test.md"));

        // Test topic pattern path
        let topic_path = layer
            .get_pattern_path(&PatternType::Topic("architecture".to_string()), "design")
            .unwrap();
        assert_eq!(
            topic_path,
            temp_dir
                .path()
                .join("topics")
                .join("architecture")
                .join("design.md")
        );

        // Test project pattern path
        let project_path = layer
            .get_pattern_path(&PatternType::Project("myapp".to_string()), "readme")
            .unwrap();
        assert_eq!(
            project_path,
            temp_dir
                .path()
                .join("projects")
                .join("myapp")
                .join("readme.md")
        );
    }
}
