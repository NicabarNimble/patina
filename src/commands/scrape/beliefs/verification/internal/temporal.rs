//! Temporal DSL — translate derive-moments commands into counting SQL.

/// Allowed fields for `derive-moments | summary.<field>` temporal queries.
const TEMPORAL_SUMMARY_FIELDS: &[&str] = &[
    "total_commits",
    "genesis",
    "big_bang",
    "major",
    "breaking",
    "migration",
    "rewrite",
];

/// Parsed temporal query from DSL text.
#[derive(Debug, Clone)]
pub struct ParsedTemporalQuery {
    pub summary_field: String,
}

/// Parse a temporal DSL command string.
///
/// Format: `derive-moments | summary.<field>`
///
/// Example: `derive-moments | summary.total_commits`
pub fn parse_temporal_query(text: &str) -> std::result::Result<ParsedTemporalQuery, String> {
    let text = text.trim();

    // Must start with derive-moments
    let rest = text
        .strip_prefix("derive-moments")
        .ok_or_else(|| "temporal query must start with 'derive-moments'".to_string())?
        .trim();

    // Must have pipe
    let after_pipe = rest
        .strip_prefix('|')
        .ok_or_else(|| {
            "temporal query requires '| summary.<field>' (e.g., '| summary.total_commits')"
                .to_string()
        })?
        .trim();

    // Must be summary.<field>
    let field = after_pipe
        .strip_prefix("summary.")
        .ok_or_else(|| "temporal selector must be 'summary.<field>'".to_string())?
        .trim();

    if field.is_empty() {
        return Err("summary field name cannot be empty".to_string());
    }

    if !TEMPORAL_SUMMARY_FIELDS.contains(&field) {
        return Err(format!(
            "unknown temporal summary field '{}' (allowed: {})",
            field,
            TEMPORAL_SUMMARY_FIELDS.join(", ")
        ));
    }

    Ok(ParsedTemporalQuery {
        summary_field: field.to_string(),
    })
}

/// Build SQL for a temporal summary field query.
///
/// Runs directly against commits/commit_files tables — self-contained,
/// no dependency on the moments table having been populated.
pub fn build_temporal_sql(field: &str) -> std::result::Result<String, String> {
    match field {
        "total_commits" => Ok("SELECT COUNT(*) FROM commits".to_string()),
        "genesis" => {
            // Genesis = first commit exists (1 if any commits, 0 if empty)
            Ok("SELECT CASE WHEN COUNT(*) > 0 THEN 1 ELSE 0 END FROM commits".to_string())
        }
        "big_bang" => Ok(
            "SELECT COUNT(*) FROM (SELECT sha FROM commit_files GROUP BY sha HAVING COUNT(*) > 100)"
                .to_string(),
        ),
        "major" => Ok(
            "SELECT COUNT(*) FROM (SELECT sha FROM commit_files GROUP BY sha HAVING COUNT(*) > 50 AND COUNT(*) <= 100)"
                .to_string(),
        ),
        "breaking" => Ok(
            "SELECT COUNT(*) FROM commits WHERE LOWER(message) LIKE '%breaking%'".to_string(),
        ),
        "migration" => Ok(
            "SELECT COUNT(*) FROM commits WHERE LOWER(message) LIKE '%migrate%' OR LOWER(message) LIKE '%migration%'"
                .to_string(),
        ),
        "rewrite" => Ok(
            "SELECT COUNT(*) FROM commits WHERE LOWER(message) LIKE '%rewrite%' OR LOWER(message) LIKE '%refactor%'"
                .to_string(),
        ),
        _ => Err(format!("unknown temporal field: {}", field)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_temporal_query_total_commits() {
        let parsed = parse_temporal_query("derive-moments | summary.total_commits").unwrap();
        assert_eq!(parsed.summary_field, "total_commits");
    }

    #[test]
    fn test_parse_temporal_query_rewrite() {
        let parsed = parse_temporal_query("derive-moments | summary.rewrite").unwrap();
        assert_eq!(parsed.summary_field, "rewrite");
    }

    #[test]
    fn test_parse_temporal_query_migration() {
        let parsed = parse_temporal_query("derive-moments | summary.migration").unwrap();
        assert_eq!(parsed.summary_field, "migration");
    }

    #[test]
    fn test_parse_temporal_query_unknown_field() {
        let err = parse_temporal_query("derive-moments | summary.unknown").unwrap_err();
        assert!(err.contains("unknown temporal summary field"));
    }

    #[test]
    fn test_parse_temporal_query_missing_pipe() {
        let err = parse_temporal_query("derive-moments").unwrap_err();
        assert!(err.contains("requires"));
    }

    #[test]
    fn test_parse_temporal_query_wrong_command() {
        let err = parse_temporal_query("derive | summary.total_commits").unwrap_err();
        assert!(err.contains("derive-moments"));
    }

    #[test]
    fn test_parse_temporal_query_missing_summary_prefix() {
        let err = parse_temporal_query("derive-moments | total_commits").unwrap_err();
        assert!(err.contains("summary.<field>"));
    }

    // SQL builder tests

    #[test]
    fn test_build_temporal_sql_total_commits() {
        let sql = build_temporal_sql("total_commits").unwrap();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("commits"));
    }

    #[test]
    fn test_build_temporal_sql_rewrite() {
        let sql = build_temporal_sql("rewrite").unwrap();
        assert!(sql.contains("rewrite"));
        assert!(sql.contains("refactor"));
    }

    #[test]
    fn test_build_temporal_sql_unknown() {
        assert!(build_temporal_sql("unknown").is_err());
    }
}
