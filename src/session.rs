/// Session management for Patina projects
///
/// This module provides project discovery functionality for Patina.
/// The original pattern staging system (add/commit workflow with session.json)
/// has been replaced by the git-aware navigation system that automatically
/// tracks patterns through their git lifecycle.
///
/// The SessionManager now focuses on essential project location services
/// used by various Patina commands.
use anyhow::Result;
use std::path::PathBuf;

/// Manages session context for Patina projects
pub struct SessionManager;

impl SessionManager {
    /// Find the root of a Patina project by walking up the directory tree
    /// looking for a .patina directory.
    ///
    /// This is used by commands like `patina navigate` and `patina doctor`
    /// to locate the project configuration and layer directories.
    ///
    /// # Returns
    /// - `Ok(PathBuf)` - The absolute path to the project root
    /// - `Err` - If not in a Patina project directory
    ///
    /// # Example
    /// ```no_run
    /// # use anyhow::Result;
    /// use patina::session::SessionManager;
    ///
    /// # fn main() -> Result<()> {
    /// let project_root = SessionManager::find_project_root()?;
    /// let layer_path = project_root.join("layer");
    /// # Ok(())
    /// # }
    /// ```
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_project_root_from_subdirectory() {
        // Create a temporary project structure
        let temp_dir = TempDir::new().unwrap();
        let patina_dir = temp_dir.path().join(".patina");
        fs::create_dir(&patina_dir).unwrap();

        // Create a subdirectory and navigate to it
        let sub_dir = temp_dir.path().join("src").join("commands");
        fs::create_dir_all(&sub_dir).unwrap();

        // Use a guard to ensure directory is restored even on panic
        struct DirGuard {
            original: Option<PathBuf>,
        }
        impl Drop for DirGuard {
            fn drop(&mut self) {
                if let Some(ref path) = self.original {
                    let _ = std::env::set_current_dir(path);
                }
            }
        }

        let _guard = DirGuard {
            original: std::env::current_dir().ok(),
        };

        std::env::set_current_dir(&sub_dir).unwrap();

        // Should find the project root from the subdirectory
        let found_root = SessionManager::find_project_root().unwrap();
        assert_eq!(
            found_root.canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_project_root_not_in_project() {
        // Create a directory without .patina
        let temp_dir = TempDir::new().unwrap();

        // Use a guard to ensure directory is restored even on panic
        struct DirGuard {
            original: Option<PathBuf>,
        }
        impl Drop for DirGuard {
            fn drop(&mut self) {
                if let Some(ref path) = self.original {
                    let _ = std::env::set_current_dir(path);
                }
            }
        }

        let _guard = DirGuard {
            original: std::env::current_dir().ok(),
        };

        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Should return an error when not in a Patina project
        let result = SessionManager::find_project_root();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Not in a Patina project directory"));
    }

    #[test]
    fn test_find_project_root_at_root() {
        // Create a project at the root level
        let temp_dir = TempDir::new().unwrap();
        let patina_dir = temp_dir.path().join(".patina");
        fs::create_dir(&patina_dir).unwrap();

        // Use a guard to ensure directory is restored even on panic
        struct DirGuard {
            original: Option<PathBuf>,
        }
        impl Drop for DirGuard {
            fn drop(&mut self) {
                if let Some(ref path) = self.original {
                    let _ = std::env::set_current_dir(path);
                }
            }
        }

        let _guard = DirGuard {
            original: std::env::current_dir().ok(),
        };

        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Should find the project root when already at root
        let found_root = SessionManager::find_project_root().unwrap();
        assert_eq!(
            found_root.canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }
}
