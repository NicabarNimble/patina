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
    if let Ok(repo_dir) = std::env::var("PATINA_SESSION_TOY_GIT_DIR") {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("tag")
            .arg("-a")
            .arg(name)
            .arg("-m")
            .arg(format!("Session toy tag: {}", name))
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("git tag failed: {}", stderr));
        }
        return Ok(());
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_artifact_sections() {
        let temp = tempfile::tempdir().unwrap();
        let artifact = temp.path().join("session.md");
        unsafe {
            std::env::set_var("PATINA_SESSION_ARTIFACT", &artifact);
        }

        write_artifact("note", "captured insight").unwrap();
        write_handoff("a.rs\nb.rs", "handoff summary").unwrap();

        let body = std::fs::read_to_string(&artifact).unwrap();
        assert!(body.contains("## note"));
        assert!(body.contains("captured insight"));
        assert!(body.contains("## Handoff"));

        unsafe {
            std::env::remove_var("PATINA_SESSION_ARTIFACT");
        }
    }

    #[test]
    fn creates_real_git_tag_in_temp_repo() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();

        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .arg("init")
            .status()
            .unwrap()
            .success());
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["config", "user.email", "session-toy@test.local"])
            .status()
            .unwrap()
            .success());
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["config", "user.name", "Session Toy"])
            .status()
            .unwrap()
            .success());

        let file = repo.join("README.md");
        std::fs::write(&file, "hello\n").unwrap();
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["add", "README.md"])
            .status()
            .unwrap()
            .success());
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["commit", "-m", "init"])
            .status()
            .unwrap()
            .success());

        unsafe {
            std::env::set_var("PATINA_SESSION_TOY_GIT_DIR", repo);
        }
        create_tag("session-toy-test-tag").unwrap();
        unsafe {
            std::env::remove_var("PATINA_SESSION_TOY_GIT_DIR");
        }

        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .arg("tag")
            .output()
            .unwrap();
        let tags = String::from_utf8_lossy(&output.stdout);
        assert!(tags.contains("session-toy-test-tag"));
    }
}
