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
// Assay DSL — types, command registry, parser
// ============================================================================

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
///
/// The verification engine builds `SELECT COUNT(*) FROM table WHERE condition`
/// directly, avoiding row fetching and the truncation problem entirely.
struct AssayCommandDef {
    table: &'static str,
    where_clause: &'static str,
    /// Fields allowed in `count(distinct <field>)` — validated per-command
    count_fields: &'static [&'static str],
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

/// Parsed assay query from DSL text.
#[derive(Debug, Clone)]
pub struct ParsedAssayQuery {
    pub command: String,
    pub pattern: String,
    pub aggregation: Aggregation,
}

/// Parsed temporal query from DSL text.
#[derive(Debug, Clone)]
pub struct ParsedTemporalQuery {
    pub summary_field: String,
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

/// Build counting SQL for an assay command.
///
/// Returns (sql, params) ready for rusqlite execution.
fn build_assay_sql(
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

/// Build SQL for a temporal summary field query.
///
/// Runs directly against commits/commit_files tables — self-contained,
/// no dependency on the moments table having been populated.
fn build_temporal_sql(field: &str) -> std::result::Result<String, String> {
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

// ============================================================================
// Safety validation
// ============================================================================

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
            parse_assay_query(&query.query_text)?;
            Ok(())
        }
        "temporal" => {
            // Full parse validates derive-moments command and summary field
            parse_temporal_query(&query.query_text)?;
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

// ============================================================================
// Execution
// ============================================================================

/// Execute a single verification query and compare against expectation.
///
/// Dispatches to the appropriate executor based on query type.
pub fn execute_verification_query(
    conn: &Connection,
    query: &VerificationQuery,
) -> VerificationResult {
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
    let parsed = match parse_assay_query(&query.query_text) {
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
    let (sql, params) = match build_assay_sql(&parsed) {
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
    let parsed = match parse_temporal_query(&query.query_text) {
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

    let sql = match build_temporal_sql(&parsed.summary_field) {
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

/// Run all verification queries for a belief.
///
/// Executes SQL, assay, and temporal queries. Each type dispatches to its
/// own executor through execute_verification_query.
pub fn run_verification_queries(
    conn: &Connection,
    belief_id: &str,
    queries: &[VerificationQuery],
    data_freshness: &str,
) -> (Vec<VerificationResult>, VerificationAggregates) {
    let mut results = Vec::new();
    let mut aggregates = VerificationAggregates::default();

    for query in queries {
        let result = execute_verification_query(conn, query);

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

    // ====================================================================
    // Assay DSL parsing tests
    // ====================================================================

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
        // functions allows distinct on "file"
        let parsed =
            parse_assay_query(r#"functions --pattern "test" | count(distinct file)"#).unwrap();
        assert_eq!(
            parsed.aggregation,
            Aggregation::CountDistinct("file".to_string())
        );
    }

    #[test]
    fn test_parse_assay_importers_distinct_file() {
        // importers only allows "file"
        let parsed =
            parse_assay_query(r#"importers --pattern "test" | count(distinct file)"#).unwrap();
        assert_eq!(
            parsed.aggregation,
            Aggregation::CountDistinct("file".to_string())
        );
    }

    #[test]
    fn test_parse_assay_importers_rejects_bad_field() {
        // importers does NOT allow "name"
        let err =
            parse_assay_query(r#"importers --pattern "test" | count(distinct name)"#).unwrap_err();
        assert!(err.contains("not allowed"));
    }

    // ====================================================================
    // LIKE escaping tests
    // ====================================================================

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

    // ====================================================================
    // Temporal parsing tests
    // ====================================================================

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

    // ====================================================================
    // Assay SQL builder tests
    // ====================================================================

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
        // Both LIKE clauses have their own ESCAPE
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

    // ====================================================================
    // Temporal SQL builder tests
    // ====================================================================

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

    // ====================================================================
    // Assay execution tests (in-memory DB)
    // ====================================================================

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

    // ====================================================================
    // Temporal execution tests (in-memory DB)
    // ====================================================================

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

    // ====================================================================
    // Safety validation tests for assay/temporal
    // ====================================================================

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

    // ====================================================================
    // Verification block parsing with assay/temporal types
    // ====================================================================

    #[test]
    fn test_parse_verification_blocks_mixed_types() {
        let content = r#"## Verification

```verify type="sql" label="caller count" expect=">= 20"
SELECT COUNT(*) FROM call_graph WHERE callee LIKE '%insert_event%'
```

```verify type="assay" label="callers across files" expect=">= 5"
callers --pattern "insert_event" | count(distinct file)
```

```verify type="temporal" label="commit frequency" expect=">= 1000"
derive-moments | summary.total_commits
```
"#;

        let queries = parse_verification_blocks(content);
        assert_eq!(queries.len(), 3);
        assert_eq!(queries[0].query_type, "sql");
        assert_eq!(queries[1].query_type, "assay");
        assert_eq!(
            queries[1].query_text.trim(),
            r#"callers --pattern "insert_event" | count(distinct file)"#
        );
        assert_eq!(queries[2].query_type, "temporal");
        assert_eq!(
            queries[2].query_text.trim(),
            "derive-moments | summary.total_commits"
        );
    }
}
