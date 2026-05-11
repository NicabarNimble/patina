use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use patina::spec::{serialize_spec_file, Sessions, SpecFrontmatter, SpecStatus, SpecType};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    #[serde(default)]
    pub cross_project: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_project_uid: Option<String>,
}

#[derive(Debug)]
pub struct CreateSpecRequest {
    pub type_str: String,
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub blocked_by: Vec<String>,
    pub related: Vec<String>,
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

/// Detect active session ID for a project root.
fn active_session_id(project_root: &Path) -> Option<String> {
    patina::session::current_session_file_id(project_root)
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

#[derive(Clone, Copy)]
struct CreateDeps {
    git_stage_and_commit: fn(&Path, &str, &str) -> Result<()>,
    upsert_patterns_row: fn(&Path, &str, &str, &str) -> Result<()>,
}

impl Default for CreateDeps {
    fn default() -> Self {
        Self {
            git_stage_and_commit: git_stage_and_commit_at,
            upsert_patterns_row,
        }
    }
}

#[derive(Debug, Clone)]
struct SpecMaterialization {
    directory: String,
    spec_path: String,
    directory_abs: PathBuf,
    spec_path_abs: PathBuf,
}

fn cleanup_materialized_spec_tree(directory_abs: &Path) -> Result<()> {
    if directory_abs.exists() {
        std::fs::remove_dir_all(directory_abs).with_context(|| {
            format!(
                "Failed to remove materialized spec directory {}",
                directory_abs.display()
            )
        })?;
    }
    Ok(())
}

fn upsert_patterns_row(project_root: &Path, id: &str, title: &str, spec_path: &str) -> Result<()> {
    let target_db_path = patina::eventlog::resolve_patina_db_path(project_root);
    if !target_db_path.exists() {
        return Ok(());
    }

    let conn = Connection::open(&target_db_path).context("Failed to open database")?;
    conn.execute(
        "INSERT OR REPLACE INTO patterns (id, title, layer, status, file_path) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, title, "surface", SpecStatus::Draft.as_str(), spec_path],
    )?;
    Ok(())
}

/// Create a new spec draft in a target project root and return structured result (for MCP).
pub fn create_spec_value_for_project(
    project_root: &Path,
    project_uid: Option<&str>,
    request: CreateSpecRequest,
) -> Result<CreateResult> {
    create_spec_value_for_project_with_deps(
        project_root,
        project_uid,
        request,
        CreateDeps::default(),
    )
}

fn create_spec_value_for_project_with_deps(
    project_root: &Path,
    project_uid: Option<&str>,
    request: CreateSpecRequest,
    deps: CreateDeps,
) -> Result<CreateResult> {
    let CreateSpecRequest {
        type_str,
        id,
        title,
        description,
        blocked_by,
        related,
    } = request;

    // 1. Parse and validate type
    let spec_type: SpecType = type_str
        .parse()
        .map_err(|e: patina::spec::SpecTypeError| anyhow::anyhow!("{}", e))?;

    // 2. Validate id is kebab-case
    if !is_valid_id(&id) {
        anyhow::bail!(
            "Invalid spec id \"{}\". Must be kebab-case: starts with lowercase letter, \
             contains only lowercase letters, digits, and hyphens.",
            id
        );
    }

    let type_label = spec_type.as_str();
    let materialized = SpecMaterialization {
        directory: format!("layer/surface/build/{}/{}", type_label, id),
        spec_path: format!("layer/surface/build/{}/{}/SPEC.md", type_label, id),
        directory_abs: project_root.join(format!("layer/surface/build/{}/{}", type_label, id)),
        spec_path_abs: project_root
            .join(format!("layer/surface/build/{}/{}/SPEC.md", type_label, id)),
    };

    // 3. Check directory doesn't already exist
    if materialized.directory_abs.exists() {
        anyhow::bail!(
            "Directory already exists: {}\n  \
             Spec '{}' may already be created. Check with: patina spec list",
            materialized.directory,
            id
        );
    }

    // 4. Check archive tag doesn't exist (prevents collision with archived specs)
    let archive_tag = format!("spec/{}", id);
    if git_tag_exists_at(project_root, &archive_tag)? {
        anyhow::bail!(
            "Tag '{}' already exists. A spec with this id was previously archived.\n  \
             Recover: git -C {} show {}:SPEC.md",
            archive_tag,
            project_root.display(),
            archive_tag
        );
    }

    // 5. Create directory + materialize files
    std::fs::create_dir_all(&materialized.directory_abs).with_context(|| {
        format!(
            "Failed to create directory {}",
            materialized.directory_abs.display()
        )
    })?;

    // 6. Detect active session in target project
    let session_origin = active_session_id(project_root);

    // 7. Build frontmatter
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let sessions = session_origin.as_ref().map(|origin| Sessions::Structured {
        origin: Some(origin.clone()),
        work: Vec::new(),
        updated: None,
    });

    let frontmatter = SpecFrontmatter {
        r#type: type_label.to_string(),
        id: id.to_string(),
        status: Some(SpecStatus::Draft),
        created: Some(today),
        sessions,
        blocked_by,
        related,
        ..Default::default()
    };

    // 8. Build body
    let display_title = title.as_deref().unwrap_or(&id).to_string();
    let heading = format!("# {}: {}\n", type_label, display_title);
    let blockquote = if let Some(desc) = description.as_deref() {
        format!("\n> {}\n", desc)
    } else {
        String::new()
    };
    let template = body_template(spec_type);
    let body = format!("\n{}{}\n{}", heading, blockquote, template);

    let materialize_result = (|| -> Result<()> {
        let content = serialize_spec_file(&frontmatter, &body)?;
        std::fs::write(&materialized.spec_path_abs, &content)
            .with_context(|| format!("Failed to write {}", materialized.spec_path_abs.display()))?;

        if needs_design_doc(spec_type) {
            let design_path_abs = materialized.directory_abs.join("DESIGN.md");
            let design_content = design_template(&display_title);
            std::fs::write(&design_path_abs, &design_content)
                .with_context(|| format!("Failed to write {}", design_path_abs.display()))?;
        }
        Ok(())
    })();

    if let Err(error) = materialize_result {
        let _ = cleanup_materialized_spec_tree(&materialized.directory_abs);
        return Err(error.context("spec create failed during materialization"));
    }

    // 9. Git commit (stage directory to include both SPEC.md and DESIGN.md)
    let commit_msg = format!("spec: draft {}", id);
    if let Err(error) =
        (deps.git_stage_and_commit)(project_root, &materialized.directory, &commit_msg)
    {
        match cleanup_materialized_spec_tree(&materialized.directory_abs) {
            Ok(()) => anyhow::bail!(
                "spec create failed during git stage/commit for '{}': {}\nCompensation: removed materialized spec files at {}",
                id,
                error,
                materialized.directory_abs.display()
            ),
            Err(cleanup_error) => anyhow::bail!(
                "spec create failed during git stage/commit for '{}': {}\nCompensation failure: {}\nManual repair: inspect and clean {}",
                id,
                error,
                cleanup_error,
                materialized.directory_abs.display()
            ),
        }
    }

    // 10. Update target project database
    if let Err(error) =
        (deps.upsert_patterns_row)(project_root, &id, &display_title, &materialized.spec_path)
    {
        anyhow::bail!(
            "spec '{}' was committed, but patterns DB update failed: {}\nRepair:\n  1) Verify file exists: {}\n  2) Reconcile index: patina scrape layer\n  3) Verify spec surface: patina spec show {}",
            id,
            error,
            materialized.spec_path,
            id
        );
    }

    Ok(CreateResult {
        command: "create",
        spec_id: id,
        spec_type: type_label.to_string(),
        status: "draft",
        path: materialized.spec_path,
        directory: materialized.directory,
        session_origin,
        project_uid: project_uid.map(ToString::to_string),
        project_root: Some(project_root.display().to_string()),
        cross_project: false,
        origin_project_uid: None,
    })
}

/// Backward-compatible create helper for current process project.
pub fn create_spec_value(
    type_str: &str,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    blocked_by: Vec<String>,
    related: Vec<String>,
) -> Result<CreateResult> {
    let project_root = std::env::current_dir().context("Failed to resolve current directory")?;
    create_spec_value_for_project(
        &project_root,
        None,
        CreateSpecRequest {
            type_str: type_str.to_string(),
            id: id.to_string(),
            title: title.map(ToString::to_string),
            description: description.map(ToString::to_string),
            blocked_by,
            related,
        },
    )
}

fn run_git(project_root: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run git -C {} {:?}", project_root.display(), args))
}

fn git_tag_exists_at(project_root: &Path, tag: &str) -> Result<bool> {
    let output = run_git(
        project_root,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/tags/{}", tag),
        ],
    )?;
    Ok(output.status.success())
}

fn git_stage_and_commit_at(project_root: &Path, rel_path: &str, message: &str) -> Result<()> {
    // Use -f so spec paths still stage when users have broad global ignores.
    let add = run_git(project_root, &["add", "-f", rel_path])?;
    if !add.status.success() {
        anyhow::bail!(
            "git add failed in {}: {}",
            project_root.display(),
            String::from_utf8_lossy(&add.stderr)
        );
    }

    let staged = run_git(
        project_root,
        &["diff", "--cached", "--quiet", "--exit-code"],
    )?;
    if staged.status.success() {
        return Ok(());
    }

    let commit = run_git(project_root, &["commit", "-m", message])?;
    if !commit.status.success() {
        anyhow::bail!(
            "git commit failed in {}: {}",
            project_root.display(),
            String::from_utf8_lossy(&commit.stderr)
        );
    }
    Ok(())
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

    fn init_git_repo(path: &Path) {
        std::fs::write(path.join("README.md"), "# test\n").expect("write readme");
        assert!(run_git(path, &["init", "-b", "main"])
            .expect("git init")
            .status
            .success());
        assert!(run_git(path, &["config", "user.name", "Patina Test"])
            .expect("git config name")
            .status
            .success());
        assert!(run_git(
            path,
            &["config", "user.email", "patina-test@example.invalid"]
        )
        .expect("git config email")
        .status
        .success());
        assert!(run_git(path, &["add", "README.md"])
            .expect("git add")
            .status
            .success());
        assert!(run_git(path, &["commit", "-m", "init"])
            .expect("git commit")
            .status
            .success());
    }

    fn request_for(id: &str) -> CreateSpecRequest {
        CreateSpecRequest {
            type_str: "fix".to_string(),
            id: id.to_string(),
            title: Some("tx boundary".to_string()),
            description: Some("testing".to_string()),
            blocked_by: vec![],
            related: vec![],
        }
    }

    fn fail_git_stage_and_commit(
        _project_root: &Path,
        _rel_path: &str,
        _message: &str,
    ) -> Result<()> {
        anyhow::bail!("simulated git failure")
    }

    fn fail_patterns_upsert(
        _project_root: &Path,
        _id: &str,
        _title: &str,
        _spec_path: &str,
    ) -> Result<()> {
        anyhow::bail!("simulated db failure")
    }

    #[test]
    fn create_compensates_materialized_files_when_git_commit_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        init_git_repo(temp.path());

        let id = "tx-git-fail";
        let err = create_spec_value_for_project_with_deps(
            temp.path(),
            None,
            request_for(id),
            CreateDeps {
                git_stage_and_commit: fail_git_stage_and_commit,
                upsert_patterns_row,
            },
        )
        .expect_err("expected git failure");

        let err_text = err.to_string();
        assert!(err_text.contains("simulated git failure"), "{err_text}");
        assert!(
            err_text.contains("Compensation: removed materialized spec files"),
            "{err_text}"
        );

        let directory = temp.path().join("layer/surface/build/fix").join(id);
        assert!(
            !directory.exists(),
            "materialized directory should be removed on git failure"
        );
    }

    #[test]
    fn create_reports_repair_steps_when_db_update_fails_after_commit() {
        let temp = tempfile::tempdir().expect("tempdir");
        init_git_repo(temp.path());

        let id = "tx-db-fail";
        let err = create_spec_value_for_project_with_deps(
            temp.path(),
            None,
            request_for(id),
            CreateDeps {
                git_stage_and_commit: git_stage_and_commit_at,
                upsert_patterns_row: fail_patterns_upsert,
            },
        )
        .expect_err("expected db failure");

        let err_text = err.to_string();
        assert!(
            err_text.contains("was committed, but patterns DB update failed"),
            "{err_text}"
        );
        assert!(err_text.contains("patina scrape layer"), "{err_text}");
        assert!(
            err_text.contains(&format!("patina spec show {}", id)),
            "{err_text}"
        );

        let spec_path = temp
            .path()
            .join("layer/surface/build/fix")
            .join(id)
            .join("SPEC.md");
        assert!(
            spec_path.exists(),
            "committed spec file should remain present"
        );

        let log = run_git(temp.path(), &["log", "-1", "--pretty=%s"]).expect("git log");
        let subject = String::from_utf8_lossy(&log.stdout);
        assert!(
            subject.contains(&format!("spec: draft {}", id)),
            "expected create commit subject, got: {}",
            subject
        );
    }

    #[test]
    fn create_happy_path_leaves_fs_git_db_coherent() {
        let temp = tempfile::tempdir().expect("tempdir");
        init_git_repo(temp.path());

        let db_path = patina::eventlog::resolve_patina_db_path(temp.path());
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).expect("create db parent dir");
        }
        let conn = Connection::open(&db_path).expect("open test db");
        conn.execute(
            "CREATE TABLE patterns (id TEXT PRIMARY KEY, title TEXT, layer TEXT, status TEXT, file_path TEXT)",
            [],
        )
        .expect("create patterns table");

        let id = "tx-happy";
        let result = create_spec_value_for_project_with_deps(
            temp.path(),
            None,
            request_for(id),
            CreateDeps::default(),
        )
        .expect("create should succeed");

        let spec_path = temp.path().join(&result.path);
        assert!(spec_path.exists(), "SPEC.md should exist");

        let log = run_git(temp.path(), &["log", "-1", "--pretty=%s"]).expect("git log");
        let subject = String::from_utf8_lossy(&log.stdout);
        assert!(
            subject.contains(&format!("spec: draft {}", id)),
            "expected create commit subject, got: {}",
            subject
        );

        let row: (String, String, String) = conn
            .query_row(
                "SELECT id, status, file_path FROM patterns WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("expected patterns row");
        assert_eq!(row.0, id);
        assert_eq!(row.1, "draft");
        assert_eq!(row.2, result.path);
    }
}
