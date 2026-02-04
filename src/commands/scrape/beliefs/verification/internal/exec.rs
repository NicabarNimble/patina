//! Execution dispatch, result building, and storage for verification queries.

use anyhow::Result;
use rusqlite::Connection;

use super::assay;
use super::safety;
use super::temporal;
use crate::commands::scrape::beliefs::verification::{
    VerificationQuery, VerificationResult, VerificationStatus,
};

/// Execute a single verification query and compare against expectation.
///
/// Dispatches to the appropriate executor based on query type.
pub fn execute_verification_query(
    conn: &Connection,
    query: &VerificationQuery,
) -> VerificationResult {
    // Safety check first
    if let Err(e) = safety::validate_query_safety(query) {
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

    match query.query_type.as_str() {
        "sql" => execute_sql_query(conn, query),
        "assay" => execute_assay_query(conn, query),
        "temporal" => execute_temporal_query(conn, query),
        _ => VerificationResult {
            label: query.label.clone(),
            query_type: query.query_type.clone(),
            query_text: query.query_text.clone(),
            expectation: query.expect.clone(),
            status: VerificationStatus::Error,
            result: None,
            error: Some(format!("unknown query type: {}", query.query_type)),
        },
    }
}

/// Execute a SQL verification query — runs raw SQL directly.
fn execute_sql_query(conn: &Connection, query: &VerificationQuery) -> VerificationResult {
    // Execute the query — expect a single numeric result
    let result = conn.query_row(&query.query_text, [], |row| {
        // Try to get as f64 first (handles COUNT, AVG, etc.)
        row.get::<_, f64>(0)
    });

    match result {
        Ok(value) => build_result_from_value(value, query),
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

/// Execute an assay verification query.
///
/// Parses the DSL command, builds counting SQL, executes, and compares.
fn execute_assay_query(conn: &Connection, query: &VerificationQuery) -> VerificationResult {
    // Parse the assay DSL (already validated by safety check, but parse again for data)
    let parsed = match assay::parse_assay_query(&query.query_text) {
        Ok(p) => p,
        Err(e) => {
            return VerificationResult {
                label: query.label.clone(),
                query_type: query.query_type.clone(),
                query_text: query.query_text.clone(),
                expectation: query.expect.clone(),
                status: VerificationStatus::Error,
                result: None,
                error: Some(format!("assay parse error: {}", e)),
            };
        }
    };

    // Build counting SQL
    let (sql, params) = match assay::build_assay_sql(&parsed) {
        Ok(v) => v,
        Err(e) => {
            return VerificationResult {
                label: query.label.clone(),
                query_type: query.query_type.clone(),
                query_text: query.query_text.clone(),
                expectation: query.expect.clone(),
                status: VerificationStatus::Error,
                result: None,
                error: Some(format!("assay SQL build error: {}", e)),
            };
        }
    };

    // ESCAPE is already embedded in the WHERE clause definition
    let result = conn.query_row(&sql, [&params[0]], |row| row.get::<_, f64>(0));

    match result {
        Ok(value) => build_result_from_value(value, query),
        Err(e) => VerificationResult {
            label: query.label.clone(),
            query_type: query.query_type.clone(),
            query_text: query.query_text.clone(),
            expectation: query.expect.clone(),
            status: VerificationStatus::Error,
            result: None,
            error: Some(format!("assay execution error: {}", e)),
        },
    }
}

/// Execute a temporal verification query.
///
/// Parses derive-moments summary field, runs counting SQL, compares.
fn execute_temporal_query(conn: &Connection, query: &VerificationQuery) -> VerificationResult {
    let parsed = match temporal::parse_temporal_query(&query.query_text) {
        Ok(p) => p,
        Err(e) => {
            return VerificationResult {
                label: query.label.clone(),
                query_type: query.query_type.clone(),
                query_text: query.query_text.clone(),
                expectation: query.expect.clone(),
                status: VerificationStatus::Error,
                result: None,
                error: Some(format!("temporal parse error: {}", e)),
            };
        }
    };

    let sql = match temporal::build_temporal_sql(&parsed.summary_field) {
        Ok(s) => s,
        Err(e) => {
            return VerificationResult {
                label: query.label.clone(),
                query_type: query.query_type.clone(),
                query_text: query.query_text.clone(),
                expectation: query.expect.clone(),
                status: VerificationStatus::Error,
                result: None,
                error: Some(format!("temporal SQL build error: {}", e)),
            };
        }
    };

    let result = conn.query_row(&sql, [], |row| row.get::<_, f64>(0));

    match result {
        Ok(value) => build_result_from_value(value, query),
        Err(e) => VerificationResult {
            label: query.label.clone(),
            query_type: query.query_type.clone(),
            query_text: query.query_text.clone(),
            expectation: query.expect.clone(),
            status: VerificationStatus::Error,
            result: None,
            error: Some(format!("temporal execution error: {}", e)),
        },
    }
}

/// Build a VerificationResult from a numeric value and expectation.
fn build_result_from_value(value: f64, query: &VerificationQuery) -> VerificationResult {
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
pub fn store_verification_results(
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

#[cfg(test)]
mod tests {
    use super::*;

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

    // SQL execution tests (in-memory DB)

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

        let result = execute_verification_query(&conn, &query);
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

        let result = execute_verification_query(&conn, &query);
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

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Error);
        assert!(result.error.is_some());
    }

    // Assay execution tests (in-memory DB)

    #[test]
    fn test_execute_assay_callers_count() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE call_graph (caller TEXT, callee TEXT, file TEXT, call_type TEXT);
             INSERT INTO call_graph VALUES ('fn_a', 'insert_event', 'src/a.rs', 'direct');
             INSERT INTO call_graph VALUES ('fn_b', 'insert_event', 'src/b.rs', 'direct');
             INSERT INTO call_graph VALUES ('fn_c', 'insert_event', 'src/a.rs', 'direct');",
        )
        .unwrap();

        let query = VerificationQuery {
            query_type: "assay".to_string(),
            label: "insert_event callers".to_string(),
            expect: "= 3".to_string(),
            query_text: r#"callers --pattern "insert_event""#.to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Pass);
        assert_eq!(result.result, Some("3".to_string()));
    }

    #[test]
    fn test_execute_assay_callers_count_distinct_file() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE call_graph (caller TEXT, callee TEXT, file TEXT, call_type TEXT);
             INSERT INTO call_graph VALUES ('fn_a', 'insert_event', 'src/a.rs', 'direct');
             INSERT INTO call_graph VALUES ('fn_b', 'insert_event', 'src/b.rs', 'direct');
             INSERT INTO call_graph VALUES ('fn_c', 'insert_event', 'src/a.rs', 'direct');",
        )
        .unwrap();

        let query = VerificationQuery {
            query_type: "assay".to_string(),
            label: "insert_event file spread".to_string(),
            expect: "= 2".to_string(),
            query_text: r#"callers --pattern "insert_event" | count(distinct file)"#.to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Pass);
        assert_eq!(result.result, Some("2".to_string()));
    }

    #[test]
    fn test_execute_assay_functions_count() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE function_facts (name TEXT, file TEXT, is_public INTEGER, is_async INTEGER, parameters TEXT, return_type TEXT);
             INSERT INTO function_facts VALUES ('migrate_if_needed', 'src/db.rs', 1, 0, '', 'Result');
             INSERT INTO function_facts VALUES ('run_migration_v2', 'src/db.rs', 0, 0, '', 'Result');
             INSERT INTO function_facts VALUES ('test_migration', 'src/db.rs', 0, 0, '', '()');",
        )
        .unwrap();

        let query = VerificationQuery {
            query_type: "assay".to_string(),
            label: "migration functions".to_string(),
            expect: "= 2".to_string(),
            query_text: r#"functions --pattern "migration""#.to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Pass);
        // 2 match: run_migration_v2, test_migration (migrate_if_needed has "migrate" not "migration")
        assert_eq!(result.result, Some("2".to_string()));
    }

    #[test]
    fn test_execute_assay_importers_count() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE import_facts (file TEXT, import_path TEXT, import_kind TEXT, imported_names TEXT);
             INSERT INTO import_facts VALUES ('src/main.rs', 'commands::assay', 'use', 'execute');
             INSERT INTO import_facts VALUES ('src/main.rs', 'commands::scry', 'use', 'run');
             INSERT INTO import_facts VALUES ('src/mcp.rs', 'commands::assay', 'use', 'AssayOptions');",
        )
        .unwrap();

        let query = VerificationQuery {
            query_type: "assay".to_string(),
            label: "commands importers".to_string(),
            expect: ">= 2".to_string(),
            query_text: r#"importers --pattern "commands""#.to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Pass);
        // 3 rows match "commands" substring
        assert_eq!(result.result, Some("3".to_string()));
    }

    #[test]
    fn test_execute_assay_contested() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE call_graph (caller TEXT, callee TEXT, file TEXT, call_type TEXT);
             INSERT INTO call_graph VALUES ('fn_a', 'insert_event', 'src/a.rs', 'direct');",
        )
        .unwrap();

        let query = VerificationQuery {
            query_type: "assay".to_string(),
            label: "should have many callers".to_string(),
            expect: ">= 10".to_string(),
            query_text: r#"callers --pattern "insert_event""#.to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Contested);
        assert_eq!(result.result, Some("1".to_string()));
    }

    #[test]
    fn test_execute_assay_missing_table() {
        let conn = Connection::open_in_memory().unwrap();

        let query = VerificationQuery {
            query_type: "assay".to_string(),
            label: "no table".to_string(),
            expect: "= 0".to_string(),
            query_text: r#"callers --pattern "test""#.to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Error);
        assert!(result.error.unwrap().contains("execution error"));
    }

    // Temporal execution tests (in-memory DB)

    #[test]
    fn test_execute_temporal_total_commits() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE commits (sha TEXT PRIMARY KEY, message TEXT, timestamp TEXT, author_name TEXT, author_email TEXT);
             INSERT INTO commits VALUES ('abc123', 'feat: add feature', '2026-01-01T00:00:00Z', 'dev', 'dev@test.com');
             INSERT INTO commits VALUES ('def456', 'fix: bug fix', '2026-01-02T00:00:00Z', 'dev', 'dev@test.com');
             INSERT INTO commits VALUES ('ghi789', 'refactor: cleanup', '2026-01-03T00:00:00Z', 'dev', 'dev@test.com');",
        )
        .unwrap();

        let query = VerificationQuery {
            query_type: "temporal".to_string(),
            label: "has commits".to_string(),
            expect: "= 3".to_string(),
            query_text: "derive-moments | summary.total_commits".to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Pass);
        assert_eq!(result.result, Some("3".to_string()));
    }

    #[test]
    fn test_execute_temporal_rewrite_count() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE commits (sha TEXT PRIMARY KEY, message TEXT, timestamp TEXT, author_name TEXT, author_email TEXT);
             INSERT INTO commits VALUES ('abc', 'feat: add feature', '2026-01-01T00:00:00Z', 'dev', 'dev@test.com');
             INSERT INTO commits VALUES ('def', 'refactor: cleanup code', '2026-01-02T00:00:00Z', 'dev', 'dev@test.com');
             INSERT INTO commits VALUES ('ghi', 'rewrite: new parser', '2026-01-03T00:00:00Z', 'dev', 'dev@test.com');
             INSERT INTO commits VALUES ('jkl', 'fix: bug', '2026-01-04T00:00:00Z', 'dev', 'dev@test.com');",
        )
        .unwrap();

        let query = VerificationQuery {
            query_type: "temporal".to_string(),
            label: "rewrite count".to_string(),
            expect: "= 2".to_string(),
            query_text: "derive-moments | summary.rewrite".to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Pass);
        assert_eq!(result.result, Some("2".to_string()));
    }

    #[test]
    fn test_execute_temporal_migration_count() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE commits (sha TEXT PRIMARY KEY, message TEXT, timestamp TEXT, author_name TEXT, author_email TEXT);
             INSERT INTO commits VALUES ('abc', 'feat: migrate to new schema', '2026-01-01T00:00:00Z', 'dev', 'dev@test.com');
             INSERT INTO commits VALUES ('def', 'fix: migration edge case', '2026-01-02T00:00:00Z', 'dev', 'dev@test.com');
             INSERT INTO commits VALUES ('ghi', 'feat: add feature', '2026-01-03T00:00:00Z', 'dev', 'dev@test.com');",
        )
        .unwrap();

        let query = VerificationQuery {
            query_type: "temporal".to_string(),
            label: "migration count".to_string(),
            expect: "= 2".to_string(),
            query_text: "derive-moments | summary.migration".to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Pass);
        assert_eq!(result.result, Some("2".to_string()));
    }

    #[test]
    fn test_execute_temporal_invalid_field() {
        let conn = Connection::open_in_memory().unwrap();

        let query = VerificationQuery {
            query_type: "temporal".to_string(),
            label: "bad field".to_string(),
            expect: "= 0".to_string(),
            query_text: "derive-moments | summary.nonexistent".to_string(),
        };

        let result = execute_verification_query(&conn, &query);
        assert_eq!(result.status, VerificationStatus::Error);
        assert!(result.error.unwrap().contains("safety validation failed"));
    }
}
