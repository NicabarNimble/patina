use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use std::path::Path;

const EXPECTED_PROJECT_SCHEMA_MAJOR: u32 = 3;
const DUCKLAKE_INSTALL_DIAGNOSTIC: &str =
    "DuckLake extension not installed — run: patina mother federation install-extensions";

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
    _connection: Option<duckdb::Connection>,
    status: FederationStatus,
}

impl FederationRuntime {
    pub fn status(&self) -> &FederationStatus {
        &self.status
    }
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

    FederationRuntime {
        _connection: Some(connection),
        status: FederationStatus {
            availability: FederationAvailability::Available,
            ducklake_loaded: true,
            projects: project_statuses,
        },
    }
}

fn unavailable(reason: String) -> FederationRuntime {
    FederationRuntime {
        _connection: None,
        status: FederationStatus {
            availability: FederationAvailability::Unavailable { reason },
            ducklake_loaded: false,
            projects: Vec::new(),
        },
    }
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
}
