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

pub fn get_previous_session_runtime_id() -> Option<String> {
    let path = get_previous_session()?;
    let content = fs::read_to_string(path).ok()?;
    parse_frontmatter_runtime_id(&content)
}

pub fn get_previous_session_handoff() -> Option<String> {
    let path = get_previous_session()?;
    let content = fs::read_to_string(path).ok()?;
    extract_handoff_section(&content)
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

pub fn set_parent_session(runtime_id: &str) -> Result<(), String> {
    let path = active_artifact_path()?;
    let current = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let updated = set_frontmatter_value(&current, "parent_session", runtime_id)?;
    fs::write(path, updated).map_err(|e| e.to_string())
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

fn extract_handoff_section(markdown: &str) -> Option<String> {
    let marker = "## Handoff";
    let start = markdown.find(marker)?;
    let tail = &markdown[start..];
    let next = tail[marker.len()..]
        .find("\n## ")
        .map(|idx| idx + marker.len() + 1);
    let section = match next {
        Some(idx) => &tail[..idx],
        None => tail,
    };
    Some(section.trim_end().to_string())
}

fn set_frontmatter_value(markdown: &str, key: &str, value: &str) -> Result<String, String> {
    let rest = markdown
        .strip_prefix("---\n")
        .ok_or_else(|| "session document is missing YAML frontmatter".to_string())?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| "session document is missing YAML frontmatter terminator".to_string())?;
    let yaml = &rest[..end];
    let body = &rest[end + 5..];

    let mut fm: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|e| format!("invalid frontmatter: {}", e))?;
    let map = fm
        .as_mapping_mut()
        .ok_or_else(|| "frontmatter is not a mapping".to_string())?;
    map.insert(
        serde_yaml::Value::String(key.to_string()),
        serde_yaml::Value::String(value.to_string()),
    );

    let rendered = serde_yaml::to_string(&fm).map_err(|e| e.to_string())?;
    Ok(format!("---\n{}---\n{}", rendered, body))
}

fn parse_frontmatter_runtime_id(markdown: &str) -> Option<String> {
    let rest = markdown.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let yaml = &rest[..end];
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).ok()?;
    value
        .as_mapping()?
        .get(serde_yaml::Value::String("runtime_id".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_artifact_sections() {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
    fn extracts_previous_session_runtime_and_handoff() {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        std::fs::create_dir_all(project.join(".patina/local")).unwrap();
        let previous = project.join("layer/sessions/previous.md");
        std::fs::create_dir_all(previous.parent().unwrap()).unwrap();
        std::fs::write(
            &previous,
            "---\nruntime_id: runtime-123\nid: sess-123\n---\n\n## Handoff\n\n- carry this\n\n## Outcome\n",
        )
        .unwrap();
        std::fs::write(
            project.join(".patina/local/last-session.md"),
            previous.display().to_string(),
        )
        .unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(project).unwrap();
        let runtime = get_previous_session_runtime_id();
        let handoff = get_previous_session_handoff();
        std::env::set_current_dir(old_dir).unwrap();

        assert_eq!(runtime.as_deref(), Some("runtime-123"));
        assert_eq!(handoff.as_deref(), Some("## Handoff\n\n- carry this"));
    }

    #[test]
    fn sets_parent_session_in_frontmatter() {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let artifact = temp.path().join("session.md");
        std::fs::write(
            &artifact,
            "---\nid: a\nruntime_id: r\nstatus: active\n---\n\n## Outcome\n",
        )
        .unwrap();
        unsafe {
            std::env::set_var("PATINA_SESSION_ARTIFACT", &artifact);
        }

        set_parent_session("runtime-parent").unwrap();
        let body = std::fs::read_to_string(&artifact).unwrap();
        assert!(body.contains("parent_session: runtime-parent"));

        unsafe {
            std::env::remove_var("PATINA_SESSION_ARTIFACT");
        }
    }

    #[test]
    fn creates_real_git_tag_in_temp_repo() {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
