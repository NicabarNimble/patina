use anyhow::{Context, Result};
use std::path::Path;

use patina::release::{BumpType, ReleaseStrategy};

use super::archive::{archive_spec_inner, load_spec, resolve_spec_dir};
use super::mutations::{git_stage_and_commit, mutate_spec};
use super::queue::tag_exists;

/// Split a spec: complete original with release, create new draft for remaining work.
///
/// Flow: validate → tag → complete original → create new spec → commit
pub fn split_spec(
    id: &str,
    new_id: Option<&str>,
    description: Option<&str>,
    json: bool,
) -> Result<()> {
    let result = split_spec_value(id, new_id, description)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let derived_id = result["new_spec_id"].as_str().unwrap_or("");
        let version_tag = result["version_tag"].as_str().unwrap_or("");
        let new_spec_path = result["new_spec_path"].as_str().unwrap_or("");
        let file_path = result["original_file"].as_str().unwrap_or("");
        println!("Split: {}", id);
        println!("  Completed: {} → archived (spec/{})", id, id);
        println!("  Version tag: {}", version_tag);
        println!("  New draft: {} ({})", derived_id, new_spec_path);
        println!("  Recover parent: git show {}:{}", version_tag, file_path);
    }

    Ok(())
}

/// Split a spec and return structured result (for MCP).
pub fn split_spec_value(
    id: &str,
    new_id: Option<&str>,
    description: Option<&str>,
) -> Result<serde_json::Value> {
    // 1. Load spec and validate status (active or paused)
    let loaded = load_spec(id)?;
    match loaded.status.as_deref() {
        Some("active") | Some("paused") => {}
        Some(s) => anyhow::bail!(
            "Cannot split '{}' — status is '{}', expected 'active' or 'paused'",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    let spec_type = loaded.frontmatter.r#type.clone();
    let title_str = loaded.title.as_deref().unwrap_or(id).to_string();

    // 2. Tag current state: spec/<id>-v<N>-complete
    let version_tags = patina::git::list_matching_tags(&format!("spec/{}-v*-complete", id))?;
    let version_n = version_tags.len() as u32 + 1;
    let version_tag = format!("spec/{}-v{}-complete", id, version_n);
    patina::git::create_tag_at(
        &version_tag,
        &format!("Split point: {} v{}", id, version_n),
        "HEAD",
    )?;

    // 3. Complete original spec via mutate_spec (replaces manual read-parse-mutate-write-DB)
    let original_file = loaded.file_path.clone();
    let out = mutate_spec(loaded, |fm| {
        fm.status = Some("complete".to_string());
        Ok(())
    })?;

    // 4. Pre-check archive tag, then release + archive
    let archive_tag = format!("spec/{}", id);
    if tag_exists(&archive_tag)? {
        anyhow::bail!(
            "Tag '{}' already exists. Spec may have been archived previously.",
            archive_tag
        );
    }
    let spec_dir = resolve_spec_dir(&out.file_path);

    let strategy = ReleaseStrategy::from_project(Path::new("."));
    let bump = BumpType::from_spec_type(&spec_type);

    if let Some(bump) = bump {
        let prepared = strategy.preflight(bump, &out.file_path)?;
        let archive_dir = spec_dir
            .as_ref()
            .and_then(|d| d.to_str())
            .or(Some(&out.file_path));
        prepared.execute(&title_str, &out.file_path, archive_dir)?;

        // Tag parent commit (still has spec file)
        println!("Creating tag: {} (on HEAD~1)", archive_tag);
        patina::git::create_tag_at(
            &archive_tag,
            &format!("Archived spec: {}", title_str),
            "HEAD~1",
        )?;
    } else {
        // No release (explore type) — archive as standalone commit
        archive_spec_inner(
            id,
            &out.file_path,
            "complete",
            &title_str,
            spec_dir.as_deref(),
        )?;
    }

    // 5. Determine new spec ID
    let derived_id = if let Some(explicit) = new_id {
        explicit.to_string()
    } else {
        // Default: <id>-v2, -v3, etc.
        let mut n = 2u32;
        loop {
            let candidate = format!("{}-v{}", id, n);
            let candidate_dir = format!("layer/surface/build/{}/{}", spec_type, candidate);
            if !Path::new(&candidate_dir).exists() {
                break candidate;
            }
            n += 1;
        }
    };

    // 6. Create new spec directory + SPEC.md
    let new_spec_dir = format!("layer/surface/build/{}/{}", spec_type, derived_id);
    std::fs::create_dir_all(&new_spec_dir)
        .with_context(|| format!("Failed to create directory {}", new_spec_dir))?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let desc_text = description.unwrap_or("Remaining work from split");
    let new_spec_content = format!(
        "---\ntype: {}\nid: {}\nstatus: draft\ncreated: {}\nsplit_from: {}\n---\n\n# {}\n\n{}\n\n## Recovery\n\nParent spec content: `git show {}:{}`\n",
        spec_type,
        derived_id,
        today,
        id,
        derived_id,
        desc_text,
        version_tag,
        original_file,
    );

    let new_spec_path = format!("{}/SPEC.md", new_spec_dir);
    std::fs::write(&new_spec_path, &new_spec_content)
        .with_context(|| format!("Failed to write {}", new_spec_path))?;

    // 7. Git commit the new spec
    let commit_msg = format!(
        "spec: split {} — ship v{}, draft remainder as {}",
        id, version_n, derived_id
    );
    git_stage_and_commit(&new_spec_path, &commit_msg)?;

    Ok(serde_json::json!({
        "command": "split",
        "original_spec_id": id,
        "new_spec_id": derived_id,
        "version_tag": version_tag,
        "archive_tag": format!("spec/{}", id),
        "new_spec_path": new_spec_path,
        "original_file": original_file,
        "status": "completed",
    }))
}
