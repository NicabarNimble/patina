//! Dependency training pair generator
//!
//! Generate (anchor, positive, negative) triplets from call_graph for contrastive learning.
//! Functions that call each other are considered dependency-related.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

use super::pairs::TrainingPair;

/// Minimum call count to consider functions as related (for filtering noise)
const MIN_CALL_COUNT: i64 = 1;

/// Generate training pairs where functions that call each other are similar
///
/// Strategy:
/// - Anchor: random function from call_graph
/// - Positive: function it calls OR function that calls it
/// - Negative: unrelated function (no call relationship)
pub fn generate_dependency_pairs(db_path: &str, num_pairs: usize) -> Result<Vec<TrainingPair>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Load call relationships (function -> set of related functions)
    // Both directions: caller->callee and callee->caller
    let mut call_relations: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_functions: HashSet<String> = HashSet::new();

    let mut stmt = conn.prepare(
        "SELECT caller, callee, COUNT(*) as cnt
         FROM call_graph
         GROUP BY caller, callee
         HAVING cnt >= ?",
    )?;

    let mut rows = stmt.query([MIN_CALL_COUNT])?;
    while let Some(row) = rows.next()? {
        let caller: String = row.get(0)?;
        let callee: String = row.get(1)?;

        // Track all functions
        all_functions.insert(caller.clone());
        all_functions.insert(callee.clone());

        // Bidirectional relationship (caller knows callee, callee knows caller)
        call_relations
            .entry(caller.clone())
            .or_default()
            .insert(callee.clone());
        call_relations
            .entry(callee.clone())
            .or_default()
            .insert(caller.clone());
    }

    // Filter to functions with at least one call relationship
    let functions_with_calls: Vec<_> = call_relations
        .iter()
        .filter(|(_, partners)| !partners.is_empty())
        .collect();

    if functions_with_calls.is_empty() {
        anyhow::bail!("No functions with call relationships found");
    }

    // Convert to vec for random access
    let all_functions_vec: Vec<_> = all_functions.iter().collect();

    println!(
        "  Found {} functions with {} call relationships",
        functions_with_calls.len(),
        call_relations.values().map(|v| v.len()).sum::<usize>() / 2
    );

    // Generate pairs
    let mut pairs = Vec::new();
    let mut rng = fastrand::Rng::new();

    for _ in 0..num_pairs {
        // Pick random function with call relationships as anchor
        let anchor_idx = rng.usize(..functions_with_calls.len());
        let (anchor_func, anchor_partners) = functions_with_calls[anchor_idx];

        // Pick positive from call partners (functions it calls or that call it)
        let partners_vec: Vec<_> = anchor_partners.iter().collect();
        let positive_idx = rng.usize(..partners_vec.len());
        let positive_func = partners_vec[positive_idx];

        // Pick negative from functions that have no call relationship with anchor
        let mut negative_func = all_functions_vec[rng.usize(..all_functions_vec.len())];
        let mut attempts = 0;
        while (anchor_partners.contains(negative_func) || negative_func == anchor_func)
            && attempts < 100
        {
            negative_func = all_functions_vec[rng.usize(..all_functions_vec.len())];
            attempts += 1;
        }

        // Convert function names to descriptive text for embedding
        let anchor = function_to_text(anchor_func);
        let positive = function_to_text(positive_func);
        let negative = function_to_text(negative_func);

        pairs.push(TrainingPair {
            anchor,
            positive,
            negative,
        });
    }

    Ok(pairs)
}

/// Maximum length for function names (E5 has 512 token limit, ~4 chars/token)
const MAX_FUNCTION_NAME_LEN: usize = 200;

/// Convert function name to text suitable for embedding
///
/// Creates a description that E5 can meaningfully embed:
/// "commands::init::execute" -> "Function: commands::init::execute (Rust function)"
pub fn function_to_text(name: &str) -> String {
    // Truncate very long names (some scraped "functions" are actually code blocks)
    let truncated = if name.len() > MAX_FUNCTION_NAME_LEN {
        &name[..MAX_FUNCTION_NAME_LEN]
    } else {
        name
    };

    // Detect language/type from naming patterns
    let func_type = if truncated.contains("::") {
        "Rust function"
    } else if truncated.contains(".") {
        "method call"
    } else if truncated.starts_with(|c: char| c.is_uppercase()) {
        "type or constructor"
    } else {
        "function"
    };

    format!("Function: {} ({})", truncated, func_type)
}

/// Query all unique functions for building the dependency index
pub fn query_function_events(conn: &Connection) -> Result<Vec<(i64, String)>> {
    // Get unique functions from call_graph (both callers and callees)
    let mut stmt = conn.prepare(
        "SELECT DISTINCT caller FROM call_graph
         UNION
         SELECT DISTINCT callee FROM call_graph
         ORDER BY 1",
    )?;

    let mut events = Vec::new();
    let mut rows = stmt.query([])?;
    let mut idx: i64 = 0;
    while let Some(row) = rows.next()? {
        let func_name: String = row.get(0)?;
        let text = function_to_text(&func_name);
        events.push((idx, text));
        idx += 1;
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_db() -> NamedTempFile {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create call_graph table
        conn.execute(
            "CREATE TABLE call_graph (
                caller TEXT NOT NULL,
                callee TEXT NOT NULL,
                file TEXT NOT NULL,
                call_type TEXT DEFAULT 'direct',
                line_number INTEGER,
                PRIMARY KEY (caller, callee, file, line_number)
            )",
            [],
        )
        .unwrap();

        // Insert test call relationships
        conn.execute(
            "INSERT INTO call_graph (caller, callee, file, call_type, line_number) VALUES
             ('main', 'init::execute', 'src/main.rs', 'direct', 10),
             ('main', 'run::start', 'src/main.rs', 'direct', 15),
             ('init::execute', 'config::load', 'src/init.rs', 'direct', 5),
             ('run::start', 'config::load', 'src/run.rs', 'direct', 8),
             ('config::load', 'fs::read', 'src/config.rs', 'direct', 20)",
            [],
        )
        .unwrap();

        temp_file
    }

    #[test]
    fn test_generate_dependency_pairs() {
        let temp_db = create_test_db();
        let pairs = generate_dependency_pairs(temp_db.path().to_str().unwrap(), 10).unwrap();

        assert_eq!(pairs.len(), 10);

        // Verify structure
        for pair in &pairs {
            assert!(!pair.anchor.is_empty());
            assert!(!pair.positive.is_empty());
            assert!(!pair.negative.is_empty());
            assert!(pair.anchor.starts_with("Function: "));
        }
    }

    #[test]
    fn test_function_to_text() {
        assert_eq!(
            function_to_text("commands::init::execute"),
            "Function: commands::init::execute (Rust function)"
        );
        assert_eq!(
            function_to_text("obj.method"),
            "Function: obj.method (method call)"
        );
        assert_eq!(
            function_to_text("MyStruct"),
            "Function: MyStruct (type or constructor)"
        );
        assert_eq!(
            function_to_text("simple_func"),
            "Function: simple_func (function)"
        );
    }

    #[test]
    fn test_query_function_events() {
        let temp_db = create_test_db();
        let conn = Connection::open(temp_db.path()).unwrap();
        let events = query_function_events(&conn).unwrap();

        // Should have unique functions: main, init::execute, run::start, config::load, fs::read
        assert_eq!(events.len(), 5);
        assert!(events.iter().all(|(_, text)| text.starts_with("Function: ")));
    }

    #[test]
    fn test_empty_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute(
            "CREATE TABLE call_graph (
                caller TEXT NOT NULL,
                callee TEXT NOT NULL,
                file TEXT NOT NULL,
                call_type TEXT DEFAULT 'direct',
                line_number INTEGER,
                PRIMARY KEY (caller, callee, file, line_number)
            )",
            [],
        )
        .unwrap();

        let result = generate_dependency_pairs(temp_file.path().to_str().unwrap(), 5);
        assert!(result.is_err());
    }
}
