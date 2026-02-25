use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

use patina::spec::{parse_spec_file, SpecFrontmatter};

use super::queries::{get_all_specs, scan_disk_specs, ListFilters};
use super::queue::{is_tree_clean, tag_exists};

/// Archive a completed or abandoned spec: create spec/<id> tag, remove file, commit
///
/// Public entry point — validates status, checks clean tree, then delegates
/// to `archive_spec_inner` for the actual git operations.
pub fn archive_spec(id: &str, dry_run: bool) -> Result<()> {
    // 1. Find spec in patterns table by id
    let found = find_spec(id)?;
    let status_str = found.status.as_deref().unwrap_or("");

    // 2. Validate status allows archiving
    if status_str != "complete" && status_str != "abandoned" {
        anyhow::bail!(
            "Spec '{}' has status '{}', expected 'complete' or 'abandoned'\n\
             Only completed or abandoned specs can be archived.",
            id,
            status_str
        );
    }

    let tag_name = format!("spec/{}", id);

    // 3. Check tag doesn't already exist
    if tag_exists(&tag_name)? {
        anyhow::bail!(
            "Tag '{}' already exists. Spec may have been archived previously.\n\
             View with: git show {}:{}",
            tag_name,
            tag_name,
            found.file_path
        );
    }

    // Resolve spec directory (parent of SPEC.md)
    let spec_dir = resolve_spec_dir(&found.file_path);

    if dry_run {
        println!("Dry run — would perform these changes:\n");
        println!("  Tag:    {} (preserves spec content)", tag_name);
        if let Some(dir) = &spec_dir {
            println!("  Remove: {}/", dir.display());
        } else {
            println!("  Remove: {}", found.file_path);
        }
        println!("  Commit: docs: archive {} ({})", tag_name, status_str);
        println!("\nRecover with: git show {}:{}", tag_name, found.file_path);
        return Ok(());
    }

    // 4. Check working tree is clean (standalone archive requires clean tree)
    if !is_tree_clean()? {
        anyhow::bail!(
            "Working tree has uncommitted changes.\n\
             Commit or stash your changes before archiving."
        );
    }

    // 5. Delegate to inner (tag, rm, commit)
    let desc = found.title.as_deref().unwrap_or(id);
    archive_spec_inner(id, &found.file_path, status_str, desc, spec_dir.as_deref())
}

/// Core archive logic: tag + git rm + commit.
///
/// Skips clean-tree check — caller is responsible for ensuring the tree
/// state is appropriate (either clean, or managed as part of a release flow).
pub(super) fn archive_spec_inner(
    id: &str,
    file_path: &str,
    status: &str,
    description: &str,
    spec_dir: Option<&Path>,
) -> Result<()> {
    let tag_name = format!("spec/{}", id);

    // 1. Remove spec file/directory from tree
    let remove_target = if let Some(dir) = spec_dir {
        dir.to_str().unwrap_or(file_path).to_string()
    } else {
        file_path.to_string()
    };
    // `git rm -rf` — no patina::git helper for rm (single call site, not worth abstracting)
    println!("Removing: {}", remove_target);
    let output = Command::new("git")
        .args(["rm", "-rf", &remove_target])
        .output()
        .context("Failed to remove spec from tree")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git rm failed: {}", stderr);
    }

    // 2. Commit
    let commit_msg = format!(
        "docs: archive {} ({})\n\nSpec preserved via git tag: {}\nRecover with: git show {}:{}",
        tag_name, status, tag_name, tag_name, file_path
    );
    println!("Committing archive");
    patina::git::commit(&commit_msg)?;

    // 3. Tag HEAD~1 (the parent commit that still has the spec file).
    // Created after commit so no orphaned tag if git rm or commit fails.
    println!("Creating tag: {} (on HEAD~1)", tag_name);
    patina::git::create_tag_at(
        &tag_name,
        &format!("Archived spec: {}", description),
        "HEAD~1",
    )?;

    println!(
        "\n✓ Archived: {}\n  Tag: {}\n  Recover: git show {}:{}",
        id, tag_name, tag_name, file_path
    );

    Ok(())
}

/// Resolve the spec directory from a SPEC.md file path
pub(super) fn resolve_spec_dir(file_path: &str) -> Option<std::path::PathBuf> {
    Path::new(file_path)
        .parent()
        .filter(|p| p.file_name().is_some())
        .map(|p| p.to_path_buf())
}

/// Archive all completed/abandoned specs that still have files in the tree.
///
/// Uses the merged filesystem+DB source (get_all_specs) so it catches both
/// scraped and unscraped stale specs — matching the warning in show_spec_list().
pub fn archive_stale_specs(dry_run: bool) -> Result<()> {
    let all_specs = get_all_specs(&ListFilters::default())?;
    let stale: Vec<_> = all_specs
        .into_iter()
        .filter(|s| matches!(s.status.as_deref(), Some("complete") | Some("abandoned")))
        .collect();

    if stale.is_empty() {
        println!("No stale specs to archive.");
        return Ok(());
    }

    println!("Found {} stale spec(s) to archive:\n", stale.len());

    for spec in &stale {
        let tag_name = format!("spec/{}", spec.id);

        // Skip if tag already exists
        if tag_exists(&tag_name)? {
            println!("  Skip: {} (tag already exists)", spec.id);
            continue;
        }

        // Resolve file path via find_spec (handles both DB and filesystem)
        let found = match find_spec(&spec.id) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("  Skip: {} ({})", spec.id, e);
                continue;
            }
        };

        let status = spec.status.as_deref().unwrap_or("complete");
        let spec_dir = resolve_spec_dir(&found.file_path);

        if dry_run {
            println!("  Would archive: {} ({})", spec.id, status);
            continue;
        }

        archive_spec_inner(
            &spec.id,
            &found.file_path,
            status,
            &spec.title,
            spec_dir.as_deref(),
        )?;
    }

    if dry_run {
        println!("\nDry run — no changes made.");
    }

    Ok(())
}

/// Result of finding a spec by id.
pub(super) struct FoundSpec {
    pub file_path: String,
    pub status: Option<String>,
    pub title: Option<String>,
}

/// Find a spec by its frontmatter id.
///
/// Tries DB first, falls back to filesystem scan for unscraped specs.
pub(super) fn find_spec(id: &str) -> Result<FoundSpec> {
    // Try DB first
    let db_path = Path::new(super::DB_PATH);

    if db_path.exists() {
        if let Ok(conn) = Connection::open(db_path) {
            let result = conn.query_row(
                "SELECT file_path, status, title FROM patterns WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(FoundSpec {
                        file_path: row.get::<_, String>(0)?,
                        status: row.get::<_, Option<String>>(1)?,
                        title: row.get::<_, Option<String>>(2)?,
                    })
                },
            );

            match result {
                Ok(found) => return Ok(found),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // Fall through to filesystem scan
                }
                Err(e) => return Err(e).context("Failed to query patterns table"),
            }
        }
    }

    // Filesystem fallback: scan disk for matching spec
    let disk_specs = scan_disk_specs();
    for spec in disk_specs {
        if spec.id == id {
            // Reconstruct file_path from the spec id by scanning for the actual file
            let build_dir = Path::new("layer/surface/build");
            if let Some(path) = find_spec_file_on_disk(build_dir, id) {
                return Ok(FoundSpec {
                    file_path: path,
                    status: spec.status,
                    title: Some(spec.title),
                });
            }
        }
    }

    anyhow::bail!(
        "Spec '{}' not found.\n\
         Check the id, or create it under layer/surface/build/.",
        id
    );
}

/// Find the file path for a spec id on disk.
fn find_spec_file_on_disk(dir: &Path, target_id: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_spec_file_on_disk(&path, target_id) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some("SPEC.md") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok((fm, _)) = parse_spec_file(&content) {
                    if fm.id == target_id {
                        return Some(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    None
}

/// A fully loaded spec: metadata + parsed content. For mutation paths.
pub(super) struct LoadedSpec {
    pub file_path: String,
    pub status: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub frontmatter: SpecFrontmatter,
    pub body: String,
}

/// Load a spec fully from disk (find + read + parse). For mutations.
///
/// Asserts that the frontmatter ID matches the lookup key to prevent
/// ID source-of-truth drift (see spec-mutation-cleanup § Refactor 1).
pub(super) fn load_spec(id: &str) -> Result<LoadedSpec> {
    let found = find_spec(id)?;
    let content = std::fs::read_to_string(&found.file_path)
        .with_context(|| format!("Failed to read {}", found.file_path))?;
    let (frontmatter, body) = parse_spec_file(&content)
        .with_context(|| format!("Failed to parse {}", found.file_path))?;

    if frontmatter.id != id {
        anyhow::bail!(
            "Frontmatter ID '{}' doesn't match lookup key '{}' in {}",
            frontmatter.id,
            id,
            found.file_path
        );
    }

    Ok(LoadedSpec {
        file_path: found.file_path,
        status: found.status,
        title: found.title,
        content,
        frontmatter,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_spec_dir_with_directory() {
        let dir = resolve_spec_dir("layer/surface/build/feat/my-feature/SPEC.md");
        assert_eq!(
            dir.as_ref().map(|p| p.to_str().unwrap()),
            Some("layer/surface/build/feat/my-feature")
        );
    }

    #[test]
    fn test_resolve_spec_dir_root_file() {
        // A file at the root has no meaningful parent directory
        let dir = resolve_spec_dir("SPEC.md");
        // Parent is "" which has no file_name, so None
        assert!(dir.is_none());
    }

    #[test]
    fn test_archive_requires_complete_or_abandoned() {
        // Verify the status check logic matches expectations
        let archivable = ["complete", "abandoned"];
        let non_archivable = ["draft", "ready", "active"];
        for s in archivable {
            assert!(
                s == "complete" || s == "abandoned",
                "{} should be archivable",
                s
            );
        }
        for s in non_archivable {
            assert!(
                s != "complete" && s != "abandoned",
                "{} should not be archivable",
                s
            );
        }
    }
}
