//! Git repository validation and branch management

use super::operations::{
    branch_exists, checkout_new_branch, commits_behind, current_branch, default_branch,
    has_commits, is_clean, rename_current_branch, status_count,
};
use super::timestamp;
use crate::git::operations::{branch_rename, is_git_repo};
use anyhow::Result;

/// Ensure we're on the patina branch, handling all edge cases
pub fn ensure_patina_branch(force: bool) -> Result<()> {
    if !is_git_repo()? {
        anyhow::bail!(
            "⚠️  Not a git repository\n\
             \n\
             Initialize git first:\n\
             git init\n\
             git add .\n\
             git commit -m \"Initial commit\""
        );
    }

    // === EDGE CASE 0: Empty repository (no commits yet) ===
    // In an empty repo, HEAD points to a phantom branch (usually main/master)
    // that doesn't really exist. We can simply rename it to 'patina'.
    if !has_commits()? {
        let current = current_branch()?;
        if current == "patina" {
            println!("ℹ️  Already on 'patina' branch (empty repository)");
            return Ok(());
        }
        rename_current_branch("patina")?;
        println!("✓ Renamed '{}' → 'patina' (empty repository)", current);
        return Ok(());
    }

    let current = current_branch()?;
    let has_patina = branch_exists("patina")?;
    let clean = is_clean()?;
    let default = default_branch()?;

    // === EDGE CASE 1: Dirty working tree ===
    if !clean && !force {
        anyhow::bail!(
            "⚠️  Uncommitted changes detected\n\
             \n\
             Your working tree has uncommitted changes.\n\
             Patina needs a clean state to proceed safely.\n\
             \n\
             Options:\n\
             1. Commit your changes: git commit -am \"WIP\"\n\
             2. Stash your changes: git stash\n\
             3. Force anyway: patina init . --llm=claude --force\n\
             \n\
             Current branch: {}\n\
             Status: {} files modified",
            current,
            status_count()?
        );
    }

    // === EDGE CASE 2: patina branch already exists ===
    if has_patina {
        if current == "patina" {
            // Already on patina
            if !force {
                println!("ℹ️  Already on 'patina' branch");

                // But is it up to date with main?
                let behind = commits_behind("patina", &default)?;
                if behind > 0 {
                    anyhow::bail!(
                        "⚠️  Branch 'patina' is {} commits behind '{}'\n\
                         \n\
                         Your patina branch is outdated. Options:\n\
                         1. Delete and recreate: git branch -D patina && patina init .\n\
                         2. Rebase: git rebase {}\n\
                         3. Force re-init: patina init . --llm=claude --force\n\
                         \n\
                         --force will backup current patina → patina-backup-{{timestamp}}",
                        behind,
                        default,
                        default
                    );
                }
            } else {
                // --force: backup existing patina branch
                let backup_name = format!("patina-backup-{}", timestamp());
                branch_rename("patina", &backup_name)?;
                println!("✓ Backed up patina → {}", backup_name);

                // Create fresh patina from default branch
                checkout_new_branch("patina", &default)?;
            }
        } else {
            // patina exists but we're not on it
            if !force {
                anyhow::bail!(
                    "⚠️  Branch 'patina' already exists (you're on '{}')\n\
                     \n\
                     A patina branch already exists. Options:\n\
                     1. Switch to it: git checkout patina\n\
                     2. Delete and recreate: git branch -D patina && patina init .\n\
                     3. Force re-init: patina init . --llm=claude --force\n\
                     \n\
                     --force will backup patina → patina-backup-{{timestamp}}",
                    current
                );
            } else {
                // --force: backup existing, create new
                let backup_name = format!("patina-backup-{}", timestamp());
                branch_rename("patina", &backup_name)?;
                println!("✓ Backed up patina → {}", backup_name);
                checkout_new_branch("patina", &default)?;
            }
        }
    } else {
        // === EDGE CASE 3: No patina branch, on some other branch ===
        if current == default {
            // On main/master - just create patina
            checkout_new_branch("patina", &default)?;
            println!("✓ Created branch 'patina' from '{}'", default);
        } else {
            // On some random branch
            if !force {
                anyhow::bail!(
                    "⚠️  Currently on branch '{}' (not '{}' or 'patina')\n\
                     \n\
                     Patina creates a 'patina' branch from your default branch.\n\
                     You're on a different branch. Options:\n\
                     1. Switch to {}: git checkout {}\n\
                     2. Force from current: patina init . --llm=claude --force\n\
                     \n\
                     --force will create patina from current branch state",
                    current,
                    default,
                    default,
                    default
                );
            } else {
                // --force: create patina from current state
                checkout_new_branch("patina", &current)?;
                println!("⚠️  Created 'patina' from '{}' (non-standard)", current);
            }
        }
    }

    Ok(())
}
