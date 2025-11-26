//! Temporal training pair generator
//!
//! Generate (anchor, positive, negative) triplets from co_changes for contrastive learning.
//! Files that change together in commits are considered temporally related.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

use super::pairs::TrainingPair;

/// Minimum co-change count to consider files as related
const MIN_COCHANGE_COUNT: i64 = 2;

/// Generate training pairs where files that co-change are similar
///
/// Strategy:
/// - Anchor: random file from co_changes
/// - Positive: file that frequently changes with anchor
/// - Negative: file that rarely/never changes with anchor
pub fn generate_temporal_pairs(db_path: &str, num_pairs: usize) -> Result<Vec<TrainingPair>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Load co-change relationships (file_a -> set of related files)
    let mut cochanges: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_files: HashSet<String> = HashSet::new();

    let mut stmt = conn.prepare(
        "SELECT file_a, file_b, count
         FROM co_changes
         WHERE count >= ?
         ORDER BY count DESC",
    )?;

    let mut rows = stmt.query([MIN_COCHANGE_COUNT])?;
    while let Some(row) = rows.next()? {
        let file_a: String = row.get(0)?;
        let file_b: String = row.get(1)?;

        // Track all files
        all_files.insert(file_a.clone());
        all_files.insert(file_b.clone());

        // Bidirectional relationship
        cochanges
            .entry(file_a.clone())
            .or_default()
            .insert(file_b.clone());
        cochanges
            .entry(file_b.clone())
            .or_default()
            .insert(file_a.clone());
    }

    // Filter to files with at least one co-change partner
    let files_with_cochanges: Vec<_> = cochanges
        .iter()
        .filter(|(_, partners)| !partners.is_empty())
        .collect();

    if files_with_cochanges.is_empty() {
        anyhow::bail!("No files with co-change relationships found");
    }

    // Convert to vec for random access
    let all_files_vec: Vec<_> = all_files.iter().collect();

    println!(
        "  Found {} files with {} co-change relationships",
        files_with_cochanges.len(),
        cochanges.values().map(|v| v.len()).sum::<usize>() / 2
    );

    // Generate pairs
    let mut pairs = Vec::new();
    let mut rng = fastrand::Rng::new();

    for _ in 0..num_pairs {
        // Pick random file with co-changes as anchor
        let anchor_idx = rng.usize(..files_with_cochanges.len());
        let (anchor_file, anchor_partners) = files_with_cochanges[anchor_idx];

        // Pick positive from co-change partners
        let partners_vec: Vec<_> = anchor_partners.iter().collect();
        let positive_idx = rng.usize(..partners_vec.len());
        let positive_file = partners_vec[positive_idx];

        // Pick negative from files that don't co-change with anchor
        let mut negative_file = all_files_vec[rng.usize(..all_files_vec.len())];
        let mut attempts = 0;
        while (anchor_partners.contains(negative_file) || *negative_file == *anchor_file)
            && attempts < 100
        {
            negative_file = all_files_vec[rng.usize(..all_files_vec.len())];
            attempts += 1;
        }

        // Convert file paths to descriptive text for embedding
        let anchor = file_to_text(anchor_file);
        let positive = file_to_text(positive_file);
        let negative = file_to_text(negative_file);

        pairs.push(TrainingPair {
            anchor,
            positive,
            negative,
        });
    }

    Ok(pairs)
}

/// Convert file path to text suitable for embedding
///
/// Creates a description that E5 can meaningfully embed:
/// "src/commands/oxidize/mod.rs" -> "File: src/commands/oxidize/mod.rs (Rust module)"
pub fn file_to_text(path: &str) -> String {
    let extension = path.rsplit('.').next().unwrap_or("");
    let file_type = match extension {
        "rs" => "Rust source",
        "ts" => "TypeScript source",
        "js" => "JavaScript source",
        "py" => "Python source",
        "go" => "Go source",
        "md" => "Markdown document",
        "yaml" | "yml" => "YAML config",
        "toml" => "TOML config",
        "json" => "JSON data",
        "sh" => "Shell script",
        _ => "file",
    };

    format!("File: {} ({})", path, file_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_db() -> NamedTempFile {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create co_changes table
        conn.execute(
            "CREATE TABLE co_changes (
                file_a TEXT,
                file_b TEXT,
                count INTEGER,
                PRIMARY KEY (file_a, file_b)
            )",
            [],
        )
        .unwrap();

        // Insert test co-changes
        conn.execute(
            "INSERT INTO co_changes (file_a, file_b, count) VALUES
             ('src/main.rs', 'src/lib.rs', 10),
             ('src/main.rs', 'src/commands/mod.rs', 8),
             ('src/lib.rs', 'src/commands/mod.rs', 5),
             ('src/utils.rs', 'src/helpers.rs', 3),
             ('tests/test.rs', 'src/main.rs', 2)",
            [],
        )
        .unwrap();

        temp_file
    }

    #[test]
    fn test_generate_temporal_pairs() {
        let temp_db = create_test_db();
        let pairs = generate_temporal_pairs(temp_db.path().to_str().unwrap(), 10).unwrap();

        assert_eq!(pairs.len(), 10);

        // Verify structure
        for pair in &pairs {
            assert!(!pair.anchor.is_empty());
            assert!(!pair.positive.is_empty());
            assert!(!pair.negative.is_empty());
            assert!(pair.anchor.starts_with("File: "));
        }
    }

    #[test]
    fn test_file_to_text() {
        assert_eq!(
            file_to_text("src/main.rs"),
            "File: src/main.rs (Rust source)"
        );
        assert_eq!(
            file_to_text("package.json"),
            "File: package.json (JSON data)"
        );
        assert_eq!(
            file_to_text("README.md"),
            "File: README.md (Markdown document)"
        );
    }

    #[test]
    fn test_empty_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute(
            "CREATE TABLE co_changes (
                file_a TEXT,
                file_b TEXT,
                count INTEGER,
                PRIMARY KEY (file_a, file_b)
            )",
            [],
        )
        .unwrap();

        let result = generate_temporal_pairs(temp_file.path().to_str().unwrap(), 5);
        assert!(result.is_err());
    }
}
