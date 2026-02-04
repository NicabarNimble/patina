//! Query safety validation — ensure verification queries cannot modify data.

use super::assay;
use super::temporal;
use crate::commands::scrape::beliefs::verification::VerificationQuery;

/// Validate that a query is safe to execute.
///
/// For SQL: must be SELECT-only (no INSERT, UPDATE, DELETE, DROP, ALTER, etc.)
/// For assay: validates command, pattern, and aggregation through the DSL parser
/// For temporal: validates derive-moments command and summary field
pub fn validate_query_safety(query: &VerificationQuery) -> Result<(), String> {
    match query.query_type.as_str() {
        "sql" => validate_sql_safety(&query.query_text),
        "assay" => {
            // Full parse validates command, pattern, and field allowlist
            assay::parse_assay_query(&query.query_text)?;
            Ok(())
        }
        "temporal" => {
            // Full parse validates derive-moments command and summary field
            temporal::parse_temporal_query(&query.query_text)?;
            Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_validate_assay_safety_valid() {
        let q = VerificationQuery {
            query_type: "assay".to_string(),
            label: "test".to_string(),
            expect: ">= 1".to_string(),
            query_text: r#"callers --pattern "test""#.to_string(),
        };
        assert!(validate_query_safety(&q).is_ok());
    }

    #[test]
    fn test_validate_assay_safety_rejects_unknown_command() {
        let q = VerificationQuery {
            query_type: "assay".to_string(),
            label: "test".to_string(),
            expect: ">= 1".to_string(),
            query_text: r#"drop_tables --pattern "test""#.to_string(),
        };
        assert!(validate_query_safety(&q).is_err());
    }

    #[test]
    fn test_validate_temporal_safety_valid() {
        let q = VerificationQuery {
            query_type: "temporal".to_string(),
            label: "test".to_string(),
            expect: ">= 1".to_string(),
            query_text: "derive-moments | summary.total_commits".to_string(),
        };
        assert!(validate_query_safety(&q).is_ok());
    }

    #[test]
    fn test_validate_temporal_safety_rejects_bad_field() {
        let q = VerificationQuery {
            query_type: "temporal".to_string(),
            label: "test".to_string(),
            expect: ">= 1".to_string(),
            query_text: "derive-moments | summary.drop_table".to_string(),
        };
        assert!(validate_query_safety(&q).is_err());
    }
}
