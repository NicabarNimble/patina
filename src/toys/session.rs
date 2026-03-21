use std::fs;
use std::path::PathBuf;

pub fn ensure_granted(session_granted: bool, plugin_name: &str) -> Result<(), String> {
    if session_granted {
        Ok(())
    } else {
        Err(format!(
            "session toy not granted for plugin '{}'",
            plugin_name
        ))
    }
}

pub fn get_session_id() -> String {
    std::env::var("PATINA_SESSION_ID").unwrap_or_default()
}

pub fn get_previous_session() -> Option<String> {
    let root = crate::session::SessionManager::find_project_root().ok()?;
    let path = root.join(".patina/local/last-session.md");
    let value = fs::read_to_string(path).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn write_artifact(section: &str, content: &str) -> Result<(), String> {
    let path = active_artifact_path()?;
    let mut current = fs::read_to_string(&path).unwrap_or_default();
    if !current.ends_with('\n') {
        current.push('\n');
    }
    current.push_str(&format!("\n## {}\n{}\n", section, content));
    fs::write(&path, current).map_err(|e| e.to_string())
}

pub fn create_tag(name: &str) -> Result<(), String> {
    crate::git::create_tag(name, &format!("Session toy tag: {}", name)).map_err(|e| e.to_string())
}

pub fn set_status(status: &str) -> Result<(), String> {
    write_artifact("Status", status)
}

pub fn write_handoff(modified_files: &str, summary: &str) -> Result<(), String> {
    let content = format!(
        "### Modified Files\n{}\n\n### Summary\n{}\n",
        modified_files, summary
    );
    write_artifact("Handoff", &content)
}

fn active_artifact_path() -> Result<PathBuf, String> {
    std::env::var("PATINA_SESSION_ARTIFACT")
        .map(PathBuf::from)
        .map_err(|_| "PATINA_SESSION_ARTIFACT is not set".to_string())
}
