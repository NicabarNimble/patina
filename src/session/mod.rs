mod internal;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub use internal::artifact::{parse_session_ids, rewrite_document_status};
pub use internal::live::LiveSessionHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceKind {
    LegacyCli,
    OpenCode,
    Gemini,
    Claude,
    Unknown,
}

impl InterfaceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LegacyCli => "legacy-cli",
            Self::OpenCode => "opencode",
            Self::Gemini => "gemini",
            Self::Claude => "claude",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_adapter_name(name: &str) -> Self {
        match name {
            "opencode" => Self::OpenCode,
            "gemini" => Self::Gemini,
            "claude" => Self::Claude,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionParticipant {
    pub participant_id: String,
    pub role: String,
    pub interface_kind: InterfaceKind,
    pub adapter_name: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BeginSessionRequest {
    pub title: String,
    pub adapter_name: String,
    pub interface_kind: InterfaceKind,
    pub persona_uid: Option<String>,
    pub parent_runtime_id: Option<String>,
    pub handoff_from_runtime_id: Option<String>,
    pub participant: Option<SessionParticipant>,
}

#[derive(Debug, Clone)]
pub struct SessionStart {
    pub handle: LiveSessionHandle,
    pub document: String,
}

#[derive(Debug, Clone)]
pub struct ArchiveSessionRequest {
    pub runtime_id: String,
    pub markdown: String,
    pub end_tag: Option<String>,
}

/// Manages session context for Patina projects.
pub struct SessionManager;

impl SessionManager {
    /// Find the root of a Patina project by walking up the directory tree.
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

pub fn begin_session(project_root: &Path, request: BeginSessionRequest) -> Result<SessionStart> {
    internal::live::begin_session(project_root, request)
}

pub fn sync_session_document(
    project_root: &Path,
    runtime_id: &str,
    markdown: &str,
) -> Result<LiveSessionHandle> {
    internal::live::sync_session_document(project_root, runtime_id, markdown)
}

pub fn archive_session(
    project_root: &Path,
    request: ArchiveSessionRequest,
) -> Result<LiveSessionHandle> {
    internal::live::archive_session(project_root, request)
}

pub fn list_active_sessions(project_root: &Path) -> Result<Vec<LiveSessionHandle>> {
    internal::live::list_active_sessions(project_root)
}

pub fn find_active_interface_session(
    project_root: &Path,
    adapter_name: &str,
    interface_kind: InterfaceKind,
    persona_uid: Option<&str>,
) -> Result<Option<LiveSessionHandle>> {
    internal::live::find_active_interface_session(
        project_root,
        adapter_name,
        interface_kind,
        persona_uid,
    )
}

pub fn load_session(project_root: &Path, runtime_id: &str) -> Result<Option<LiveSessionHandle>> {
    internal::live::load_session(project_root, runtime_id)
}

pub fn load_session_by_file_id(
    project_root: &Path,
    file_id: &str,
) -> Result<Option<LiveSessionHandle>> {
    internal::live::load_session_by_file_id(project_root, file_id)
}

pub fn current_session_file_id(project_root: &Path) -> Result<Option<String>> {
    internal::live::current_session_file_id(project_root)
}

pub fn current_session_runtime_id(project_root: &Path) -> Result<Option<String>> {
    internal::live::current_session_runtime_id(project_root)
}

pub fn load_current_interface_session(
    project_root: &Path,
    adapter_name: &str,
) -> Result<Option<LiveSessionHandle>> {
    internal::live::load_current_interface_session(project_root, adapter_name)
}

pub fn active_session_path(project_root: &Path) -> PathBuf {
    internal::projection::active_session_path(project_root)
}

pub fn last_session_path(project_root: &Path) -> PathBuf {
    internal::projection::last_session_path(project_root)
}

pub fn durable_session_path(project_root: &Path, file_id: &str) -> PathBuf {
    internal::projection::durable_session_path(project_root, file_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_project_root_from_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join(".patina")).unwrap();
        let sub_dir = temp_dir.path().join("src").join("commands");
        fs::create_dir_all(&sub_dir).unwrap();

        struct DirGuard(Option<PathBuf>);
        impl Drop for DirGuard {
            fn drop(&mut self) {
                if let Some(path) = &self.0 {
                    let _ = std::env::set_current_dir(path);
                }
            }
        }

        let _guard = DirGuard(std::env::current_dir().ok());
        std::env::set_current_dir(&sub_dir).unwrap();

        let found_root = SessionManager::find_project_root().unwrap();
        assert_eq!(
            found_root.canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_project_root_not_in_project() {
        let temp_dir = TempDir::new().unwrap();

        struct DirGuard(Option<PathBuf>);
        impl Drop for DirGuard {
            fn drop(&mut self) {
                if let Some(path) = &self.0 {
                    let _ = std::env::set_current_dir(path);
                }
            }
        }

        let _guard = DirGuard(std::env::current_dir().ok());
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = SessionManager::find_project_root();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Not in a Patina project directory"));
    }
}
