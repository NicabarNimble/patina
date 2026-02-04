//! Assay DSL — translate assay commands into counting SQL.
//!
//! The verification engine builds `SELECT COUNT(*) FROM table WHERE condition`
//! directly, avoiding row fetching and the truncation problem entirely.

/// Aggregation to apply to assay query results.
///
/// Verified against per-command allowed fields before execution.
#[derive(Debug, Clone, PartialEq)]
pub enum Aggregation {
    /// Count all matching rows (default when no pipe)
    CountAll,
    /// Count distinct values of a specific field
    CountDistinct(String),
}

/// Definition of an assay command — maps DSL syntax to SQL.
struct AssayCommandDef {
    table: &'static str,
    where_clause: &'static str,
    /// Fields allowed in `count(distinct <field>)` — validated per-command
    count_fields: &'static [&'static str],
}

/// Parsed assay query from DSL text.
#[derive(Debug, Clone)]
pub struct ParsedAssayQuery {
    pub command: String,
    pub pattern: String,
    pub aggregation: Aggregation,
}

/// Registry of assay commands and their SQL definitions.
///
/// ESCAPE '\\' is embedded in each LIKE clause to ensure escaped wildcards
/// in patterns are handled correctly (e.g., `\_` matches literal underscore).
fn get_assay_command(name: &str) -> Option<AssayCommandDef> {
    match name {
        "callers" => Some(AssayCommandDef {
            table: "call_graph",
            where_clause: "callee LIKE ?1 ESCAPE '\\'",
            count_fields: &["file", "caller", "callee", "call_type"],
        }),
        "callees" => Some(AssayCommandDef {
            table: "call_graph",
            where_clause: "caller LIKE ?1 ESCAPE '\\'",
            count_fields: &["file", "caller", "callee", "call_type"],
        }),
        "functions" => Some(AssayCommandDef {
            table: "function_facts",
            where_clause: "name LIKE ?1 ESCAPE '\\' OR file LIKE ?1 ESCAPE '\\'",
            count_fields: &["file", "name", "is_public", "is_async", "return_type"],
        }),
        "imports" => Some(AssayCommandDef {
            table: "import_facts",
            where_clause: "file LIKE ?1 ESCAPE '\\'",
            count_fields: &["import_path", "import_kind"],
        }),
        "importers" => Some(AssayCommandDef {
            table: "import_facts",
            where_clause: "import_path LIKE ?1 ESCAPE '\\'",
            count_fields: &["file"],
        }),
        _ => None,
    }
}

/// Escape LIKE wildcards in a pattern string.
///
/// Prevents `%` and `_` in user input from changing query semantics.
/// Uses `\` as escape character (must pair with ESCAPE '\' in SQL).
fn escape_like_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Parse an assay DSL command string.
///
/// Format: `<command> --pattern "<pattern>"` with optional `| count(distinct <field>)`
///
/// Examples:
///   `callers --pattern "insert_event"`
///   `functions --pattern "migration" | count(distinct file)`
pub fn parse_assay_query(text: &str) -> std::result::Result<ParsedAssayQuery, String> {
    let text = text.trim();

    // Split on pipe first
    let (command_part, pipe_part) = if let Some(idx) = text.find('|') {
        (text[..idx].trim(), Some(text[idx + 1..].trim()))
    } else {
        (text, None)
    };

    // Parse command name
    let mut parts = command_part.split_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| "empty assay command".to_string())?
        .to_string();

    // Validate command exists
    let cmd_def = get_assay_command(&command).ok_or_else(|| {
        format!(
            "unknown assay command '{}' (allowed: callers, callees, functions, imports, importers)",
            command
        )
    })?;

    // Parse --pattern "value"
    let pattern = parse_pattern_arg(parts.collect::<Vec<_>>().join(" ").as_str())?;

    // Parse optional pipe aggregation
    let aggregation = match pipe_part {
        None => Aggregation::CountAll,
        Some(pipe) => parse_aggregation(pipe, &command, cmd_def.count_fields)?,
    };

    Ok(ParsedAssayQuery {
        command,
        pattern,
        aggregation,
    })
}

/// Build counting SQL for an assay command.
///
/// Returns (sql, params) ready for rusqlite execution.
pub fn build_assay_sql(
    parsed: &ParsedAssayQuery,
) -> std::result::Result<(String, Vec<String>), String> {
    let cmd_def = get_assay_command(&parsed.command)
        .ok_or_else(|| format!("unknown command: {}", parsed.command))?;

    let escaped = escape_like_pattern(&parsed.pattern);
    let like_pattern = format!("%{}%", escaped);

    let select = match &parsed.aggregation {
        Aggregation::CountAll => "COUNT(*)".to_string(),
        Aggregation::CountDistinct(field) => format!("COUNT(DISTINCT {})", field),
    };

    let sql = format!(
        "SELECT {} FROM {} WHERE {}",
        select, cmd_def.table, cmd_def.where_clause
    );

    Ok((sql, vec![like_pattern]))
}

/// Parse `--pattern "value"` from argument string.
fn parse_pattern_arg(args: &str) -> std::result::Result<String, String> {
    let args = args.trim();

    // Look for --pattern "value" or --pattern 'value' or --pattern value
    if let Some(rest) = args.strip_prefix("--pattern") {
        let rest = rest.trim();
        if let Some(quoted) = rest.strip_prefix('"') {
            // Find closing quote
            if let Some(end) = quoted.find('"') {
                let value = &quoted[..end];
                if value.is_empty() {
                    return Err("--pattern value cannot be empty".to_string());
                }
                return Ok(value.to_string());
            }
            return Err("unclosed quote in --pattern value".to_string());
        }
        // Unquoted value — take first word
        let value = rest.split_whitespace().next().unwrap_or("");
        if value.is_empty() {
            return Err("--pattern requires a value".to_string());
        }
        Ok(value.to_string())
    } else {
        Err("assay command requires --pattern argument".to_string())
    }
}

/// Parse aggregation from pipe expression.
///
/// Allowed forms:
///   `count` → CountAll
///   `count(distinct <field>)` → CountDistinct(field) (field validated per-command)
fn parse_aggregation(
    pipe: &str,
    command: &str,
    allowed_fields: &[&str],
) -> std::result::Result<Aggregation, String> {
    let pipe = pipe.trim().to_lowercase();

    if pipe == "count" {
        return Ok(Aggregation::CountAll);
    }

    // count(distinct field)
    if let Some(inner) = pipe
        .strip_prefix("count(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let inner = inner.trim();
        if let Some(field) = inner.strip_prefix("distinct").map(|s| s.trim()) {
            if field.is_empty() {
                return Err("count(distinct ...) requires a field name".to_string());
            }
            if !allowed_fields.contains(&field) {
                return Err(format!(
                    "field '{}' not allowed for '{}' (allowed: {})",
                    field,
                    command,
                    allowed_fields.join(", ")
                ));
            }
            return Ok(Aggregation::CountDistinct(field.to_string()));
        }
    }

    Err(format!(
        "unknown aggregation '{}' (allowed: count, count(distinct <field>))",
        pipe
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assay_query_basic() {
        let parsed = parse_assay_query(r#"callers --pattern "insert_event""#).unwrap();
        assert_eq!(parsed.command, "callers");
        assert_eq!(parsed.pattern, "insert_event");
        assert_eq!(parsed.aggregation, Aggregation::CountAll);
    }

    #[test]
    fn test_parse_assay_query_with_count_pipe() {
        let parsed = parse_assay_query(r#"callers --pattern "insert_event" | count"#).unwrap();
        assert_eq!(parsed.command, "callers");
        assert_eq!(parsed.pattern, "insert_event");
        assert_eq!(parsed.aggregation, Aggregation::CountAll);
    }

    #[test]
    fn test_parse_assay_query_with_count_distinct() {
        let parsed =
            parse_assay_query(r#"callers --pattern "insert_event" | count(distinct file)"#)
                .unwrap();
        assert_eq!(parsed.command, "callers");
        assert_eq!(parsed.pattern, "insert_event");
        assert_eq!(
            parsed.aggregation,
            Aggregation::CountDistinct("file".to_string())
        );
    }

    #[test]
    fn test_parse_assay_query_functions() {
        let parsed = parse_assay_query(r#"functions --pattern "migration""#).unwrap();
        assert_eq!(parsed.command, "functions");
        assert_eq!(parsed.pattern, "migration");
    }

    #[test]
    fn test_parse_assay_query_importers() {
        let parsed = parse_assay_query(r#"importers --pattern "commands""#).unwrap();
        assert_eq!(parsed.command, "importers");
        assert_eq!(parsed.pattern, "commands");
    }

    #[test]
    fn test_parse_assay_query_unknown_command() {
        let err = parse_assay_query(r#"unknown --pattern "test""#).unwrap_err();
        assert!(err.contains("unknown assay command"));
    }

    #[test]
    fn test_parse_assay_query_missing_pattern() {
        let err = parse_assay_query("callers").unwrap_err();
        assert!(err.contains("--pattern"));
    }

    #[test]
    fn test_parse_assay_query_empty_pattern() {
        let err = parse_assay_query(r#"callers --pattern """#).unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn test_parse_assay_query_invalid_field() {
        let err = parse_assay_query(r#"callers --pattern "test" | count(distinct nonexistent)"#)
            .unwrap_err();
        assert!(err.contains("not allowed"));
    }

    #[test]
    fn test_parse_assay_query_invalid_aggregation() {
        let err = parse_assay_query(r#"callers --pattern "test" | sum"#).unwrap_err();
        assert!(err.contains("unknown aggregation"));
    }

    #[test]
    fn test_parse_assay_functions_distinct_file() {
        let parsed =
            parse_assay_query(r#"functions --pattern "test" | count(distinct file)"#).unwrap();
        assert_eq!(
            parsed.aggregation,
            Aggregation::CountDistinct("file".to_string())
        );
    }

    #[test]
    fn test_parse_assay_importers_distinct_file() {
        let parsed =
            parse_assay_query(r#"importers --pattern "test" | count(distinct file)"#).unwrap();
        assert_eq!(
            parsed.aggregation,
            Aggregation::CountDistinct("file".to_string())
        );
    }

    #[test]
    fn test_parse_assay_importers_rejects_bad_field() {
        let err =
            parse_assay_query(r#"importers --pattern "test" | count(distinct name)"#).unwrap_err();
        assert!(err.contains("not allowed"));
    }

    // LIKE escaping tests

    #[test]
    fn test_escape_like_pattern_plain() {
        assert_eq!(escape_like_pattern("insert_event"), "insert\\_event");
    }

    #[test]
    fn test_escape_like_pattern_with_percent() {
        assert_eq!(escape_like_pattern("100%"), "100\\%");
    }

    #[test]
    fn test_escape_like_pattern_with_backslash() {
        assert_eq!(escape_like_pattern("path\\file"), "path\\\\file");
    }

    #[test]
    fn test_escape_like_pattern_normal_text() {
        assert_eq!(escape_like_pattern("commands"), "commands");
    }

    // SQL builder tests

    #[test]
    fn test_build_assay_sql_count_all() {
        let parsed = ParsedAssayQuery {
            command: "callers".to_string(),
            pattern: "insert_event".to_string(),
            aggregation: Aggregation::CountAll,
        };
        let (sql, params) = build_assay_sql(&parsed).unwrap();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("call_graph"));
        assert!(sql.contains("callee LIKE ?1 ESCAPE"));
        assert_eq!(params[0], "%insert\\_event%");
    }

    #[test]
    fn test_build_assay_sql_count_distinct() {
        let parsed = ParsedAssayQuery {
            command: "callers".to_string(),
            pattern: "insert_event".to_string(),
            aggregation: Aggregation::CountDistinct("file".to_string()),
        };
        let (sql, _) = build_assay_sql(&parsed).unwrap();
        assert!(sql.contains("COUNT(DISTINCT file)"));
    }

    #[test]
    fn test_build_assay_sql_functions() {
        let parsed = ParsedAssayQuery {
            command: "functions".to_string(),
            pattern: "migration".to_string(),
            aggregation: Aggregation::CountAll,
        };
        let (sql, _) = build_assay_sql(&parsed).unwrap();
        assert!(sql.contains("function_facts"));
        assert!(sql.contains("name LIKE ?1 ESCAPE"));
        assert!(sql.contains("file LIKE ?1 ESCAPE"));
    }

    #[test]
    fn test_build_assay_sql_importers() {
        let parsed = ParsedAssayQuery {
            command: "importers".to_string(),
            pattern: "commands".to_string(),
            aggregation: Aggregation::CountAll,
        };
        let (sql, _) = build_assay_sql(&parsed).unwrap();
        assert!(sql.contains("import_facts"));
        assert!(sql.contains("import_path LIKE ?1 ESCAPE"));
    }
}
