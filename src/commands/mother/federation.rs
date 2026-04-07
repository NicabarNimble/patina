use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use rusqlite::{params, OptionalExtension};
use std::collections::HashSet;
use std::path::Path;

const EXPECTED_PROJECT_SCHEMA_MAJOR: u32 = 3;
const DUCKLAKE_INSTALL_DIAGNOSTIC: &str =
    "DuckLake extension not installed — run: patina mother federation install-extensions";
const DEFAULT_QUERY_LIMIT: usize = 1000;
const MAX_QUERY_LIMIT: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FederationAvailability {
    Available,
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectAttachState {
    Attached,
    Failed,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAttachStatus {
    pub uid: String,
    pub alias: String,
    pub state: ProjectAttachState,
    pub reason: Option<String>,
    pub schema_version_major: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationStatus {
    pub availability: FederationAvailability,
    pub ducklake_loaded: bool,
    pub projects: Vec<ProjectAttachStatus>,
}

impl FederationStatus {
    pub fn attached_count(&self) -> usize {
        self.projects
            .iter()
            .filter(|entry| entry.state == ProjectAttachState::Attached)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.projects
            .iter()
            .filter(|entry| entry.state == ProjectAttachState::Failed)
            .count()
    }

    pub fn stale_count(&self) -> usize {
        self.projects
            .iter()
            .filter(|entry| entry.state == ProjectAttachState::Stale)
            .count()
    }
}

pub struct FederationRuntime {
    connection: Option<duckdb::Connection>,
    status: FederationStatus,
    allowed_tables: Vec<String>,
}

impl FederationRuntime {
    pub fn status(&self) -> &FederationStatus {
        &self.status
    }

    pub fn connection(&self) -> Option<&duckdb::Connection> {
        self.connection.as_ref()
    }

    pub fn allowed_tables(&self) -> &[String] {
        &self.allowed_tables
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FederationQueryError {
    Invalid { reason: String },
}

impl FederationQueryError {
    fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid {
            reason: reason.into(),
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            Self::Invalid { reason } => reason,
        }
    }
}

pub fn validate_query(sql: &str) -> std::result::Result<(), FederationQueryError> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(FederationQueryError::invalid("query must not be empty"));
    }
    if trimmed.contains(';') {
        return Err(FederationQueryError::invalid(
            "multiple statements are not allowed",
        ));
    }

    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("select") && !lower.starts_with("with") {
        return Err(FederationQueryError::invalid(
            "only SELECT statements allowed",
        ));
    }
    Ok(())
}

pub fn enforce_limit(sql: &str, limit: usize) -> String {
    let clamped = limit.clamp(1, MAX_QUERY_LIMIT);
    let effective = if limit == 0 {
        DEFAULT_QUERY_LIMIT
    } else {
        clamped
    };
    let limit_re = Regex::new(r"(?i)\blimit\s+(\d+)\b").expect("valid LIMIT regex");

    if let Some(captures) = limit_re.captures(sql) {
        if let Some(raw_limit) = captures.get(1).map(|v| v.as_str()) {
            if let Ok(parsed) = raw_limit.parse::<usize>() {
                let bounded = parsed.min(MAX_QUERY_LIMIT);
                let replacement = format!("LIMIT {}", bounded);
                return limit_re.replacen(sql, 1, replacement).to_string();
            }
        }
    }

    format!("{} LIMIT {}", sql.trim_end(), effective)
}

pub fn check_table_allowlist(
    sql: &str,
    allowed: &[String],
) -> std::result::Result<(), FederationQueryError> {
    let allowed_set: HashSet<String> = allowed.iter().map(|s| s.to_ascii_lowercase()).collect();
    for table in extract_table_references(sql) {
        if !allowed_set.contains(&table.to_ascii_lowercase()) {
            return Err(FederationQueryError::invalid(format!(
                "table '{}' not in federation allowlist",
                table
            )));
        }
    }
    Ok(())
}

pub fn startup(runtime_store: &patina::mother::KnowledgeRuntimeStore) -> FederationRuntime {
    let federation_path = patina::paths::mother::federation_db();
    if let Some(parent) = federation_path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            emit_open_failure("create_parent_dir");
            let reason = format!(
                "failed to create federation directory {}: {}",
                parent.display(),
                error
            );
            tracing::warn!(%reason, "federation unavailable");
            return unavailable(reason);
        }
    }

    let connection = match duckdb::Connection::open(&federation_path)
        .with_context(|| format!("opening federation db {}", federation_path.display()))
    {
        Ok(conn) => conn,
        Err(error) => {
            emit_open_failure("open_db");
            let reason = error.to_string();
            tracing::warn!(%reason, "federation unavailable");
            return unavailable(reason);
        }
    };

    if let Err(error) = connection.execute_batch("LOAD ducklake") {
        emit_open_failure("load_ducklake");
        tracing::warn!(error = %error, diagnostic = %DUCKLAKE_INSTALL_DIAGNOSTIC, "federation unavailable");
        return unavailable(DUCKLAKE_INSTALL_DIAGNOSTIC.to_string());
    }

    let projects = match runtime_store.list_registered_projects() {
        Ok(projects) => projects,
        Err(error) => {
            emit_open_failure("read_project_registry");
            let reason = format!("failed to read project registry: {}", error);
            tracing::warn!(%reason, "federation unavailable");
            return unavailable(reason);
        }
    };

    let mut project_statuses = Vec::with_capacity(projects.len());
    for project in projects {
        let alias = format!("p_{}", project.project_uid);
        let patina_db = match patina::paths::mother::projects::patina_db(&project.project_uid) {
            Ok(path) => path,
            Err(error) => {
                emit_attach_failure("resolve_path");
                project_statuses.push(ProjectAttachStatus {
                    uid: project.project_uid,
                    alias,
                    state: ProjectAttachState::Failed,
                    reason: Some(error),
                    schema_version_major: None,
                });
                continue;
            }
        };

        if !patina_db.exists() {
            project_statuses.push(ProjectAttachStatus {
                uid: project.project_uid,
                alias,
                state: ProjectAttachState::Stale,
                reason: Some(format!(
                    "project database missing at {}",
                    patina_db.display()
                )),
                schema_version_major: None,
            });
            continue;
        }

        let schema_major = match read_project_schema_major(&patina_db) {
            Ok(major) => major,
            Err(error) => {
                emit_attach_failure("schema_unreadable");
                project_statuses.push(ProjectAttachStatus {
                    uid: project.project_uid,
                    alias,
                    state: ProjectAttachState::Failed,
                    reason: Some(format!("schema version unreadable: {}", error)),
                    schema_version_major: None,
                });
                continue;
            }
        };

        if schema_major != EXPECTED_PROJECT_SCHEMA_MAJOR {
            emit_attach_failure("schema_incompatible");
            project_statuses.push(ProjectAttachStatus {
                uid: project.project_uid.clone(),
                alias,
                state: ProjectAttachState::Failed,
                reason: Some(incompatible_schema_reason(
                    &project.project_uid,
                    schema_major,
                )),
                schema_version_major: Some(schema_major),
            });
            continue;
        }

        let attach_sql = format!(
            "ATTACH '{}' AS {} (TYPE SQLITE)",
            escape_sql_literal(&patina_db.to_string_lossy()),
            alias
        );
        if let Err(error) = connection.execute_batch(&attach_sql) {
            emit_attach_failure("attach_error");
            project_statuses.push(ProjectAttachStatus {
                uid: project.project_uid,
                alias,
                state: ProjectAttachState::Failed,
                reason: Some(format!("attach failed: {}", error)),
                schema_version_major: Some(schema_major),
            });
            continue;
        }

        project_statuses.push(ProjectAttachStatus {
            uid: project.project_uid,
            alias,
            state: ProjectAttachState::Attached,
            reason: None,
            schema_version_major: Some(schema_major),
        });
    }

    emit_attach_count(
        project_statuses
            .iter()
            .filter(|entry| entry.state == ProjectAttachState::Attached)
            .count() as f64,
    );

    let allowed_tables = build_table_allowlist(&connection, &project_statuses);

    FederationRuntime {
        connection: Some(connection),
        status: FederationStatus {
            availability: FederationAvailability::Available,
            ducklake_loaded: true,
            projects: project_statuses,
        },
        allowed_tables,
    }
}

fn unavailable(reason: String) -> FederationRuntime {
    FederationRuntime {
        connection: None,
        status: FederationStatus {
            availability: FederationAvailability::Unavailable { reason },
            ducklake_loaded: false,
            projects: Vec::new(),
        },
        allowed_tables: Vec::new(),
    }
}

fn build_table_allowlist(
    connection: &duckdb::Connection,
    projects: &[ProjectAttachStatus],
) -> Vec<String> {
    let mut allowed = HashSet::new();
    for project in projects {
        if project.state != ProjectAttachState::Attached {
            continue;
        }

        let sql = format!(
            "SELECT name FROM {}.sqlite_master WHERE type = 'table' ORDER BY name",
            project.alias
        );
        let mut stmt = match connection.prepare(&sql) {
            Ok(stmt) => stmt,
            Err(error) => {
                tracing::warn!(alias = %project.alias, %error, "failed to prepare allowlist query");
                continue;
            }
        };

        let mut rows = match stmt.query([]) {
            Ok(rows) => rows,
            Err(error) => {
                tracing::warn!(alias = %project.alias, %error, "failed to query sqlite_master for allowlist");
                continue;
            }
        };

        while let Ok(Some(row)) = rows.next() {
            if let Ok(name) = row.get::<_, String>(0) {
                let lowered = name.to_ascii_lowercase();
                allowed.insert(lowered.clone());
                allowed.insert(format!(
                    "{}.{}",
                    project.alias.to_ascii_lowercase(),
                    lowered
                ));
            }
        }
    }

    let mut values: Vec<String> = allowed.into_iter().collect();
    values.sort();
    values
}

fn extract_table_references(sql: &str) -> Vec<String> {
    let table_re = Regex::new(r"(?i)\b(?:from|join)\s+([a-zA-Z_][\w]*(?:\.[a-zA-Z_][\w]*)?)")
        .expect("valid table extraction regex");
    table_re
        .captures_iter(sql)
        .filter_map(|caps| {
            caps.get(1)
                .map(|m| m.as_str().trim_matches('"').to_string())
        })
        .collect()
}

fn emit_open_failure(action: &str) {
    emit_metric("open_failure", "counter", 1.0, action);
}

fn emit_attach_failure(action: &str) {
    emit_metric("attach_failure", "counter", 1.0, action);
}

fn emit_attach_count(value: f64) {
    emit_metric("attach_count", "gauge", value, "attach_summary");
}

fn emit_metric(name: &str, kind: &str, value: f64, action: &str) {
    let events_path = match patina::eventlog::events_db_path() {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(metric = name, %error, "failed to resolve events path for federation metric");
            return;
        }
    };

    let conn = match rusqlite::Connection::open(&events_path) {
        Ok(conn) => conn,
        Err(error) => {
            tracing::warn!(metric = name, path = %events_path.display(), %error, "failed to open events db for federation metric");
            return;
        }
    };

    if let Err(error) = mother_crate::eventlog_schema::prepare_events_db(&conn) {
        tracing::warn!(metric = name, %error, "failed to initialize events schema for federation metric");
        return;
    }

    let payload = serde_json::json!({
        "name": format!("mother:federation:{}", name),
        "kind": kind,
        "value": value,
        "labels": [
            ["scope", "federation"],
            ["action", action],
        ],
        "source": "mother",
        "scope": "federation",
    });

    if let Err(error) = conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "measure.metric",
            Utc::now().to_rfc3339(),
            format!("mother:federation:{}", name),
            Option::<String>::None,
            payload.to_string(),
            "local"
        ],
    ) {
        tracing::warn!(metric = name, %error, "failed to emit federation metric");
    }
}

fn read_project_schema_major(db_path: &Path) -> Result<u32> {
    let conn = rusqlite::Connection::open(db_path)
        .with_context(|| format!("opening project db {}", db_path.display()))?;
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM scrape_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()
        .with_context(|| "reading schema_version from scrape_meta")?;

    match value {
        Some(raw) => parse_schema_major(&raw),
        None => Ok(0),
    }
}

fn incompatible_schema_reason(project_uid: &str, schema_major: u32) -> String {
    format!(
        "project {} schema v{} incompatible, expected v{} — run patina scrape to upgrade",
        project_uid, schema_major, EXPECTED_PROJECT_SCHEMA_MAJOR
    )
}

fn parse_schema_major(value: &str) -> Result<u32> {
    let major_text = value.trim().split('.').next().unwrap_or_default();
    major_text
        .parse::<u32>()
        .with_context(|| format!("invalid schema version '{}'", value))
}

fn escape_sql_literal(input: &str) -> String {
    input.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_schema_major_accepts_int_and_semver() {
        assert_eq!(parse_schema_major("3").unwrap(), 3);
        assert_eq!(parse_schema_major("3.1.0").unwrap(), 3);
    }

    #[test]
    fn parse_schema_major_rejects_invalid_value() {
        assert!(parse_schema_major("alpha").is_err());
    }

    #[test]
    fn escape_sql_literal_doubles_single_quotes() {
        assert_eq!(escape_sql_literal("a'b"), "a''b");
    }

    #[test]
    fn read_project_schema_major_returns_zero_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("patina.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE scrape_meta (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();

        assert_eq!(read_project_schema_major(&db_path).unwrap(), 0);
    }

    #[test]
    fn read_project_schema_major_reads_present_value() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("patina.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE scrape_meta (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO scrape_meta (key, value) VALUES (?1, ?2)",
            params!["schema_version", "3"],
        )
        .unwrap();

        assert_eq!(read_project_schema_major(&db_path).unwrap(), 3);
    }

    #[test]
    fn incompatible_schema_reason_includes_upgrade_guidance() {
        let message = incompatible_schema_reason("2bdc808e", 0);
        assert!(message.contains("schema v0 incompatible, expected v3"));
        assert!(message.contains("run patina scrape to upgrade"));
    }

    #[test]
    fn validate_query_accepts_select_and_cte() {
        assert!(validate_query("SELECT * FROM p_2bdc808e.scrape_meta").is_ok());
        assert!(validate_query("WITH t AS (SELECT 1) SELECT * FROM t").is_ok());
    }

    #[test]
    fn validate_query_rejects_non_select_statements() {
        assert!(validate_query("INSERT INTO x VALUES (1)").is_err());
        assert!(validate_query("DELETE FROM x").is_err());
        assert!(validate_query("DROP TABLE x").is_err());
    }

    #[test]
    fn enforce_limit_adds_default_and_clamps_large_values() {
        let with_default = enforce_limit("SELECT * FROM t", 0);
        assert!(with_default.to_ascii_lowercase().contains("limit 1000"));

        let clamped_existing = enforce_limit("SELECT * FROM t LIMIT 999999", 500);
        assert!(clamped_existing
            .to_ascii_lowercase()
            .contains("limit 10000"));
    }

    #[test]
    fn check_table_allowlist_rejects_unknown_references() {
        let allowed = vec![
            "p_2bdc808e.scrape_meta".to_string(),
            "scrape_meta".to_string(),
        ];
        assert!(check_table_allowlist("SELECT * FROM scrape_meta", &allowed).is_ok());
        assert!(check_table_allowlist("SELECT * FROM unknown_table", &allowed).is_err());
    }
}
