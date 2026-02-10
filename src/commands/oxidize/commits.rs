//! Commit-based training pair generator for ref repos
//!
//! When a repo has no sessions (like ref repos), use commit messages as
//! training signal for semantic projection. Commit messages are natural
//! language descriptions of code changes - free (NL, code) pairs.
//!
//! Strategy:
//! - Anchor: commit message (natural language)
//! - Positive: function from file touched by commit
//! - Negative: function from file NOT touched by commit

use super::pairs::TrainingPair;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashSet;

/// Generate training pairs from commits
///
/// Filters to conventional commits with meaningful messages, then creates
/// triplets using functions from touched vs untouched files.
/// Phase 5c: uses ALL viable commits instead of sampling a subset.
pub fn generate_commit_pairs(db_path: &str) -> Result<Vec<TrainingPair>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Get all filtered commits (conventional format, meaningful length)
    let commits = query_filtered_commits(&conn)?;

    if commits.is_empty() {
        anyhow::bail!(
            "No suitable commits found for training (need conventional commits with length > 30)"
        );
    }

    println!("   Found {} filtered commits for training", commits.len());

    // Get all functions for positive/negative sampling
    let all_functions = query_all_functions(&conn)?;

    if all_functions.is_empty() {
        anyhow::bail!("No functions found in database - run scrape first");
    }

    println!("   Found {} functions for sampling", all_functions.len());

    // Build file -> functions index for efficient lookup
    // Normalize paths by stripping leading "./" for consistent matching
    let mut file_to_functions: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (file, desc) in &all_functions {
        let normalized = normalize_path(file);
        file_to_functions
            .entry(normalized)
            .or_default()
            .push(desc.clone());
    }

    let mut all_files: Vec<&String> = file_to_functions.keys().collect();
    all_files.sort();

    // Generate pairs from ALL viable commits (Phase 5c: no sampling limit)
    let mut pairs = Vec::new();
    let mut rng = fastrand::Rng::with_seed(42);

    for (sha, message, moment_type) in &commits {
        // Get files touched by this commit (normalized)
        let touched_files: Vec<String> = query_commit_files(&conn, sha)?
            .into_iter()
            .map(|f| normalize_path(&f))
            .collect();
        if touched_files.is_empty() {
            continue;
        }

        // Find a touched file that has functions
        let positive_file = touched_files
            .iter()
            .find(|f| file_to_functions.contains_key(f.as_str()));

        let positive_file = match positive_file {
            Some(f) => f,
            None => continue, // No functions in touched files
        };

        // Get a function from touched file
        let positive_functions = &file_to_functions[positive_file];
        let positive_idx = rng.usize(..positive_functions.len());
        let positive = positive_functions[positive_idx].clone();

        // Find an untouched file with functions
        let touched_set: HashSet<&str> = touched_files.iter().map(|s| s.as_str()).collect();
        let untouched_files: Vec<_> = all_files
            .iter()
            .filter(|f| !touched_set.contains(f.as_str()))
            .collect();

        if untouched_files.is_empty() {
            continue;
        }

        let negative_file_idx = rng.usize(..untouched_files.len());
        let negative_file = untouched_files[negative_file_idx];
        let negative_functions = &file_to_functions[*negative_file];
        let negative_idx = rng.usize(..negative_functions.len());
        let negative = negative_functions[negative_idx].clone();

        // Weight by moment type (for future weighted training)
        let _weight = moment_to_weight(moment_type.as_deref());

        pairs.push(TrainingPair {
            anchor: message.clone(),
            positive,
            negative,
        });
    }

    if pairs.is_empty() {
        anyhow::bail!("Could not generate any training pairs from commits");
    }

    Ok(pairs)
}

/// Query all filtered commits (conventional format, meaningful length)
/// Phase 5c: no LIMIT — returns all viable commits for full pair generation.
fn query_filtered_commits(conn: &Connection) -> Result<Vec<(String, String, Option<String>)>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT c.sha, c.message, m.moment_type
        FROM commits c
        LEFT JOIN moments m ON c.sha = m.sha
        WHERE (
            c.message LIKE 'feat%'
            OR c.message LIKE 'fix%'
            OR c.message LIKE 'refactor%'
            OR c.message LIKE 'perf%'
            OR c.message LIKE 'docs%'
            OR c.message LIKE 'test%'
        )
        AND length(c.message) > 30
        AND c.message NOT LIKE '%wip%'
        AND c.message NOT LIKE 'Merge %'
        ORDER BY c.timestamp DESC
        "#,
    )?;

    let mut commits = Vec::new();
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let sha: String = row.get(0)?;
        let message: String = row.get(1)?;
        let moment_type: Option<String> = row.get(2)?;
        commits.push((sha, message, moment_type));
    }

    Ok(commits)
}

/// Query files touched by a commit
fn query_commit_files(conn: &Connection, sha: &str) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT file_path FROM commit_files WHERE sha = ? ORDER BY file_path")?;
    let mut files = Vec::new();
    let mut rows = stmt.query([sha])?;

    while let Some(row) = rows.next()? {
        let file_path: String = row.get(0)?;
        files.push(file_path);
    }

    Ok(files)
}

/// Query all functions with embeddable descriptions
fn query_all_functions(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT file, name, parameters, return_type, is_public, is_async
         FROM function_facts
         WHERE name != ''
         ORDER BY file, name",
    )?;

    let mut functions = Vec::new();
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let file: String = row.get(0)?;
        let name: String = row.get(1)?;
        let params: Option<String> = row.get(2)?;
        let return_type: Option<String> = row.get(3)?;
        let is_public: bool = row.get(4)?;
        let is_async: bool = row.get(5)?;

        // Create embeddable text (same format as semantic index)
        let mut desc = format!("Function `{}` in file `{}`", name, file);
        if is_public {
            desc.push_str(", public");
        }
        if is_async {
            desc.push_str(", async");
        }
        if let Some(p) = params {
            if !p.is_empty() {
                desc.push_str(&format!(", parameters: {}", p));
            }
        }
        if let Some(rt) = return_type {
            if !rt.is_empty() {
                desc.push_str(&format!(", returns: {}", rt));
            }
        }

        functions.push((file, desc));
    }

    Ok(functions)
}

/// Normalize file path by stripping leading "./"
fn normalize_path(path: &str) -> String {
    path.strip_prefix("./").unwrap_or(path).to_string()
}

/// Calculate weight multiplier based on moment type
fn moment_to_weight(moment_type: Option<&str>) -> f32 {
    match moment_type {
        Some("breaking") => 3.0,
        Some("big_bang") => 2.0,
        Some("migration") => 1.5,
        Some("rewrite") => 1.2,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_db() -> NamedTempFile {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create tables
        conn.execute_batch(
            r#"
            CREATE TABLE commits (
                sha TEXT PRIMARY KEY,
                message TEXT,
                author_name TEXT,
                author_email TEXT,
                timestamp TEXT,
                branch TEXT
            );

            CREATE TABLE commit_files (
                sha TEXT,
                file_path TEXT,
                change_type TEXT,
                PRIMARY KEY (sha, file_path)
            );

            CREATE TABLE moments (
                sha TEXT PRIMARY KEY,
                moment_type TEXT
            );

            CREATE TABLE function_facts (
                file TEXT,
                name TEXT,
                parameters TEXT,
                return_type TEXT,
                is_public BOOLEAN,
                is_async BOOLEAN,
                PRIMARY KEY (file, name)
            );

            -- Insert test data
            INSERT INTO commits VALUES
                ('abc123', 'feat: add user authentication flow', 'dev', 'dev@test.com', '2025-01-01', 'main'),
                ('def456', 'fix: handle null pointer in parser', 'dev', 'dev@test.com', '2025-01-02', 'main');

            INSERT INTO commit_files VALUES
                ('abc123', 'src/auth.rs', 'A'),
                ('def456', 'src/parser.rs', 'M');

            INSERT INTO function_facts VALUES
                ('src/auth.rs', 'authenticate', 'user, password', 'Result', 1, 0),
                ('src/parser.rs', 'parse', 'input', 'Option', 1, 0),
                ('src/other.rs', 'helper', '', 'void', 0, 0);
            "#,
        )
        .unwrap();

        temp_file
    }

    #[test]
    fn test_generate_commit_pairs() {
        let temp_db = create_test_db();
        let pairs = generate_commit_pairs(temp_db.path().to_str().unwrap()).unwrap();

        assert!(!pairs.is_empty());

        for pair in &pairs {
            assert!(!pair.anchor.is_empty());
            assert!(!pair.positive.is_empty());
            assert!(!pair.negative.is_empty());
            // Anchor should be commit message (NL)
            assert!(pair.anchor.starts_with("feat:") || pair.anchor.starts_with("fix:"));
            // Positive/negative should be function descriptions
            assert!(pair.positive.contains("Function"));
            assert!(pair.negative.contains("Function"));
        }
    }
}
