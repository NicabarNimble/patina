//! Belief verification — parse, validate, execute, and store verification queries
//!
//! Verification queries live in belief markdown files as `## Verification` sections
//! with fenced code blocks using the `verify` info-string. Results are stored in
//! the `belief_verifications` table and aggregated on the `beliefs` table.
//!
//! Design: queries are source data (authored intent), results are derived data (DB only).
//! See: layer/surface/build/feat/belief-verification/SPEC.md

use anyhow::Result;
use rusqlite::Connection;

/// A parsed verification query from a belief's `## Verification` section
#[derive(Debug, Clone)]
pub struct VerificationQuery {
    pub query_type: String, // "sql", "assay", "temporal"
    pub label: String,
    pub expect: String,     // "= 0", "> 5", ">= 1", "< 10"
    pub query_text: String, // SQL or assay command
}

/// Result of executing a single verification query
#[derive(Debug)]
pub struct VerificationResult {
    pub label: String,
    pub query_type: String,
    pub query_text: String,
    pub expectation: String,
    pub status: VerificationStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum VerificationStatus {
    Pass,
    Contested,
    Error,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationStatus::Pass => "pass",
            VerificationStatus::Contested => "contested",
            VerificationStatus::Error => "error",
        }
    }
}

/// Aggregate verification counts for a belief
#[derive(Debug, Default)]
pub struct VerificationAggregates {
    pub total: i32,
    pub passed: i32,
    pub failed: i32,
    pub errored: i32,
}

// ============================================================================
// Parsing
// ============================================================================

/// Parse `## Verification` section from belief markdown content.
///
/// Extracts fenced code blocks with `verify` info-string:
/// ```text
/// ```verify type="sql" label="No async functions" expect="= 0"
/// SELECT COUNT(*) FROM function_facts WHERE is_async = 1
/// ```
/// ```
pub fn parse_verification_blocks(content: &str) -> Vec<VerificationQuery> {
    let mut queries = Vec::new();
    let mut in_verification_section = false;
    let mut in_verify_block = false;
    let mut current_attrs: Option<(String, String, String)> = None; // (type, label, expect)
    let mut current_body = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Track which section we're in
        if trimmed.starts_with("## ") {
            in_verification_section = trimmed.starts_with("## Verification");
            continue;
        }

        if !in_verification_section {
            continue;
        }

        if in_verify_block {
            // End of fenced block
            if trimmed.starts_with("```") {
                if let Some((query_type, label, expect)) = current_attrs.take() {
                    let query_text = current_body.trim().to_string();
                    if !query_text.is_empty() {
                        queries.push(VerificationQuery {
                            query_type,
                            label,
                            expect,
                            query_text,
                        });
                    }
                }
                in_verify_block = false;
                current_body.clear();
            } else {
                current_body.push_str(line);
                current_body.push('\n');
            }
        } else if let Some(info_string) = trimmed.strip_prefix("```verify") {
            // Start of verify block — parse attributes from info-string
            if let Some(attrs) = parse_verify_attributes(info_string) {
                current_attrs = Some(attrs);
                in_verify_block = true;
                current_body.clear();
            }
        }
    }

    queries
}

/// Parse key="value" attributes from a verify info-string.
///
/// Input: ` type="sql" label="No async functions" expect="= 0"`
/// Returns: (type, label, expect) or None if required attributes are missing.
fn parse_verify_attributes(info: &str) -> Option<(String, String, String)> {
    let mut query_type = None;
    let mut label = None;
    let mut expect = None;

    // Simple state machine: find key="value" pairs
    let mut chars = info.chars().peekable();
    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Read key
        let key: String = chars
            .by_ref()
            .take_while(|&c| c != '=')
            .collect::<String>()
            .trim()
            .to_string();

        if key.is_empty() {
            break;
        }

        // Expect opening quote
        if chars.peek() != Some(&'"') {
            // Skip non-quoted values
            while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                chars.next();
            }
            continue;
        }
        chars.next(); // consume opening "

        // Read value until closing quote
        let value: String = chars.by_ref().take_while(|&c| c != '"').collect();

        match key.as_str() {
            "type" => query_type = Some(value),
            "label" => label = Some(value),
            "expect" => expect = Some(value),
            _ => {} // ignore unknown attributes
        }
    }

    match (query_type, label, expect) {
        (Some(t), Some(l), Some(e)) => Some((t, l, e)),
        _ => None,
    }
}

// ============================================================================
// Safety validation
// ============================================================================

/// Validate that a query is safe to execute.
///
/// For SQL: must be SELECT-only (no INSERT, UPDATE, DELETE, DROP, ALTER, etc.)
/// For assay: must be an allowlisted subcommand (Phase 2, stub for now)
pub fn validate_query_safety(query: &VerificationQuery) -> Result<(), String> {
    match query.query_type.as_str() {
        "sql" => validate_sql_safety(&query.query_text),
        "assay" | "temporal" => {
            // Phase 2: assay validation will go here
            Err(format!(
                "query type '{}' not yet supported (Phase 2)",
                query.query_type
            ))
        }
        other => Err(format!("unknown query type: '{}'", other)),
    }
}

/// Validate a SQL query is SELECT-only.
///
/// Rejects any statement that could modify data. The check is conservative:
/// the query must start with SELECT (after stripping whitespace/comments).
fn validate_sql_safety(sql: &str) -> Result<(), String> {
    let normalized = sql
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with("--"))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_uppercase();

    if normalized.is_empty() {
        return Err("empty query".to_string());
    }

    // Must start with SELECT or WITH (for CTEs)
    if !normalized.starts_with("SELECT") && !normalized.starts_with("WITH") {
        return Err(format!(
            "query must start with SELECT or WITH, got: '{}'",
            &normalized[..normalized.len().min(40)]
        ));
    }

    // Reject dangerous keywords anywhere in the query
    let dangerous = [
        "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "CREATE", "ATTACH", "DETACH", "PRAGMA",
        "REPLACE", "VACUUM", "REINDEX",
    ];
    for keyword in &dangerous {
        // Check for keyword as a whole word (not part of a column name)
        // Simple heuristic: preceded and followed by non-alphanumeric
        let pattern = format!(" {} ", keyword);
        if normalized.contains(&pattern)
            || normalized.starts_with(&format!("{} ", keyword))
            || normalized.ends_with(&format!(" {}", keyword))
        {
            return Err(format!("query contains forbidden keyword: {}", keyword));
        }
    }

    Ok(())
}

// ============================================================================
// Execution
// ============================================================================

/// Execute a single SQL verification query and compare against expectation.
pub fn execute_sql_query(conn: &Connection, query: &VerificationQuery) -> VerificationResult {
    // Safety check first
    if let Err(e) = validate_query_safety(query) {
        return VerificationResult {
            label: query.label.clone(),
            query_type: query.query_type.clone(),
            query_text: query.query_text.clone(),
            expectation: query.expect.clone(),
            status: VerificationStatus::Error,
            result: None,
            error: Some(format!("safety validation failed: {}", e)),
        };
    }

    // Execute the query — expect a single numeric result
    let result = conn.query_row(&query.query_text, [], |row| {
        // Try to get as f64 first (handles COUNT, AVG, etc.)
        row.get::<_, f64>(0)
    });

    match result {
        Ok(value) => {
            // Compare against expectation
            match evaluate_expectation(value, &query.expect) {
                Ok(passed) => VerificationResult {
                    label: query.label.clone(),
                    query_type: query.query_type.clone(),
                    query_text: query.query_text.clone(),
                    expectation: query.expect.clone(),
                    status: if passed {
                        VerificationStatus::Pass
                    } else {
                        VerificationStatus::Contested
                    },
                    result: Some(format_result(value)),
                    error: None,
                },
                Err(e) => VerificationResult {
                    label: query.label.clone(),
                    query_type: query.query_type.clone(),
                    query_text: query.query_text.clone(),
                    expectation: query.expect.clone(),
                    status: VerificationStatus::Error,
                    result: Some(format_result(value)),
                    error: Some(format!("expectation parse error: {}", e)),
                },
            }
        }
        Err(e) => VerificationResult {
            label: query.label.clone(),
            query_type: query.query_type.clone(),
            query_text: query.query_text.clone(),
            expectation: query.expect.clone(),
            status: VerificationStatus::Error,
            result: None,
            error: Some(format!("SQL error: {}", e)),
        },
    }
}

/// Format a numeric result for display — integers show as integers, floats keep decimals
fn format_result(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < i64::MAX as f64 {
        format!("{}", value as i64)
    } else {
        format!("{:.2}", value)
    }
}

/// Evaluate an expectation string against a numeric value.
///
/// Supported formats: "= 0", "> 5", ">= 1", "< 10", "<= 100"
fn evaluate_expectation(value: f64, expect: &str) -> Result<bool, String> {
    let expect = expect.trim();

    // Parse operator and threshold
    let (op, threshold_str) = if let Some(rest) = expect.strip_prefix(">=") {
        (">=", rest.trim())
    } else if let Some(rest) = expect.strip_prefix("<=") {
        ("<=", rest.trim())
    } else if let Some(rest) = expect.strip_prefix('>') {
        (">", rest.trim())
    } else if let Some(rest) = expect.strip_prefix('<') {
        ("<", rest.trim())
    } else if let Some(rest) = expect.strip_prefix('=') {
        ("=", rest.trim())
    } else {
        return Err(format!("unrecognized expectation format: '{}'", expect));
    };

    let threshold: f64 = threshold_str
        .parse()
        .map_err(|_| format!("cannot parse threshold '{}' as number", threshold_str))?;

    let result = match op {
        "=" => (value - threshold).abs() < f64::EPSILON,
        ">" => value > threshold,
        ">=" => value >= threshold,
        "<" => value < threshold,
        "<=" => value <= threshold,
        _ => unreachable!(),
    };

    Ok(result)
}

/// Run all verification queries for a belief.
///
/// Only executes SQL queries in Phase 1. Assay/temporal queries return errors
/// indicating they're not yet supported.
pub fn run_verification_queries(
    conn: &Connection,
    belief_id: &str,
    queries: &[VerificationQuery],
    data_freshness: &str,
) -> (Vec<VerificationResult>, VerificationAggregates) {
    let mut results = Vec::new();
    let mut aggregates = VerificationAggregates::default();

    for query in queries {
        let result = match query.query_type.as_str() {
            "sql" => execute_sql_query(conn, query),
            _ => VerificationResult {
                label: query.label.clone(),
                query_type: query.query_type.clone(),
                query_text: query.query_text.clone(),
                expectation: query.expect.clone(),
                status: VerificationStatus::Error,
                result: None,
                error: Some(format!(
                    "query type '{}' not yet supported",
                    query.query_type
                )),
            },
        };

        aggregates.total += 1;
        match result.status {
            VerificationStatus::Pass => aggregates.passed += 1,
            VerificationStatus::Contested => aggregates.failed += 1,
            VerificationStatus::Error => aggregates.errored += 1,
        }

        results.push(result);
    }

    // Store results in belief_verifications table
    if let Err(e) = store_verification_results(conn, belief_id, &results, data_freshness) {
        eprintln!(
            "  Warning: failed to store verification results for {}: {}",
            belief_id, e
        );
    }

    (results, aggregates)
}

// ============================================================================
// Storage
// ============================================================================

/// Create the belief_verifications table and add aggregate columns to beliefs.
pub fn create_tables(conn: &Connection) -> Result<()> {
    // Verification results are transient — recomputed on every scrape.
    // Drop and recreate to handle schema changes cleanly (no migration needed).
    // No FK constraint: Phase 2.5 stores results before Phase 3 inserts beliefs.
    conn.execute_batch(
        r#"
        DROP TABLE IF EXISTS belief_verifications;
        CREATE TABLE belief_verifications (
            belief_id TEXT NOT NULL,
            label TEXT NOT NULL,
            query_type TEXT NOT NULL,
            query_text TEXT NOT NULL,
            expectation TEXT NOT NULL,
            last_status TEXT NOT NULL,
            last_result TEXT,
            last_error TEXT,
            last_run_at TEXT NOT NULL,
            data_freshness TEXT NOT NULL,
            PRIMARY KEY (belief_id, label)
        );
        "#,
    )?;

    // Add aggregate columns to beliefs table (ignore if already exist)
    let columns = [
        ("verification_total", "INTEGER DEFAULT 0"),
        ("verification_passed", "INTEGER DEFAULT 0"),
        ("verification_failed", "INTEGER DEFAULT 0"),
        ("verification_errored", "INTEGER DEFAULT 0"),
    ];

    for (col_name, col_type) in &columns {
        let sql = format!("ALTER TABLE beliefs ADD COLUMN {} {}", col_name, col_type);
        let _ = conn.execute(&sql, []);
    }

    Ok(())
}

/// Store verification results in the belief_verifications table.
fn store_verification_results(
    conn: &Connection,
    belief_id: &str,
    results: &[VerificationResult],
    data_freshness: &str,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    // Delete old results for this belief (clean slate on each scrape)
    conn.execute(
        "DELETE FROM belief_verifications WHERE belief_id = ?1",
        [belief_id],
    )?;

    let mut stmt = conn.prepare(
        "INSERT INTO belief_verifications (belief_id, label, query_type, query_text, expectation, last_status, last_result, last_error, last_run_at, data_freshness)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;

    for result in results {
        stmt.execute(rusqlite::params![
            belief_id,
            result.label,
            result.query_type,
            result.query_text,
            result.expectation,
            result.status.as_str(),
            result.result,
            result.error,
            now,
            data_freshness,
        ])?;
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_verification_blocks() {
        let content = r#"---
type: belief
id: test-belief
---

# test-belief

Some statement.

## Verification

```verify type="sql" label="No async functions" expect="= 0"
SELECT COUNT(*) FROM function_facts WHERE is_async = 1
```

```verify type="sql" label="No tokio imports" expect="= 0"
SELECT COUNT(*) FROM import_facts WHERE import_path LIKE '%tokio%'
```

## Evidence

- Some evidence here
"#;

        let queries = parse_verification_blocks(content);
        assert_eq!(queries.len(), 2);

        assert_eq!(queries[0].query_type, "sql");
        assert_eq!(queries[0].label, "No async functions");
        assert_eq!(queries[0].expect, "= 0");
        assert!(queries[0]
            .query_text
            .contains("SELECT COUNT(*) FROM function_facts"));

        assert_eq!(queries[1].label, "No tokio imports");
    }

    #[test]
    fn test_parse_empty_verification() {
        let content = "## Statement\nSome belief.\n## Evidence\n- something\n";
        let queries = parse_verification_blocks(content);
        assert!(queries.is_empty());
    }

    #[test]
    fn test_parse_verification_stops_at_next_section() {
        let content = r#"## Verification

```verify type="sql" label="Test" expect="= 0"
SELECT 1
```

## Evidence

```verify type="sql" label="Should not be parsed" expect="= 0"
SELECT 2
```
"#;

        let queries = parse_verification_blocks(content);
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].label, "Test");
    }

    #[test]
    fn test_parse_verify_attributes() {
        let attrs = parse_verify_attributes(r#" type="sql" label="Test query" expect=">= 5""#);
        assert!(attrs.is_some());
        let (t, l, e) = attrs.unwrap();
        assert_eq!(t, "sql");
        assert_eq!(l, "Test query");
        assert_eq!(e, ">= 5");
    }

    #[test]
    fn test_parse_verify_attributes_missing_field() {
        let attrs = parse_verify_attributes(r#" type="sql" label="Test""#);
        assert!(attrs.is_none()); // missing expect
    }

    #[test]
    fn test_validate_sql_safety_select() {
        let q = VerificationQuery {
            query_type: "sql".to_string(),
            label: "test".to_string(),
            expect: "= 0".to_string(),
            query_text: "SELECT COUNT(*) FROM function_facts WHERE is_async = 1".to_string(),
        };
        assert!(validate_query_safety(&q).is_ok());
    }

    #[test]
    fn test_validate_sql_safety_with_cte() {
        let q = VerificationQuery {
            query_type: "sql".to_string(),
            label: "test".to_string(),
            expect: "< 10".to_string(),
            query_text:
                "WITH counts AS (SELECT COUNT(*) as c FROM commits) SELECT AVG(c) FROM counts"
                    .to_string(),
        };
        assert!(validate_query_safety(&q).is_ok());
    }

    #[test]
    fn test_validate_sql_safety_rejects_insert() {
        let q = VerificationQuery {
            query_type: "sql".to_string(),
            label: "test".to_string(),
            expect: "= 0".to_string(),
            query_text: "INSERT INTO beliefs VALUES ('hack', 'bad')".to_string(),
        };
        assert!(validate_query_safety(&q).is_err());
    }

    #[test]
    fn test_validate_sql_safety_rejects_drop() {
        let q = VerificationQuery {
            query_type: "sql".to_string(),
            label: "test".to_string(),
            expect: "= 0".to_string(),
            query_text: "SELECT 1; DROP TABLE beliefs".to_string(),
        };
        assert!(validate_query_safety(&q).is_err());
    }

    #[test]
    fn test_evaluate_expectation_equals() {
        assert!(evaluate_expectation(0.0, "= 0").unwrap());
        assert!(!evaluate_expectation(1.0, "= 0").unwrap());
    }

    #[test]
    fn test_evaluate_expectation_greater() {
        assert!(evaluate_expectation(10.0, "> 5").unwrap());
        assert!(!evaluate_expectation(5.0, "> 5").unwrap());
    }

    #[test]
    fn test_evaluate_expectation_greater_equal() {
        assert!(evaluate_expectation(5.0, ">= 5").unwrap());
        assert!(evaluate_expectation(6.0, ">= 5").unwrap());
        assert!(!evaluate_expectation(4.0, ">= 5").unwrap());
    }

    #[test]
    fn test_evaluate_expectation_less() {
        assert!(evaluate_expectation(4.0, "< 10").unwrap());
        assert!(!evaluate_expectation(10.0, "< 10").unwrap());
    }

    #[test]
    fn test_evaluate_expectation_less_equal() {
        assert!(evaluate_expectation(10.0, "<= 10").unwrap());
        assert!(!evaluate_expectation(11.0, "<= 10").unwrap());
    }

    #[test]
    fn test_evaluate_expectation_invalid() {
        assert!(evaluate_expectation(0.0, "~= 5").is_err());
        assert!(evaluate_expectation(0.0, "= abc").is_err());
    }

    #[test]
    fn test_format_result_integer() {
        assert_eq!(format_result(0.0), "0");
        assert_eq!(format_result(29.0), "29");
        assert_eq!(format_result(1520.0), "1520");
    }

    #[test]
    fn test_format_result_float() {
        assert_eq!(format_result(4.08), "4.08");
        assert_eq!(format_result(3.14159), "3.14");
    }

    #[test]
    fn test_execute_sql_query_against_real_db() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE test (val INTEGER); INSERT INTO test VALUES (42);")
            .unwrap();

        let query = VerificationQuery {
            query_type: "sql".to_string(),
            label: "test value".to_string(),
            expect: "= 42".to_string(),
            query_text: "SELECT val FROM test".to_string(),
        };

        let result = execute_sql_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Pass);
        assert_eq!(result.result, Some("42".to_string()));
    }

    #[test]
    fn test_execute_sql_query_contested() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE test (val INTEGER); INSERT INTO test VALUES (5);")
            .unwrap();

        let query = VerificationQuery {
            query_type: "sql".to_string(),
            label: "should be zero".to_string(),
            expect: "= 0".to_string(),
            query_text: "SELECT val FROM test".to_string(),
        };

        let result = execute_sql_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Contested);
        assert_eq!(result.result, Some("5".to_string()));
    }

    #[test]
    fn test_execute_sql_query_error() {
        let conn = Connection::open_in_memory().unwrap();

        let query = VerificationQuery {
            query_type: "sql".to_string(),
            label: "bad query".to_string(),
            expect: "= 0".to_string(),
            query_text: "SELECT * FROM nonexistent_table".to_string(),
        };

        let result = execute_sql_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Error);
        assert!(result.error.is_some());
    }
}
