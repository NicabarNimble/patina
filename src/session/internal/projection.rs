use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const ACTIVE_SESSION_PATH: &str = ".patina/local/active-session.md";
const LAST_SESSION_PATH: &str = ".patina/local/last-session.md";
const INTERFACE_SESSIONS_DIR: &str = ".patina/local/interface-sessions";
const SESSIONS_DIR: &str = "layer/sessions";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeInterfaceSessionPointer {
    pub adapter: String,
    pub runtime_id: String,
    pub file_id: String,
}

pub fn active_session_path(project_root: &Path) -> PathBuf {
    project_root.join(ACTIVE_SESSION_PATH)
}

pub fn last_session_path(project_root: &Path) -> PathBuf {
    project_root.join(LAST_SESSION_PATH)
}

pub fn durable_session_path(project_root: &Path, file_id: &str) -> PathBuf {
    project_root
        .join(SESSIONS_DIR)
        .join(format!("{file_id}.md"))
}

pub fn native_interface_session_path(project_root: &Path, adapter: &str) -> PathBuf {
    project_root
        .join(INTERFACE_SESSIONS_DIR)
        .join(format!("{adapter}.toml"))
}

pub fn write_durable_artifact(
    project_root: &Path,
    file_id: &str,
    markdown: &str,
) -> Result<PathBuf> {
    let path = durable_session_path(project_root, file_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating session artifact dir {}", parent.display()))?;
    }
    fs::write(&path, markdown).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

pub fn write_native_interface_session(
    project_root: &Path,
    adapter: &str,
    runtime_id: &str,
    file_id: &str,
) -> Result<()> {
    let path = native_interface_session_path(project_root, adapter);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating interface session dir {}", parent.display()))?;
    }
    let pointer = NativeInterfaceSessionPointer {
        adapter: adapter.to_string(),
        runtime_id: runtime_id.to_string(),
        file_id: file_id.to_string(),
    };
    let serialized = toml::to_string(&pointer)?;
    fs::write(&path, serialized)
        .with_context(|| format!("writing native interface session {}", path.display()))?;
    Ok(())
}

pub fn read_native_interface_session(
    project_root: &Path,
    adapter: &str,
) -> Result<Option<NativeInterfaceSessionPointer>> {
    let path = native_interface_session_path(project_root, adapter);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("reading native interface session {}", path.display()))?;
    let pointer = toml::from_str(&content)
        .with_context(|| format!("parsing native interface session {}", path.display()))?;
    Ok(Some(pointer))
}

pub fn clear_native_interface_session(
    project_root: &Path,
    adapter: &str,
    runtime_id: Option<&str>,
) -> Result<()> {
    let path = native_interface_session_path(project_root, adapter);
    if !path.exists() {
        return Ok(());
    }
    if let Some(expected_runtime_id) = runtime_id {
        if let Some(pointer) = read_native_interface_session(project_root, adapter)? {
            if pointer.runtime_id != expected_runtime_id {
                return Ok(());
            }
        }
    }
    fs::remove_file(&path)
        .with_context(|| format!("clearing native interface session {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn native_interface_session_pointer_round_trips() {
        let temp = TempDir::new().unwrap();
        write_native_interface_session(temp.path(), "claude", "runtime-123", "file-123").unwrap();

        let pointer = read_native_interface_session(temp.path(), "claude")
            .unwrap()
            .unwrap();
        assert_eq!(
            pointer,
            NativeInterfaceSessionPointer {
                adapter: "claude".to_string(),
                runtime_id: "runtime-123".to_string(),
                file_id: "file-123".to_string(),
            }
        );

        clear_native_interface_session(temp.path(), "claude", Some("runtime-123")).unwrap();
        assert!(read_native_interface_session(temp.path(), "claude")
            .unwrap()
            .is_none());
    }
}
