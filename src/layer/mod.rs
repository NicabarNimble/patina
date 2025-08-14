use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

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
            self.root_path.join("surface"),
            self.root_path.join("dust"),
            self.root_path.join("sessions"),
        ];

        for dir in &dirs {
            fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }

        Ok(())
    }

    /// Get the sessions directory path
    pub fn sessions_path(&self) -> PathBuf {
        self.root_path.join("sessions")
    }

    /// Get the core directory path
    pub fn core_path(&self) -> PathBuf {
        self.root_path.join("core")
    }

    /// Get the surface directory path
    pub fn surface_path(&self) -> PathBuf {
        self.root_path.join("surface")
    }

    /// Get the dust directory path
    pub fn dust_path(&self) -> PathBuf {
        self.root_path.join("dust")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
        assert!(temp_dir.path().join("surface").exists());
        assert!(temp_dir.path().join("dust").exists());
        assert!(temp_dir.path().join("sessions").exists());
    }

    #[test]
    fn test_path_accessors() {
        let temp_dir = TempDir::new().unwrap();
        let layer = Layer::new(&temp_dir);

        assert_eq!(layer.core_path(), temp_dir.path().join("core"));
        assert_eq!(layer.surface_path(), temp_dir.path().join("surface"));
        assert_eq!(layer.dust_path(), temp_dir.path().join("dust"));
        assert_eq!(layer.sessions_path(), temp_dir.path().join("sessions"));
    }
}
