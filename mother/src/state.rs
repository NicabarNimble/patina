use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

use crate::{TaskIntent, TaskIntentKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Leased,
    Running,
    Succeeded,
    Failed,
    DeadLetter,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Leased => "leased",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::DeadLetter => "dead_letter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Running,
    Succeeded,
    Failed,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LakeCursorUpdate<'a> {
    pub lake_name: &'a str,
    pub source_name: &'a str,
    pub data_type: &'a str,
    pub cursor_value: Option<&'a str>,
    pub records_written: u64,
    pub status: &'a str,
    pub last_error: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub id: String,
    pub plugin_name: String,
    pub kind: TaskIntentKind,
    pub payload_json: String,
    pub dedupe_key: Option<String>,
    pub attempts: i64,
}

#[derive(Debug, Clone)]
pub struct KnowledgeRuntimeStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotherSessionStatus {
    Active,
    Completed,
    Archived,
}

impl MotherSessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    fn from_db(value: &str) -> Self {
        match value {
            "completed" => Self::Completed,
            "archived" => Self::Archived,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectUid(String);

impl ProjectUid {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            anyhow::bail!("project_uid must not be empty");
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PersonaUid(String);

impl PersonaUid {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            anyhow::bail!("persona_uid must not be empty");
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterfaceKindId(String);

impl InterfaceKindId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            anyhow::bail!("interface_kind must not be empty");
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct MotherSessionRecord {
    pub runtime_id: String,
    pub project_uid: String,
    pub file_id: String,
    pub title: String,
    pub persona_uid: Option<String>,
    pub status: MotherSessionStatus,
    pub interface_kind: String,
    pub adapter_name: String,
    pub branch: Option<String>,
    pub start_tag: Option<String>,
    pub end_tag: Option<String>,
    pub parent_runtime_id: Option<String>,
    pub handoff_from_runtime_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl MotherSessionRecord {
    pub fn starting_commit(&self) -> String {
        "none".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct MotherSessionParticipant {
    pub session_runtime_id: String,
    pub participant_id: String,
    pub role: String,
    pub interface_kind: Option<String>,
    pub adapter_name: Option<String>,
    pub display_name: Option<String>,
    pub joined_at: String,
    pub left_at: Option<String>,
}

impl Default for KnowledgeRuntimeStore {
    fn default() -> Self {
        let home = std::env::var_os("PATINA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".patina")
            });
        Self::new(home.join("mother").join("runtime.db"))
    }
}

impl KnowledgeRuntimeStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn open(&self) -> Result<Connection> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating mother runtime dir {}", parent.display()))?;
        }
        let conn = Connection::open(&self.path)
            .with_context(|| format!("opening mother runtime db {}", self.path.display()))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;")?;
        self.init_schema(&conn)?;
        Ok(conn)
    }

    fn init_schema(&self, conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS mother_child_state (
                plugin_name TEXT NOT NULL,
                key TEXT NOT NULL,
                value_json TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (plugin_name, key)
            );

            CREATE TABLE IF NOT EXISTS mother_child_checkpoints (
                plugin_name TEXT NOT NULL,
                stream TEXT NOT NULL,
                checkpoint_json TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (plugin_name, stream)
            );

            CREATE TABLE IF NOT EXISTS mother_child_subscriptions (
                plugin_name TEXT NOT NULL,
                stream TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (plugin_name, stream)
            );

            CREATE TABLE IF NOT EXISTS mother_child_offsets (
                plugin_name TEXT NOT NULL,
                stream TEXT NOT NULL,
                acked_offset INTEGER NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (plugin_name, stream)
            );

            CREATE TABLE IF NOT EXISTS mother_child_tasks (
                id TEXT PRIMARY KEY,
                plugin_name TEXT NOT NULL,
                intent_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                dedupe_key TEXT,
                status TEXT NOT NULL,
                lease_owner TEXT,
                lease_until TEXT,
                attempts INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_mother_child_tasks_dedupe
            ON mother_child_tasks (plugin_name, dedupe_key)
            WHERE dedupe_key IS NOT NULL;

            CREATE TABLE IF NOT EXISTS mother_child_runs (
                id INTEGER PRIMARY KEY,
                plugin_name TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                status TEXT NOT NULL,
                metrics_json TEXT,
                error TEXT
            );

            CREATE TABLE IF NOT EXISTS graph_mutation_log (
                seq INTEGER PRIMARY KEY,
                plugin_name TEXT NOT NULL,
                action TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS belief_mutation_log (
                seq INTEGER PRIMARY KEY,
                plugin_name TEXT NOT NULL,
                action TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mother_lake_cursors (
                lake_name TEXT NOT NULL,
                source_name TEXT NOT NULL,
                data_type TEXT NOT NULL,
                cursor_value TEXT,
                records_written INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                last_error TEXT,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (lake_name, source_name, data_type)
            );

            CREATE TABLE IF NOT EXISTS belief_verifications (
                id INTEGER PRIMARY KEY,
                belief_id TEXT NOT NULL,
                plugin_name TEXT NOT NULL,
                status TEXT NOT NULL,
                evidence_json TEXT,
                notes TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS belief_evidence (
                id INTEGER PRIMARY KEY,
                belief_id TEXT NOT NULL,
                plugin_name TEXT NOT NULL,
                evidence_id TEXT NOT NULL,
                payload_json TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS belief_relationships (
                belief_id TEXT NOT NULL,
                related_belief_id TEXT NOT NULL,
                relation TEXT NOT NULL,
                plugin_name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (belief_id, related_belief_id, relation, plugin_name)
            );

            CREATE TABLE IF NOT EXISTS mother_sessions (
                runtime_id TEXT PRIMARY KEY,
                project_uid TEXT NOT NULL,
                file_id TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                persona_uid TEXT,
                status TEXT NOT NULL,
                interface_kind TEXT NOT NULL,
                adapter_name TEXT NOT NULL,
                branch TEXT,
                start_tag TEXT,
                end_tag TEXT,
                parent_runtime_id TEXT,
                handoff_from_runtime_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_mother_sessions_project_status
            ON mother_sessions (project_uid, status, updated_at DESC);

            CREATE TABLE IF NOT EXISTS mother_session_participants (
                session_runtime_id TEXT NOT NULL,
                participant_id TEXT NOT NULL,
                role TEXT NOT NULL,
                interface_kind TEXT,
                adapter_name TEXT,
                display_name TEXT,
                joined_at TEXT NOT NULL,
                left_at TEXT,
                PRIMARY KEY (session_runtime_id, participant_id, joined_at)
            );

            CREATE TABLE IF NOT EXISTS mother_session_handoffs (
                id INTEGER PRIMARY KEY,
                from_runtime_id TEXT NOT NULL,
                to_runtime_id TEXT NOT NULL,
                reason TEXT,
                created_at TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub fn get_state(&self, plugin_name: &str, key: &str) -> Result<Option<String>> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT value_json FROM mother_child_state WHERE plugin_name = ?1 AND key = ?2",
            params![plugin_name, key],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn put_state(&self, plugin_name: &str, key: &str, value_json: &str) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO mother_child_state (plugin_name, key, value_json, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(plugin_name, key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at = excluded.updated_at
            "#,
            params![plugin_name, key, value_json, now],
        )?;
        Ok(())
    }

    pub fn delete_state(&self, plugin_name: &str, key: &str) -> Result<()> {
        let conn = self.open()?;
        conn.execute(
            "DELETE FROM mother_child_state WHERE plugin_name = ?1 AND key = ?2",
            params![plugin_name, key],
        )?;
        Ok(())
    }

    pub fn list_state_prefix(&self, plugin_name: &str, prefix: &str) -> Result<Vec<String>> {
        let conn = self.open()?;
        let like_pattern = format!("{prefix}%");
        let mut stmt = conn.prepare(
            "SELECT key FROM mother_child_state WHERE plugin_name = ?1 AND key LIKE ?2 ORDER BY key",
        )?;
        let keys = stmt
            .query_map(params![plugin_name, like_pattern], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(keys)
    }

    pub fn load_checkpoint(&self, plugin_name: &str, stream: &str) -> Result<Option<String>> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT checkpoint_json FROM mother_child_checkpoints WHERE plugin_name = ?1 AND stream = ?2",
            params![plugin_name, stream],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn save_checkpoint(
        &self,
        plugin_name: &str,
        stream: &str,
        checkpoint_json: &str,
    ) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO mother_child_checkpoints (plugin_name, stream, checkpoint_json, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(plugin_name, stream) DO UPDATE SET
                checkpoint_json = excluded.checkpoint_json,
                updated_at = excluded.updated_at
            "#,
            params![plugin_name, stream, checkpoint_json, now],
        )?;
        Ok(())
    }

    pub fn ensure_subscriptions(&self, plugin_name: &str, streams: &[String]) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        for stream in streams {
            conn.execute(
                "INSERT OR IGNORE INTO mother_child_subscriptions (plugin_name, stream, created_at) VALUES (?1, ?2, ?3)",
                params![plugin_name, stream, now],
            )?;
        }
        Ok(())
    }

    pub fn load_offset(&self, plugin_name: &str, stream: &str) -> Result<Option<u64>> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT acked_offset FROM mother_child_offsets WHERE plugin_name = ?1 AND stream = ?2",
            params![plugin_name, stream],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map(|opt| opt.map(|v| v as u64))
        .map_err(Into::into)
    }

    pub fn ack_offset(&self, plugin_name: &str, stream: &str, offset: u64) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO mother_child_offsets (plugin_name, stream, acked_offset, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(plugin_name, stream) DO UPDATE SET
                acked_offset = excluded.acked_offset,
                updated_at = excluded.updated_at
            "#,
            params![plugin_name, stream, offset as i64, now],
        )?;
        Ok(())
    }

    pub fn enqueue_task(&self, plugin_name: &str, intent: &TaskIntent) -> Result<String> {
        let conn = self.open()?;
        if let Some(dedupe_key) = intent.dedupe_key.as_deref() {
            if let Some(existing) = conn
                .query_row(
                    "SELECT id FROM mother_child_tasks WHERE plugin_name = ?1 AND dedupe_key = ?2 AND status IN ('queued', 'leased', 'running')",
                    params![plugin_name, dedupe_key],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                return Ok(existing);
            }

            conn.execute(
                "DELETE FROM mother_child_tasks WHERE plugin_name = ?1 AND dedupe_key = ?2 AND status IN ('succeeded', 'dead_letter')",
                params![plugin_name, dedupe_key],
            )?;
        }

        let now = Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            r#"
            INSERT INTO mother_child_tasks (
                id, plugin_name, intent_type, payload_json, dedupe_key,
                status, lease_until, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?7)
            "#,
            params![
                id,
                plugin_name,
                intent.kind.as_str(),
                serde_json::to_string(&intent.payload)?,
                intent.dedupe_key,
                TaskStatus::Queued.as_str(),
                now
            ],
        )?;
        Ok(id)
    }

    pub fn lease_next_task(
        &self,
        plugin_name: &str,
        lease_owner: &str,
    ) -> Result<Option<QueuedTask>> {
        let conn = self.open()?;
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, intent_type, payload_json, dedupe_key, attempts
            FROM mother_child_tasks
            WHERE plugin_name = ?1
              AND (
                    (status = 'queued' AND (lease_until IS NULL OR lease_until <= ?2))
                 OR (status = 'leased' AND lease_until <= ?2)
              )
            ORDER BY created_at
            LIMIT 1
            "#,
        )?;

        let task = stmt
            .query_row(params![plugin_name, now_str], |row| {
                Ok(QueuedTask {
                    id: row.get(0)?,
                    plugin_name: plugin_name.to_string(),
                    kind: TaskIntentKind::parse(&row.get::<_, String>(1)?)
                        .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                    payload_json: row.get(2)?,
                    dedupe_key: row.get(3)?,
                    attempts: row.get(4)?,
                })
            })
            .optional()?;

        let Some(mut task) = task else {
            return Ok(None);
        };

        let lease_until = (now + Duration::seconds(60)).to_rfc3339();
        conn.execute(
            r#"
            UPDATE mother_child_tasks
            SET status = ?2,
                lease_owner = ?3,
                lease_until = ?4,
                attempts = attempts + 1,
                updated_at = ?5
            WHERE id = ?1
            "#,
            params![
                task.id,
                TaskStatus::Leased.as_str(),
                lease_owner,
                lease_until,
                now_str
            ],
        )?;
        task.attempts += 1;
        Ok(Some(task))
    }

    pub fn mark_task_running(&self, task_id: &str) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE mother_child_tasks SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![task_id, TaskStatus::Running.as_str(), now],
        )?;
        Ok(())
    }

    pub fn mark_task_succeeded(&self, task_id: &str) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            UPDATE mother_child_tasks
            SET status = ?2,
                lease_owner = NULL,
                lease_until = NULL,
                last_error = NULL,
                updated_at = ?3
            WHERE id = ?1
            "#,
            params![task_id, TaskStatus::Succeeded.as_str(), now],
        )?;
        Ok(())
    }

    pub fn mark_task_failed(&self, task_id: &str, attempts: i64, error: &str) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now();
        let status = if attempts >= 5 {
            TaskStatus::DeadLetter
        } else {
            TaskStatus::Queued
        };
        let backoff_minutes = match attempts {
            0 | 1 => 1,
            2 => 5,
            3 => 15,
            4 => 60,
            _ => 360,
        };
        let retry_at = if status == TaskStatus::Queued {
            Some((now + Duration::minutes(backoff_minutes)).to_rfc3339())
        } else {
            None
        };
        conn.execute(
            r#"
            UPDATE mother_child_tasks
            SET status = ?2,
                lease_owner = NULL,
                lease_until = ?3,
                last_error = ?4,
                updated_at = ?5
            WHERE id = ?1
            "#,
            params![task_id, status.as_str(), retry_at, error, now.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn record_run_start(&self, plugin_name: &str) -> Result<i64> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO mother_child_runs (plugin_name, started_at, status) VALUES (?1, ?2, ?3)",
            params![plugin_name, now, RunStatus::Running.as_str()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn finish_run(
        &self,
        run_id: i64,
        status: RunStatus,
        metrics_json: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            UPDATE mother_child_runs
            SET finished_at = ?2,
                status = ?3,
                metrics_json = ?4,
                error = ?5
            WHERE id = ?1
            "#,
            params![run_id, now, status.as_str(), metrics_json, error],
        )?;
        Ok(())
    }

    pub fn save_lake_cursor(&self, update: &LakeCursorUpdate<'_>) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO mother_lake_cursors (
                lake_name, source_name, data_type, cursor_value,
                records_written, status, last_error, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(lake_name, source_name, data_type) DO UPDATE SET
                cursor_value = excluded.cursor_value,
                records_written = excluded.records_written,
                status = excluded.status,
                last_error = excluded.last_error,
                updated_at = excluded.updated_at
            "#,
            params![
                update.lake_name,
                update.source_name,
                update.data_type,
                update.cursor_value,
                update.records_written as i64,
                update.status,
                update.last_error,
                now
            ],
        )?;
        Ok(())
    }

    pub fn load_lake_cursor(
        &self,
        lake_name: &str,
        source_name: &str,
        data_type: &str,
    ) -> Result<Option<String>> {
        let conn = self.open()?;
        let value: Option<Option<String>> = conn
            .query_row(
                "SELECT cursor_value FROM mother_lake_cursors WHERE lake_name = ?1 AND source_name = ?2 AND data_type = ?3",
                params![lake_name, source_name, data_type],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.flatten())
    }

    pub fn record_graph_mutation(
        &self,
        plugin_name: &str,
        action: &str,
        payload_json: &str,
    ) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO graph_mutation_log (plugin_name, action, payload_json, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![plugin_name, action, payload_json, now],
        )?;
        Ok(())
    }

    pub fn record_belief_mutation(
        &self,
        plugin_name: &str,
        action: &str,
        payload_json: &str,
    ) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO belief_mutation_log (plugin_name, action, payload_json, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![plugin_name, action, payload_json, now],
        )?;
        Ok(())
    }

    pub fn record_belief_verification(
        &self,
        plugin_name: &str,
        belief_id: &str,
        status: &str,
        evidence_json: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO belief_verifications (belief_id, plugin_name, status, evidence_json, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![belief_id, plugin_name, status, evidence_json, notes, now],
        )?;
        Ok(())
    }

    pub fn attach_belief_evidence(
        &self,
        plugin_name: &str,
        belief_id: &str,
        evidence_id: &str,
        payload_json: Option<&str>,
    ) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO belief_evidence (belief_id, plugin_name, evidence_id, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![belief_id, plugin_name, evidence_id, payload_json, now],
        )?;
        Ok(())
    }

    pub fn link_beliefs(
        &self,
        plugin_name: &str,
        belief_id: &str,
        related_belief_id: &str,
        relation: &str,
    ) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO belief_relationships (belief_id, related_belief_id, relation, plugin_name, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![belief_id, related_belief_id, relation, plugin_name, now],
        )?;
        Ok(())
    }

    pub fn create_mother_session(
        &self,
        record: &MotherSessionRecord,
        participants: &[MotherSessionParticipant],
    ) -> Result<()> {
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        tx.execute(
            r#"
            INSERT INTO mother_sessions (
                runtime_id, project_uid, file_id, title, persona_uid, status,
                interface_kind, adapter_name, branch, start_tag, end_tag,
                parent_runtime_id, handoff_from_runtime_id, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                record.runtime_id,
                record.project_uid,
                record.file_id,
                record.title,
                record.persona_uid,
                record.status.as_str(),
                record.interface_kind,
                record.adapter_name,
                record.branch,
                record.start_tag,
                record.end_tag,
                record.parent_runtime_id,
                record.handoff_from_runtime_id,
                record.created_at,
                record.updated_at,
            ],
        )?;

        for participant in participants {
            tx.execute(
                r#"
                INSERT INTO mother_session_participants (
                    session_runtime_id, participant_id, role, interface_kind,
                    adapter_name, display_name, joined_at, left_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    participant.session_runtime_id,
                    participant.participant_id,
                    participant.role,
                    participant.interface_kind,
                    participant.adapter_name,
                    participant.display_name,
                    participant.joined_at,
                    participant.left_at,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_mother_session(&self, runtime_id: &str) -> Result<Option<MotherSessionRecord>> {
        let conn = self.open()?;
        conn.query_row(
            r#"
            SELECT runtime_id, project_uid, file_id, title, persona_uid, status,
                   interface_kind, adapter_name, branch, start_tag, end_tag,
                   parent_runtime_id, handoff_from_runtime_id, created_at, updated_at
            FROM mother_sessions
            WHERE runtime_id = ?1
            "#,
            params![runtime_id],
            map_mother_session_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn get_mother_session_by_file_id(
        &self,
        file_id: &str,
    ) -> Result<Option<MotherSessionRecord>> {
        let conn = self.open()?;
        conn.query_row(
            r#"
            SELECT runtime_id, project_uid, file_id, title, persona_uid, status,
                   interface_kind, adapter_name, branch, start_tag, end_tag,
                   parent_runtime_id, handoff_from_runtime_id, created_at, updated_at
            FROM mother_sessions
            WHERE file_id = ?1
            "#,
            params![file_id],
            map_mother_session_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn list_active_mother_sessions(
        &self,
        project_uid: &ProjectUid,
    ) -> Result<Vec<MotherSessionRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT runtime_id, project_uid, file_id, title, persona_uid, status,
                   interface_kind, adapter_name, branch, start_tag, end_tag,
                   parent_runtime_id, handoff_from_runtime_id, created_at, updated_at
            FROM mother_sessions
            WHERE project_uid = ?1 AND status = 'active'
            ORDER BY updated_at DESC, created_at DESC
            "#,
        )?;
        let rows = stmt
            .query_map(params![project_uid.as_str()], map_mother_session_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_active_mother_session_for_interface(
        &self,
        project_uid: &ProjectUid,
        adapter_name: &str,
        interface_kind: &InterfaceKindId,
        persona_uid: Option<&PersonaUid>,
    ) -> Result<Option<MotherSessionRecord>> {
        let conn = self.open()?;
        conn.query_row(
            r#"
            SELECT runtime_id, project_uid, file_id, title, persona_uid, status,
                   interface_kind, adapter_name, branch, start_tag, end_tag,
                   parent_runtime_id, handoff_from_runtime_id, created_at, updated_at
            FROM mother_sessions
            WHERE project_uid = ?1
              AND status = 'active'
              AND adapter_name = ?2
              AND interface_kind = ?3
              AND ((?4 IS NULL AND persona_uid IS NULL) OR persona_uid = ?4)
            ORDER BY updated_at DESC, created_at DESC
            LIMIT 1
            "#,
            params![
                project_uid.as_str(),
                adapter_name,
                interface_kind.as_str(),
                persona_uid.map(PersonaUid::as_str)
            ],
            map_mother_session_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn touch_mother_session(&self, runtime_id: &str, updated_at: &str) -> Result<()> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE mother_sessions SET updated_at = ?2 WHERE runtime_id = ?1",
            params![runtime_id, updated_at],
        )?;
        Ok(())
    }

    pub fn finish_mother_session(
        &self,
        runtime_id: &str,
        status: MotherSessionStatus,
        end_tag: Option<&str>,
        updated_at: &str,
    ) -> Result<()> {
        let conn = self.open()?;
        conn.execute(
            r#"
            UPDATE mother_sessions
            SET status = ?2,
                end_tag = COALESCE(?3, end_tag),
                updated_at = ?4
            WHERE runtime_id = ?1
            "#,
            params![runtime_id, status.as_str(), end_tag, updated_at],
        )?;
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

fn map_mother_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MotherSessionRecord> {
    let status: String = row.get(5)?;
    Ok(MotherSessionRecord {
        runtime_id: row.get(0)?,
        project_uid: row.get(1)?,
        file_id: row.get(2)?,
        title: row.get(3)?,
        persona_uid: row.get(4)?,
        status: MotherSessionStatus::from_db(&status),
        interface_kind: row.get(6)?,
        adapter_name: row.get(7)?,
        branch: row.get(8)?,
        start_tag: row.get(9)?,
        end_tag: row.get(10)?,
        parent_runtime_id: row.get(11)?,
        handoff_from_runtime_id: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> KnowledgeRuntimeStore {
        let path = std::env::temp_dir().join(format!(
            "patina-knowledge-runtime-{}.db",
            uuid::Uuid::new_v4()
        ));
        KnowledgeRuntimeStore::new(path)
    }

    #[test]
    fn state_checkpoints_and_offsets_are_namespaced_and_persistent() {
        let store = temp_store();

        store
            .put_state("ducklake", "source:one", r#"{"ok":true}"#)
            .unwrap();
        store
            .put_state("belief-verifier", "source:one", r#"{"ok":false}"#)
            .unwrap();
        store
            .save_checkpoint("ducklake", "ducklake.sync", r#"{"offset":7}"#)
            .unwrap();
        store
            .ensure_subscriptions("belief-verifier", &[String::from("belief.changed")])
            .unwrap();
        store
            .ack_offset("belief-verifier", "belief.changed", 42)
            .unwrap();

        let reopened = KnowledgeRuntimeStore::new(store.path().clone());
        assert_eq!(
            reopened
                .get_state("ducklake", "source:one")
                .unwrap()
                .as_deref(),
            Some(r#"{"ok":true}"#)
        );
        assert_eq!(
            reopened
                .get_state("belief-verifier", "source:one")
                .unwrap()
                .as_deref(),
            Some(r#"{"ok":false}"#)
        );
        assert_eq!(
            reopened.list_state_prefix("ducklake", "source:").unwrap(),
            vec!["source:one".to_string()]
        );
        assert_eq!(
            reopened
                .load_checkpoint("ducklake", "ducklake.sync")
                .unwrap()
                .as_deref(),
            Some(r#"{"offset":7}"#)
        );
        assert_eq!(
            reopened
                .load_offset("belief-verifier", "belief.changed")
                .unwrap(),
            Some(42)
        );
    }

    #[test]
    fn task_dedupe_and_leasing_work() {
        let store = temp_store();
        let intent = TaskIntent {
            kind: TaskIntentKind::FetchSource,
            payload: serde_json::json!({"source_id": "abc"}),
            dedupe_key: Some("ducklake:abc".into()),
        };

        let first = store.enqueue_task("ducklake", &intent).unwrap();
        let second = store.enqueue_task("ducklake", &intent).unwrap();
        assert_eq!(first, second);

        let leased = store
            .lease_next_task("ducklake", "worker-1")
            .unwrap()
            .expect("expected queued task");
        assert_eq!(leased.id, first);
        assert_eq!(leased.kind, TaskIntentKind::FetchSource);
        assert_eq!(leased.dedupe_key.as_deref(), Some("ducklake:abc"));
        assert_eq!(leased.attempts, 1);

        store.mark_task_running(&leased.id).unwrap();
        store.mark_task_succeeded(&leased.id).unwrap();
        assert!(store
            .lease_next_task("ducklake", "worker-2")
            .unwrap()
            .is_none());

        let after_success = store.enqueue_task("ducklake", &intent).unwrap();
        assert_ne!(after_success, first);
        let leased_again = store
            .lease_next_task("ducklake", "worker-3")
            .unwrap()
            .expect("expected re-enqueued task after terminal status");
        assert_eq!(leased_again.id, after_success);
    }

    #[test]
    fn lake_cursor_roundtrips_null_cursor_values() {
        let store = temp_store();
        store
            .save_lake_cursor(&LakeCursorUpdate {
                lake_name: "default",
                source_name: "github-lake-small",
                data_type: "issues",
                cursor_value: None,
                records_written: 0,
                status: "ok",
                last_error: None,
            })
            .unwrap();

        let cursor = store
            .load_lake_cursor("default", "github-lake-small", "issues")
            .unwrap();
        assert_eq!(cursor, None);
    }

    #[test]
    fn mother_sessions_support_parallel_active_records() {
        let store = temp_store();
        let created_at = Utc::now().to_rfc3339();

        let first = MotherSessionRecord {
            runtime_id: uuid::Uuid::new_v4().to_string(),
            project_uid: "proj-1234".to_string(),
            file_id: "20260311-100000-ABCD".to_string(),
            title: "OpenCode session".to_string(),
            persona_uid: None,
            status: MotherSessionStatus::Active,
            interface_kind: "opencode".to_string(),
            adapter_name: "opencode".to_string(),
            branch: Some("patina".to_string()),
            start_tag: Some("session-20260311-100000-ABCD-opencode-start".to_string()),
            end_tag: None,
            parent_runtime_id: None,
            handoff_from_runtime_id: None,
            created_at: created_at.clone(),
            updated_at: created_at.clone(),
        };
        let second = MotherSessionRecord {
            runtime_id: uuid::Uuid::new_v4().to_string(),
            project_uid: "proj-1234".to_string(),
            file_id: "20260311-100001-EFGH".to_string(),
            title: "Gemini session".to_string(),
            persona_uid: Some("persona-1".to_string()),
            status: MotherSessionStatus::Active,
            interface_kind: "gemini".to_string(),
            adapter_name: "gemini".to_string(),
            branch: Some("patina".to_string()),
            start_tag: Some("session-20260311-100001-EFGH-gemini-start".to_string()),
            end_tag: None,
            parent_runtime_id: None,
            handoff_from_runtime_id: None,
            created_at: created_at.clone(),
            updated_at: created_at.clone(),
        };

        store.create_mother_session(&first, &[]).unwrap();
        store.create_mother_session(&second, &[]).unwrap();

        let project_uid = ProjectUid::new("proj-1234").unwrap();
        let opencode_kind = InterfaceKindId::new("opencode").unwrap();
        let gemini_kind = InterfaceKindId::new("gemini").unwrap();
        let active = store.list_active_mother_sessions(&project_uid).unwrap();
        assert_eq!(active.len(), 2);
        let persona_uid = PersonaUid::new("persona-1").unwrap();
        let missing_persona_uid = PersonaUid::new("persona-missing").unwrap();
        assert!(store
            .find_active_mother_session_for_interface(
                &project_uid,
                "opencode",
                &opencode_kind,
                None,
            )
            .unwrap()
            .is_some());
        assert!(store
            .find_active_mother_session_for_interface(
                &project_uid,
                "gemini",
                &gemini_kind,
                Some(&persona_uid),
            )
            .unwrap()
            .is_some());
        assert!(store
            .find_active_mother_session_for_interface(
                &project_uid,
                "gemini",
                &gemini_kind,
                Some(&missing_persona_uid),
            )
            .unwrap()
            .is_none());

        store
            .finish_mother_session(
                &first.runtime_id,
                MotherSessionStatus::Archived,
                Some("session-20260311-100000-ABCD-opencode-end"),
                &Utc::now().to_rfc3339(),
            )
            .unwrap();

        let reloaded = store
            .get_mother_session(&first.runtime_id)
            .unwrap()
            .unwrap();
        assert_eq!(reloaded.status, MotherSessionStatus::Archived);
        assert_eq!(
            reloaded.end_tag.as_deref(),
            Some("session-20260311-100000-ABCD-opencode-end")
        );
    }
}
