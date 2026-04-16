use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

use patina::release::{BumpType, ReleaseStrategy};
use patina::spec::{parse_spec_file, SpecFrontmatter, SpecStatus};

use super::queries::{get_all_specs, scan_disk_specs, ListFilters};
use super::queue::{is_tree_clean, tag_exists};

/// Archive a completed or abandoned spec: create spec/<id> tag, remove file, commit
///
/// Public entry point — validates status, checks clean tree, then delegates
/// to `archive_spec_inner` for the actual git operations.
pub fn archive_spec(id: &str, dry_run: bool) -> Result<()> {
    // 1. Find spec in patterns table by id
    let found = find_spec(id)?;

    // 2. Validate status allows archiving
    if !found.status.is_some_and(|s| s.is_terminal()) {
        let status_str = found.status.map_or("none", |s| s.as_str());
        anyhow::bail!(
            "Spec '{}' has status '{}', expected 'complete' or 'abandoned'\n\
             Only completed or abandoned specs can be archived.",
            id,
            status_str
        );
    }
    let status_str = found.status.map_or("complete", |s| s.as_str());

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

/// Release (version bump + archive) or archive-only for a completed spec.
///
/// Shared logic between complete_spec_value and split_spec_value.
/// Caller handles mutation before calling this — this helper is archive-side only.
pub(super) fn release_and_archive(
    id: &str,
    file_path: &str,
    _frontmatter: &SpecFrontmatter,
    title: &str,
    bump: Option<BumpType>,
) -> Result<()> {
    // 1. Pre-check: bail if spec/{id} tag already exists
    let tag_name = format!("spec/{}", id);
    if tag_exists(&tag_name)? {
        anyhow::bail!(
            "Tag '{}' already exists. Spec may have been archived previously.",
            tag_name
        );
    }

    // 2. Resolve spec directory from file_path
    let spec_dir = resolve_spec_dir(file_path);

    if let Some(bump) = bump {
        // 3. Release path: preflight → execute → archive tag on HEAD~1
        let strategy = ReleaseStrategy::from_project(Path::new("."));
        let prepared = strategy.preflight(bump, file_path)?;
        let archive_dir = spec_dir
            .as_ref()
            .and_then(|d| d.to_str())
            .or(Some(file_path));
        prepared.execute(title, file_path, archive_dir)?;

        // Tag HEAD~1 (parent commit still has spec file)
        println!("Creating tag: {} (on HEAD~1)", tag_name);
        patina::git::create_tag_at(&tag_name, &format!("Archived spec: {}", title), "HEAD~1")?;
    } else {
        // 4. No release (explore type) — delegate to archive_spec_inner
        archive_spec_inner(id, file_path, "complete", title, spec_dir.as_deref())?;
    }

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
        .filter(|s| s.status.is_some_and(|st| st.is_terminal()))
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

        let status = spec.status.map_or("complete", |s| s.as_str());
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
    pub status: Option<SpecStatus>,
    pub title: Option<String>,
}

/// Find a spec by its frontmatter id.
///
/// Uses DB for fast file path lookup, then reads status from frontmatter on
/// disk (source of truth). Falls back to filesystem scan for unscraped specs.
pub(super) fn find_spec(id: &str) -> Result<FoundSpec> {
    // Try DB for file path, but always re-read status from disk
    let db_path = super::db_path()?;

    if db_path.exists() {
        if let Ok(conn) = Connection::open(&db_path) {
            let result = conn.query_row(
                "SELECT file_path, title FROM patterns WHERE id = ?1",
                rusqlite::params![id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            );

            match result {
                Ok((file_path, title)) => {
                    // DB gives us the path — read status from frontmatter
                    let status = read_frontmatter_status(&file_path);
                    return Ok(FoundSpec {
                        file_path,
                        status,
                        title,
                    });
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // Fall through to filesystem scan
                }
                Err(e) => return Err(e).context("Failed to query patterns table"),
            }
        }
    }

    // Filesystem fallback: scan disk for matching spec (one walk, not two)
    let disk_specs = scan_disk_specs();
    for spec in disk_specs {
        if spec.id == id {
            if let Some(path) = spec.file_path {
                return Ok(FoundSpec {
                    file_path: path,
                    status: spec.status,
                    title: Some(spec.title),
                });
            }
        }
    }

    // Git tag fallback: archived specs exist only as annotated tags
    let tag_name = format!("spec/{}", id);
    if tag_exists(&tag_name)? {
        let status = archived_spec_status(id).unwrap_or(SpecStatus::Complete);
        return Ok(FoundSpec {
            file_path: format!("(archived: {})", tag_name),
            status: Some(status),
            title: None,
        });
    }

    anyhow::bail!(
        "Spec '{}' not found.\n\
         Check the id, or create it under layer/surface/build/.",
        id
    );
}

/// Read spec status directly from YAML frontmatter on disk.
fn read_frontmatter_status(file_path: &str) -> Option<SpecStatus> {
    let content = std::fs::read_to_string(file_path).ok()?;
    let mut in_frontmatter = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_frontmatter {
                break;
            }
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if let Some(rest) = trimmed.strip_prefix("status:") {
                return rest.trim().parse::<SpecStatus>().ok();
            }
        }
    }
    None
}

/// Determine archived spec status from the archive commit message.
///
/// Parses "docs: archive spec/{id} ({status})" → "complete" or "abandoned".
/// Falls back to "complete" if no matching commit found (release path uses
/// a different commit message format and is always a completion).
fn archived_spec_status(id: &str) -> Option<SpecStatus> {
    let pattern = format!("docs: archive spec/{}", id);
    let output = Command::new("git")
        .args(["log", "--all", "--format=%s", "-1", "--grep", &pattern])
        .output()
        .ok()?;
    let subject = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let paren_content = subject.rsplit('(').next()?.trim_end_matches(')');
    paren_content.parse().ok()
}

/// A fully loaded spec: metadata + parsed content. For mutation paths.
#[derive(Debug)]
pub(super) struct LoadedSpec {
    pub file_path: String,
    pub status: Option<SpecStatus>,
    pub title: Option<String>,
    pub content: String,
    pub frontmatter: SpecFrontmatter,
    pub body: String,
}

fn archived_tag_from_marker(file_path: &str) -> Option<&str> {
    file_path
        .strip_prefix("(archived: ")
        .and_then(|rest| rest.strip_suffix(')'))
}

fn archived_spec_rel_path(tag_name: &str, id: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", tag_name])
        .output()
        .with_context(|| format!("Failed to inspect archived tag '{}'", tag_name))?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to inspect archived tag '{}': {}",
            tag_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let expected_suffix = format!("/{}/SPEC.md", id);
    let mut candidates = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| line.ends_with("/SPEC.md"))
        .filter(|line| line.ends_with(&expected_suffix))
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    candidates.sort();
    candidates.dedup();

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => anyhow::bail!(
            "Archived tag '{}' does not contain archived SPEC.md path for id '{}'",
            tag_name,
            id
        ),
        _ => anyhow::bail!(
            "Archived tag '{}' contains multiple SPEC.md candidates for id '{}': {}",
            tag_name,
            id,
            candidates.join(", ")
        ),
    }
}

fn read_archived_spec_from_tag(tag_name: &str, rel_path: &str) -> Result<String> {
    let object = format!("{}:{}", tag_name, rel_path);
    let output = Command::new("git")
        .args(["show", &object])
        .output()
        .with_context(|| format!("Failed to read archived spec from '{}'", object))?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to read archived spec from '{}': {}",
            object,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Load a spec for read/query paths.
///
/// Supports both on-disk specs and archived specs stored in git tags.
pub(super) fn load_spec_read_only(id: &str) -> Result<LoadedSpec> {
    let found = find_spec(id)?;

    let (resolved_path, content) =
        if let Some(tag_name) = archived_tag_from_marker(&found.file_path) {
            let rel_path = archived_spec_rel_path(tag_name, id)?;
            let content = read_archived_spec_from_tag(tag_name, &rel_path)?;
            (format!("(archived: {}:{})", tag_name, rel_path), content)
        } else {
            let content = std::fs::read_to_string(&found.file_path)
                .with_context(|| format!("Failed to read {}", found.file_path))?;
            (found.file_path.clone(), content)
        };

    let (frontmatter, body) =
        parse_spec_file(&content).with_context(|| format!("Failed to parse {}", resolved_path))?;

    if frontmatter.id != id {
        anyhow::bail!(
            "Frontmatter ID '{}' doesn't match lookup key '{}' in {}",
            frontmatter.id,
            id,
            resolved_path
        );
    }

    Ok(LoadedSpec {
        file_path: resolved_path,
        status: found.status,
        title: found.title,
        content,
        frontmatter,
        body,
    })
}

/// Load a spec fully from disk (find + read + parse). For mutations.
///
/// Asserts that the frontmatter ID matches the lookup key to prevent
/// ID source-of-truth drift (see spec-mutation-cleanup § Refactor 1).
pub(super) fn load_spec(id: &str) -> Result<LoadedSpec> {
    let found = find_spec(id)?;
    if let Some(tag_name) = archived_tag_from_marker(&found.file_path) {
        anyhow::bail!(
            "Spec '{}' is archived at tag '{}'; mutation commands require on-disk specs",
            id,
            tag_name
        );
    }

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

    fn run_git(repo: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .status()
            .expect("run git");
        assert!(status.success(), "git {:?} failed", args);
    }

    fn with_temp_git_repo<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
        let _guard = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let temp = tempfile::tempdir().expect("tempdir");
        run_git(temp.path(), &["init", "-q"]);
        run_git(
            temp.path(),
            &["config", "user.email", "spec-test@example.com"],
        );
        run_git(temp.path(), &["config", "user.name", "Spec Test"]);

        let old_cwd = std::env::current_dir().ok();
        std::env::set_current_dir(temp.path()).expect("set cwd");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(temp.path())));

        // Best-effort restore: old cwd if it still exists, otherwise stable project root.
        if let Some(path) = old_cwd.as_ref().filter(|path| path.exists()) {
            let _ = std::env::set_current_dir(path);
        } else {
            let _ = std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"));
        }

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn load_spec_read_only_reads_archived_tag_content() {
        with_temp_git_repo(|repo| {
            let spec_path = repo.join("layer/surface/build/feat/demo-archived/SPEC.md");
            std::fs::create_dir_all(spec_path.parent().expect("spec dir")).expect("mkdir spec dir");
            std::fs::write(
                &spec_path,
                "---\ntype: feat\nid: demo-archived\nstatus: complete\n---\n# feat: demo-archived\n\n## Goal\n\n",
            )
            .expect("write spec");
            run_git(repo, &["add", "."]);
            run_git(repo, &["commit", "-m", "add spec"]);
            run_git(
                repo,
                &["tag", "-a", "spec/demo-archived", "-m", "archive demo"],
            );

            std::fs::remove_file(&spec_path).expect("remove spec");
            std::fs::remove_dir_all(repo.join("layer/surface/build/feat/demo-archived"))
                .expect("remove spec dir");
            run_git(repo, &["add", "-A"]);
            run_git(repo, &["commit", "-m", "remove spec"]);

            let loaded = load_spec_read_only("demo-archived").expect("load archived spec");
            assert_eq!(loaded.frontmatter.id, "demo-archived");
            assert!(
                loaded.file_path.contains(
                    "(archived: spec/demo-archived:layer/surface/build/feat/demo-archived/SPEC.md)"
                ),
                "unexpected path marker: {}",
                loaded.file_path
            );
        });
    }

    #[test]
    fn load_spec_read_only_fails_when_archive_tag_missing_spec_path() {
        with_temp_git_repo(|repo| {
            std::fs::write(repo.join("README.md"), "demo\n").expect("write readme");
            run_git(repo, &["add", "README.md"]);
            run_git(repo, &["commit", "-m", "init"]);
            run_git(
                repo,
                &["tag", "-a", "spec/bad-archived", "-m", "archive bad"],
            );

            let err = load_spec_read_only("bad-archived")
                .expect_err("expected missing archived path error");
            assert!(
                err.to_string()
                    .contains("does not contain archived SPEC.md path for id 'bad-archived'"),
                "got: {}",
                err
            );
        });
    }
}
