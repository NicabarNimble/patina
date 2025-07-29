use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
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
        self.patterns.iter().filter(|p| !p.committed).collect()
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

        let session: Session =
            serde_json::from_str(&content).with_context(|| "Failed to parse session file")?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_new() {
        let session = Session::new();
        assert!(!session.id.is_empty());
        assert!(session.patterns.is_empty());
        assert_eq!(session.created_at, session.updated_at);
    }

    #[test]
    fn test_session_default() {
        let session1 = Session::default();
        let session2 = Session::new();
        // Both should create new sessions with unique IDs
        assert_ne!(session1.id, session2.id);
    }

    #[test]
    fn test_add_pattern() {
        let mut session = Session::new();
        let original_updated = session.updated_at.clone();

        // Add a small delay to ensure timestamps differ
        std::thread::sleep(std::time::Duration::from_millis(10));

        session.add_pattern("core".to_string(), "test-pattern".to_string());

        assert_eq!(session.patterns.len(), 1);
        assert_eq!(session.patterns[0].pattern_type, "core");
        assert_eq!(session.patterns[0].name, "test-pattern");
        assert!(session.patterns[0].content.is_none());
        assert!(!session.patterns[0].committed);
        assert_ne!(session.updated_at, original_updated);
    }

    #[test]
    fn test_uncommitted_patterns() {
        let mut session = Session::new();

        // Add uncommitted patterns
        session.add_pattern("core".to_string(), "pattern1".to_string());
        session.add_pattern("topic".to_string(), "pattern2".to_string());

        // Mark one as committed
        session.patterns[0].committed = true;

        let uncommitted = session.uncommitted_patterns();
        assert_eq!(uncommitted.len(), 1);
        assert_eq!(uncommitted[0].name, "pattern2");
    }

    #[test]
    fn test_mark_committed() {
        let mut session = Session::new();

        session.add_pattern("core".to_string(), "pattern1".to_string());
        session.add_pattern("topic".to_string(), "pattern2".to_string());
        session.add_pattern("project".to_string(), "pattern3".to_string());

        // Mark specific patterns as committed
        session.mark_committed(&["pattern1".to_string(), "pattern3".to_string()]);

        assert!(session.patterns[0].committed);
        assert!(!session.patterns[1].committed);
        assert!(session.patterns[2].committed);
    }

    #[test]
    fn test_session_manager_new() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(&temp_dir);
        assert_eq!(manager.project_root, temp_dir.path());
    }

    #[test]
    fn test_find_project_root() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("myproject");
        let sub_dir = project_dir.join("src").join("commands");

        // Create directory structure
        fs::create_dir_all(&sub_dir).unwrap();
        fs::create_dir(project_dir.join(".patina")).unwrap();

        // Change to subdirectory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sub_dir).unwrap();

        // Test finding project root
        let found_root = SessionManager::find_project_root().unwrap();
        // Normalize paths to handle symlinks (e.g., /private/var vs /var on macOS)
        assert_eq!(
            found_root.canonicalize().unwrap(),
            project_dir.canonicalize().unwrap()
        );

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_find_project_root_not_in_project() {
        let temp_dir = TempDir::new().unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = SessionManager::find_project_root();
        assert!(result.is_err());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_session_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(&temp_dir);

        let session_file = manager.session_file();
        assert_eq!(
            session_file,
            temp_dir.path().join(".patina").join("session.json")
        );
    }

    #[test]
    fn test_current_session_none() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(&temp_dir);

        let session = manager.current_session().unwrap();
        assert!(session.is_none());
    }

    #[test]
    fn test_create_and_save_session() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(&temp_dir);

        let session = manager.create_session().unwrap();
        assert!(!session.id.is_empty());

        // Verify file was created
        let session_file = manager.session_file();
        assert!(session_file.exists());

        // Verify content
        let content = fs::read_to_string(&session_file).unwrap();
        let loaded_session: Session = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded_session.id, session.id);
    }

    #[test]
    fn test_get_or_create_session() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(&temp_dir);

        // First call should create
        let session1 = manager.get_or_create_session().unwrap();

        // Add a pattern to distinguish sessions
        let mut modified_session = session1.clone();
        modified_session.add_pattern("test".to_string(), "pattern".to_string());
        manager.save_session(&modified_session).unwrap();

        // Second call should get existing
        let session2 = manager.get_or_create_session().unwrap();
        assert_eq!(session2.id, modified_session.id);
        assert_eq!(session2.patterns.len(), 1);
    }

    #[test]
    fn test_session_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(&temp_dir);

        // Create session with patterns
        let mut session = Session::new();
        session.add_pattern("core".to_string(), "pattern1".to_string());
        session.add_pattern("topic".to_string(), "pattern2".to_string());
        session.mark_committed(&["pattern1".to_string()]);

        // Save and reload
        manager.save_session(&session).unwrap();
        let loaded = manager.current_session().unwrap().unwrap();

        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.patterns.len(), 2);
        assert!(loaded.patterns[0].committed);
        assert!(!loaded.patterns[1].committed);
    }
}
