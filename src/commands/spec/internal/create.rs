use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;

use patina::spec::{serialize_spec_file, Sessions, SpecFrontmatter, SpecStatus, SpecType};

use super::mutations::git_stage_and_commit;
use super::queue::tag_exists;
use super::DB_PATH;

/// Typed result for spec create command.
#[derive(Debug, Serialize)]
pub struct CreateResult {
    pub command: &'static str,
    pub spec_id: String,
    pub spec_type: String,
    pub status: &'static str,
    pub path: String,
    pub directory: String,
    pub session_origin: Option<String>,
}

/// Body template for each spec type.
fn body_template(spec_type: SpecType) -> &'static str {
    match spec_type {
        SpecType::Feat => {
            "## Problem\n\n## Goal\n\n## Status\n\n## Non-Goals\n\n## Target Shape\n\n## Solution\n\n## Implementation Order\n\n## Resolved Decisions\n\n## Verification\n\n## Exit Criteria\n\n## Build Readiness\n"
        }
        SpecType::Fix => "## Problem\n\n## Root Cause\n\n## Fix\n\n## Exit Criteria\n",
        SpecType::Refactor => {
            "## Problem\n\n## Goal\n\n## Status\n\n## Non-Goals\n\n## Current State\n\n## Target State\n\n## Solution\n\n## Implementation Order\n\n## Resolved Decisions\n\n## Verification\n\n## Exit Criteria\n\n## Build Readiness\n"
        }
        SpecType::Explore => "## Question\n\n## Findings\n\n## Conclusions\n",
    }
}

/// Whether this spec type gets a DESIGN.md scaffold.
fn needs_design_doc(spec_type: SpecType) -> bool {
    matches!(spec_type, SpecType::Feat | SpecType::Refactor)
}

/// DESIGN.md template content.
fn design_template(title: &str) -> String {
    format!(
        "# Design: {}\n\n## Why This Design\n\n## Build Target\n\n## Resolved Decisions\n\n## Commits\n1. `commit message` — what and why\n\n## Direct Code Targets\n- `path/to/file.rs:line` — exact change location\n\n## Verification Plan\n\n## Build Readiness\n\n## Open Questions\n",
        title
    )
}

/// Detect active session ID from .patina/local/active-session.md frontmatter.
fn active_session_id() -> Option<String> {
    let project_root = patina::session::SessionManager::find_project_root().ok()?;
    patina::session::current_session_file_id(&project_root)
        .ok()
        .flatten()
}

/// Validate kebab-case identifier: ^[a-z][a-z0-9-]*$
fn is_valid_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Create a new spec draft — scaffold directory, write frontmatter, commit.
pub fn create_spec(
    type_str: &str,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    blocked_by: Vec<String>,
    related: Vec<String>,
    json: bool,
) -> Result<()> {
    let result = create_spec_value(type_str, id, title, description, blocked_by, related)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Created: {}", result.path);
        println!("  Type:    {}", result.spec_type);
        println!("  Status:  {}", result.status);
        if let Some(ref session) = result.session_origin {
            println!("  Session: {}", session);
        }
        println!();
        println!("Edit: $EDITOR {}", result.path);
    }

    Ok(())
}

/// Create a new spec draft and return structured result (for MCP).
pub fn create_spec_value(
    type_str: &str,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    blocked_by: Vec<String>,
    related: Vec<String>,
) -> Result<CreateResult> {
    // 1. Parse and validate type
    let spec_type: SpecType = type_str
        .parse()
        .map_err(|e: patina::spec::SpecTypeError| anyhow::anyhow!("{}", e))?;

    // 2. Validate id is kebab-case
    if !is_valid_id(id) {
        anyhow::bail!(
            "Invalid spec id \"{}\". Must be kebab-case: starts with lowercase letter, \
             contains only lowercase letters, digits, and hyphens.",
            id
        );
    }

    // 3. Check directory doesn't already exist
    let type_str = spec_type.as_str();
    let directory = format!("layer/surface/build/{}/{}", type_str, id);
    let spec_path = format!("{}/SPEC.md", directory);

    if Path::new(&directory).exists() {
        anyhow::bail!(
            "Directory already exists: {}\n  \
             Spec '{}' may already be created. Check with: patina spec list",
            directory,
            id
        );
    }

    // 4. Check archive tag doesn't exist (prevents collision with archived specs)
    let archive_tag = format!("spec/{}", id);
    if tag_exists(&archive_tag)? {
        anyhow::bail!(
            "Tag '{}' already exists. A spec with this id was previously archived.\n  \
             Recover: git show {}:SPEC.md",
            archive_tag,
            archive_tag
        );
    }

    // 5. Create directory
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("Failed to create directory {}", directory))?;

    // 6. Detect active session
    let session_origin = active_session_id();

    // 7. Build frontmatter
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let sessions = session_origin.as_ref().map(|origin| Sessions::Structured {
        origin: Some(origin.clone()),
        work: Vec::new(),
        updated: None,
    });

    let frontmatter = SpecFrontmatter {
        r#type: type_str.to_string(),
        id: id.to_string(),
        status: Some(SpecStatus::Draft),
        created: Some(today),
        sessions,
        blocked_by,
        related,
        ..Default::default()
    };

    // 8. Build body
    let display_title = title.unwrap_or(id);
    let heading = format!("# {}: {}\n", type_str, display_title);
    let blockquote = if let Some(desc) = description {
        format!("\n> {}\n", desc)
    } else {
        String::new()
    };
    let template = body_template(spec_type);
    let body = format!("\n{}{}\n{}", heading, blockquote, template);

    // 9. Write SPEC.md
    let content = serialize_spec_file(&frontmatter, &body)?;
    std::fs::write(&spec_path, &content)
        .with_context(|| format!("Failed to write {}", spec_path))?;

    // 9b. Write DESIGN.md for feat and refactor types
    if needs_design_doc(spec_type) {
        let design_path = format!("{}/DESIGN.md", directory);
        let design_content = design_template(display_title);
        std::fs::write(&design_path, &design_content)
            .with_context(|| format!("Failed to write {}", design_path))?;
    }

    // 10. Git commit (stage directory to include both SPEC.md and DESIGN.md)
    let commit_msg = format!("spec: draft {}", id);
    git_stage_and_commit(&directory, &commit_msg)?;

    // 11. Update database
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        let conn = Connection::open(db_path).context("Failed to open database")?;
        conn.execute(
            "INSERT OR REPLACE INTO patterns (id, title, layer, status, file_path) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, display_title, "surface", SpecStatus::Draft.as_str(), spec_path],
        )?;
    }

    Ok(CreateResult {
        command: "create",
        spec_id: id.to_string(),
        spec_type: type_str.to_string(),
        status: "draft",
        path: spec_path,
        directory,
        session_origin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use patina::spec::SPEC_TYPES;

    #[test]
    fn test_valid_ids() {
        assert!(is_valid_id("my-feature"));
        assert!(is_valid_id("fix123"));
        assert!(is_valid_id("a"));
        assert!(is_valid_id("spec-create"));
    }

    #[test]
    fn test_invalid_ids() {
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("123abc"));
        assert!(!is_valid_id("My-Feature"));
        assert!(!is_valid_id("-starts-with-dash"));
        assert!(!is_valid_id("has_underscore"));
        assert!(!is_valid_id("has space"));
    }

    #[test]
    fn test_needs_design_doc() {
        assert!(needs_design_doc(SpecType::Feat));
        assert!(needs_design_doc(SpecType::Refactor));
        assert!(!needs_design_doc(SpecType::Fix));
        assert!(!needs_design_doc(SpecType::Explore));
    }

    #[test]
    fn test_design_template_has_sections() {
        let content = design_template("My Feature");
        assert!(content.starts_with("# Design: My Feature"));
        assert!(content.contains("## Why This Design"));
        assert!(content.contains("## Build Target"));
        assert!(content.contains("## Resolved Decisions"));
        assert!(content.contains("## Commits"));
        assert!(content.contains("## Direct Code Targets"));
        assert!(content.contains("## Verification Plan"));
        assert!(content.contains("## Build Readiness"));
        assert!(content.contains("## Open Questions"));
    }

    #[test]
    fn test_body_templates_not_empty() {
        for &name in SPEC_TYPES {
            let t: SpecType = name.parse().unwrap();
            let template = body_template(t);
            assert!(!template.is_empty(), "template for {} is empty", name);
            assert!(
                template.contains("##"),
                "template for {} has no section headings",
                name
            );
        }
    }
}
