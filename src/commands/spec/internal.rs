use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

/// Archive a completed spec: create spec/<id> tag, remove file, update build.md, commit
pub fn archive_spec(id: &str, dry_run: bool) -> Result<()> {
    // 1. Find spec in patterns table by id
    let (file_path, status, title) = find_spec(id)?;

    // 2. Validate status is complete
    if status != "complete" {
        anyhow::bail!(
            "Spec '{}' has status '{}', expected 'complete'\n\
             Only completed specs can be archived.",
            id,
            status
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
            file_path
        );
    }

    // Resolve spec directory (parent of SPEC.md)
    let spec_file = Path::new(&file_path);
    let spec_dir = spec_file
        .parent()
        .filter(|p| p.file_name().is_some())
        .map(|p| p.to_path_buf());

    if dry_run {
        println!("Dry run — would perform these changes:\n");
        println!("  Tag:    {} (preserves spec content)", tag_name);
        if let Some(dir) = &spec_dir {
            println!("  Remove: {}/", dir.display());
        } else {
            println!("  Remove: {}", file_path);
        }
        println!(
            "  Update: layer/core/build.md (add to Archived section)"
        );
        println!(
            "  Commit: docs: archive {} (complete)",
            tag_name
        );
        println!(
            "\nRecover with: git show {}:{}",
            tag_name, file_path
        );
        return Ok(());
    }

    // 4. Check working tree is clean (only for actual execution, not dry-run)
    if !is_tree_clean()? {
        anyhow::bail!(
            "Working tree has uncommitted changes.\n\
             Commit or stash your changes before archiving."
        );
    }

    // 5. Create annotated tag
    println!("Creating tag: {}", tag_name);
    let desc = title.as_deref().unwrap_or(id);
    let output = Command::new("git")
        .args(["tag", "-a", &tag_name, "-m", &format!("Archived spec: {}", desc)])
        .output()
        .context("Failed to create git tag")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git tag failed: {}", stderr);
    }

    // 6. Remove spec file/directory from tree
    let remove_target = if let Some(dir) = &spec_dir {
        // Check if directory contains only SPEC.md (or SPEC.md + nothing else interesting)
        dir.to_str().unwrap_or(&file_path).to_string()
    } else {
        file_path.clone()
    };
    println!("Removing: {}", remove_target);
    let output = Command::new("git")
        .args(["rm", "-r", &remove_target])
        .output()
        .context("Failed to remove spec from tree")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git rm failed: {}", stderr);
    }

    // 7. Update build.md Archives section
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let archive_entry = format!("- `{}` - {} ({})", tag_name, desc, today);
    if let Err(e) = update_build_md(&archive_entry) {
        eprintln!("Warning: failed to update build.md: {}", e);
        eprintln!("  You may want to add this entry manually:");
        eprintln!("  {}", archive_entry);
    }

    // 8. Commit
    let commit_msg = format!(
        "docs: archive {} (complete)\n\nSpec preserved via git tag: {}\nRecover with: git show {}:{}",
        tag_name, tag_name, tag_name, file_path
    );
    println!("Committing archive");

    // Stage build.md too
    let _ = Command::new("git")
        .args(["add", "layer/core/build.md"])
        .output();

    let output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("Failed to commit archive")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git commit failed: {}", stderr);
    }

    println!(
        "\n✓ Archived: {}\n  Tag: {}\n  Recover: git show {}:{}",
        id, tag_name, tag_name, file_path
    );

    Ok(())
}

/// Find a spec by its frontmatter id in the patterns table
fn find_spec(id: &str) -> Result<(String, String, Option<String>)> {
    let db_path = Path::new(".patina/local/data/patina.db");
    if !db_path.exists() {
        anyhow::bail!(
            "Knowledge database not found. Run 'patina scrape' first."
        );
    }

    let conn = Connection::open(db_path).context("Failed to open database")?;

    let result = conn.query_row(
        "SELECT file_path, status, title FROM patterns WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        },
    );

    match result {
        Ok(row) => Ok(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            anyhow::bail!(
                "Spec '{}' not found in patterns table.\n\
                 Run 'patina scrape' to index specs, or check the id.",
                id
            );
        }
        Err(e) => Err(e).context("Failed to query patterns table"),
    }
}

/// Check if a git tag exists
fn tag_exists(tag: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["tag", "-l", tag])
        .output()
        .context("Failed to list git tags")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}

/// Check if working tree is clean (no uncommitted tracked changes)
fn is_tree_clean() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-uno"])
        .output()
        .context("Failed to check git status")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().is_empty())
}

/// Update build.md to add an entry to the "Archived (git tags)" section
fn update_build_md(entry: &str) -> Result<()> {
    let build_path = "layer/core/build.md";
    let content = std::fs::read_to_string(build_path)
        .with_context(|| format!("Failed to read {}", build_path))?;

    // Find the "Full list:" line that ends the archived section and insert before it
    let marker = "Full list: `git tag -l 'spec/*'`";
    if let Some(pos) = content.find(marker) {
        let new_content = format!(
            "{}{}\n{}",
            &content[..pos],
            entry,
            &content[pos..]
        );
        std::fs::write(build_path, &new_content)
            .with_context(|| format!("Failed to write {}", build_path))?;

        // Also update the tag count
        update_tag_count(&new_content, build_path)?;

        Ok(())
    } else {
        anyhow::bail!(
            "Could not find '{}' marker in {}",
            marker,
            build_path
        );
    }
}

/// Update the "(N archived specs)" count in build.md
fn update_tag_count(content: &str, path: &str) -> Result<()> {
    // Match pattern like "(46 archived specs)"
    if let Some(start) = content.find("archived specs)") {
        // Walk backwards to find the opening paren and number
        let prefix = &content[..start];
        if let Some(paren_pos) = prefix.rfind('(') {
            let num_str = prefix[paren_pos + 1..].trim();
            if let Ok(count) = num_str.parse::<u32>() {
                let old = format!("({} archived specs)", count);
                let new = format!("({} archived specs)", count + 1);
                let updated = content.replace(&old, &new);
                std::fs::write(path, updated)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tag_name_format() {
        let id = "session-092-hardening";
        let tag = format!("spec/{}", id);
        assert_eq!(tag, "spec/session-092-hardening");
    }
}
