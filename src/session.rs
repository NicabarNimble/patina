use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub patterns: Vec<SessionPattern>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionPattern {
    pub pattern_type: String,
    pub name: String,
    pub content: Option<String>,
    pub added_at: String,
    pub committed: bool,
}

impl Session {
    pub fn new() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            patterns: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
    
    pub fn add_pattern(&mut self, pattern_type: String, name: String) {
        let pattern = SessionPattern {
            pattern_type,
            name,
            content: None,
            added_at: chrono::Utc::now().to_rfc3339(),
            committed: false,
        };
        
        self.patterns.push(pattern);
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
    
    pub fn uncommitted_patterns(&self) -> Vec<&SessionPattern> {
        self.patterns.iter()
            .filter(|p| !p.committed)
            .collect()
    }
    
    pub fn mark_committed(&mut self, pattern_names: &[String]) {
        for pattern in &mut self.patterns {
            if pattern_names.contains(&pattern.name) {
                pattern.committed = true;
            }
        }
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

pub struct SessionManager {
    project_root: PathBuf,
}

impl SessionManager {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
        }
    }
    
    pub fn find_project_root() -> Result<PathBuf> {
        let mut current = std::env::current_dir()?;
        
        loop {
            if current.join(".patina").exists() {
                return Ok(current);
            }
            
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                anyhow::bail!("Not in a Patina project directory");
            }
        }
    }
    
    fn session_file(&self) -> PathBuf {
        self.project_root.join(".patina").join("session.json")
    }
    
    pub fn current_session(&self) -> Result<Option<Session>> {
        let session_file = self.session_file();
        
        if !session_file.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(&session_file)
            .with_context(|| format!("Failed to read session file: {}", session_file.display()))?;
        
        let session: Session = serde_json::from_str(&content)
            .with_context(|| "Failed to parse session file")?;
        
        Ok(Some(session))
    }
    
    pub fn create_session(&self) -> Result<Session> {
        let session = Session::new();
        self.save_session(&session)?;
        Ok(session)
    }
    
    pub fn save_session(&self, session: &Session) -> Result<()> {
        let session_file = self.session_file();
        
        // Ensure .patina directory exists
        if let Some(parent) = session_file.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(session)?;
        fs::write(&session_file, content)
            .with_context(|| format!("Failed to write session file: {}", session_file.display()))?;
        
        Ok(())
    }
    
    pub fn get_or_create_session(&self) -> Result<Session> {
        if let Some(session) = self.current_session()? {
            Ok(session)
        } else {
            self.create_session()
        }
    }
}