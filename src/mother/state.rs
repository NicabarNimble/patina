use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

use super::{TaskIntent, TaskIntentKind};

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

impl Default for KnowledgeRuntimeStore {
    fn default() -> Self {
        Self::new(crate::paths::mother::runtime_db())
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

    pub fn save_checkpoint(&self, plugin_name: &str, stream: &str, checkpoint_json: &str) -> Result<()> {
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
                    "SELECT id FROM mother_child_tasks WHERE plugin_name = ?1 AND dedupe_key = ?2",
                    params![plugin_name, dedupe_key],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                return Ok(existing);
            }
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

    pub fn lease_next_task(&self, plugin_name: &str, lease_owner: &str) -> Result<Option<QueuedTask>> {
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

    pub fn save_lake_cursor(
        &self,
        lake_name: &str,
        source_name: &str,
        data_type: &str,
        cursor_value: Option<&str>,
        records_written: u64,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<()> {
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
                lake_name,
                source_name,
                data_type,
                cursor_value,
                records_written as i64,
                status,
                last_error,
                now
            ],
        )?;
        Ok(())
    }

    pub fn load_lake_cursor(&self, lake_name: &str, source_name: &str, data_type: &str) -> Result<Option<String>> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT cursor_value FROM mother_lake_cursors WHERE lake_name = ?1 AND source_name = ?2 AND data_type = ?3",
            params![lake_name, source_name, data_type],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn record_graph_mutation(&self, plugin_name: &str, action: &str, payload_json: &str) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO graph_mutation_log (plugin_name, action, payload_json, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![plugin_name, action, payload_json, now],
        )?;
        Ok(())
    }

    pub fn record_belief_mutation(&self, plugin_name: &str, action: &str, payload_json: &str) -> Result<()> {
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

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
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

        store.put_state("ducklake", "source:one", r#"{"ok":true}"#).unwrap();
        store.put_state("belief-verifier", "source:one", r#"{"ok":false}"#).unwrap();
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
            reopened.get_state("ducklake", "source:one").unwrap().as_deref(),
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
    }
}
