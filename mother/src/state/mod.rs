use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

use crate::{
    view_buffer::{
        self, Buffer, DisplayPattern, DisplayRequest, DisplayRequestOutcome, Frame,
        ObservabilityGap, ObservabilityImprovementArtifact, ShapeMatch, ViewDerivation,
        ViewMaturationEvent, ViewShape, ViewShapeAdaptation, ViewShapeCreation, ViewShapeRevision,
        Window,
    },
    TaskIntent, TaskIntentKind,
};

mod children_registry;
pub use children_registry::{
    ChildInstallRecord, ChildInstallUpdate, ChildRegistryAuditEventUpdate,
    ChildRegistryAuditRecord, ChildRegistryEntryRecord, ChildRegistryEntryUpdate,
    ChildRegistrySourceRecord, ChildRegistrySourceUpdate, ChildRegistryStore,
    ProjectChildAssignmentRecord, ProjectChildAssignmentUpdate,
};

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
pub struct MotherRuntimeStore {
    state_path: PathBuf,
    project_uid: Option<ProjectUid>,
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
pub struct VoiceUid(String);

impl VoiceUid {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            anyhow::bail!("voice_uid must not be empty");
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
    pub voice_uid: Option<String>,
    pub status: MotherSessionStatus,
    pub interface_kind: String,
    pub interface_name: String,
    pub branch: Option<String>,
    pub start_tag: Option<String>,
    pub end_tag: Option<String>,
    pub parent_runtime_id: Option<String>,
    pub handoff_from_runtime_id: Option<String>,
    pub starting_commit: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl MotherSessionRecord {
    pub fn starting_commit(&self) -> String {
        self.starting_commit
            .clone()
            .unwrap_or_else(|| "none".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct MotherSessionParticipant {
    pub session_runtime_id: String,
    pub participant_id: String,
    pub role: String,
    pub interface_kind: Option<String>,
    pub interface_name: Option<String>,
    pub display_name: Option<String>,
    pub joined_at: String,
    pub left_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRegistration {
    pub project_uid: String,
    pub project_path: String,
    pub registered_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBeliefStateRecord {
    pub project_uid: String,
    pub project_id: Option<String>,
    pub source_commit_sha: Option<String>,
    pub source_belief_count: Option<i64>,
    pub source_value_count: Option<i64>,
    pub source_fingerprint: Option<String>,
    pub source_last_activity: Option<String>,
    pub indexed_belief_count: Option<i64>,
    pub indexed_value_count: Option<i64>,
    pub indexed_fingerprint: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub last_verified_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBeliefStateUpdate {
    pub project_uid: String,
    pub project_id: Option<String>,
    pub source_commit_sha: Option<String>,
    pub source_belief_count: Option<i64>,
    pub source_value_count: Option<i64>,
    pub source_fingerprint: Option<String>,
    pub source_last_activity: Option<String>,
    pub indexed_belief_count: Option<i64>,
    pub indexed_value_count: Option<i64>,
    pub indexed_fingerprint: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupAttemptRecord {
    pub stage: String,
    pub status: String,
    pub error_excerpt: Option<String>,
    pub updated_at: String,
}

impl Default for MotherRuntimeStore {
    fn default() -> Self {
        let home = std::env::var_os("PATINA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".patina")
            });
        let project_uid = std::env::current_dir()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join(".patina/uid")).ok())
            .and_then(|raw| ProjectUid::new(raw.trim().to_string()).ok());
        Self {
            state_path: home.join("mother").join("state.db"),
            project_uid,
        }
    }
}

impl MotherRuntimeStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            state_path: path,
            project_uid: None,
        }
    }

    pub fn new_with_project(path: PathBuf, project_uid: ProjectUid) -> Self {
        Self {
            state_path: path,
            project_uid: Some(project_uid),
        }
    }

    fn open(&self) -> Result<Connection> {
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating mother state dir {}", parent.display()))?;
        }
        let conn = Connection::open(&self.state_path)
            .with_context(|| format!("opening mother state db {}", self.state_path.display()))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;")?;
        self.init_schema(&conn)?;
        Ok(conn)
    }

    fn open_project_runtime(&self) -> Result<Connection> {
        let project_uid = self
            .project_uid
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("project uid required for child runtime state"))?;
        let mother_root = self
            .state_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid mother state path"))?;
        let runtime_path = mother_root
            .join("projects")
            .join(project_uid.as_str())
            .join("runtime.db");

        if let Some(parent) = runtime_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating project runtime dir {}", parent.display()))?;
        }

        let conn = Connection::open(&runtime_path)
            .with_context(|| format!("opening project runtime db {}", runtime_path.display()))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;")?;
        self.init_project_runtime_schema(&conn)?;
        Ok(conn)
    }

    fn init_schema(&self, conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
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
                voice_uid TEXT,
                status TEXT NOT NULL,
                interface_kind TEXT NOT NULL,
                interface_name TEXT NOT NULL,
                branch TEXT,
                start_tag TEXT,
                end_tag TEXT,
                parent_runtime_id TEXT,
                handoff_from_runtime_id TEXT,
                starting_commit TEXT,
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
                interface_name TEXT,
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

            CREATE TABLE IF NOT EXISTS project_registry (
                project_uid TEXT PRIMARY KEY,
                project_path TEXT NOT NULL,
                registered_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mother_startup_attempts (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                stage TEXT NOT NULL,
                status TEXT NOT NULL,
                error_excerpt TEXT,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mother_users (
                user_id TEXT PRIMARY KEY,
                user_handle TEXT NOT NULL UNIQUE,
                display_name TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK (status IN ('active', 'suspended', 'revoked'))
            );

            CREATE TABLE IF NOT EXISTS mother_nodes (
                node_id TEXT PRIMARY KEY,
                node_slug TEXT NOT NULL UNIQUE,
                hostname TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK (status IN ('active', 'maintenance', 'retired'))
            );

            CREATE TABLE IF NOT EXISTS mother_node_memberships (
                node_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                role TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (node_id, user_id),
                CHECK (role IN ('full_admin', 'admin', 'member')),
                CHECK (status IN ('active', 'disabled')),
                FOREIGN KEY (node_id) REFERENCES mother_nodes(node_id),
                FOREIGN KEY (user_id) REFERENCES mother_users(user_id)
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_mother_node_single_full_admin
            ON mother_node_memberships(node_id)
            WHERE role = 'full_admin' AND status = 'active';

            CREATE TABLE IF NOT EXISTS mother_visions (
                vision_id TEXT PRIMARY KEY,
                vision_slug TEXT NOT NULL UNIQUE,
                owner_user_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK (status IN ('active', 'archived')),
                FOREIGN KEY (owner_user_id) REFERENCES mother_users(user_id)
            );

            CREATE TABLE IF NOT EXISTS mother_vision_memberships (
                vision_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                role TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (vision_id, user_id),
                CHECK (role IN ('admin', 'member')),
                CHECK (status IN ('active', 'disabled')),
                FOREIGN KEY (vision_id) REFERENCES mother_visions(vision_id),
                FOREIGN KEY (user_id) REFERENCES mother_users(user_id)
            );

            CREATE TABLE IF NOT EXISTS mother_node_visions (
                node_id TEXT NOT NULL,
                vision_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (node_id, vision_id),
                CHECK (status IN ('active', 'disabled')),
                FOREIGN KEY (node_id) REFERENCES mother_nodes(node_id),
                FOREIGN KEY (vision_id) REFERENCES mother_visions(vision_id)
            );

            CREATE TABLE IF NOT EXISTS mother_project_identities (
                project_uid TEXT PRIMARY KEY,
                project_id TEXT NOT NULL UNIQUE,
                user_id TEXT NOT NULL,
                vision_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                project_path TEXT NOT NULL,
                registered_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK (status IN ('active', 'archived')),
                FOREIGN KEY (project_uid) REFERENCES project_registry(project_uid),
                FOREIGN KEY (user_id) REFERENCES mother_users(user_id),
                FOREIGN KEY (vision_id) REFERENCES mother_visions(vision_id),
                FOREIGN KEY (node_id) REFERENCES mother_nodes(node_id)
            );

            CREATE INDEX IF NOT EXISTS idx_mother_project_identities_vision
            ON mother_project_identities(vision_id, status, updated_at DESC);

            CREATE TABLE IF NOT EXISTS mother_project_belief_state (
                project_uid TEXT PRIMARY KEY,
                project_id TEXT,
                source_commit_sha TEXT,
                source_belief_count INTEGER,
                source_value_count INTEGER,
                source_fingerprint TEXT,
                source_last_activity TEXT,
                indexed_belief_count INTEGER,
                indexed_value_count INTEGER,
                indexed_fingerprint TEXT,
                status TEXT NOT NULL DEFAULT 'unknown',
                last_error TEXT,
                last_verified_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (project_uid) REFERENCES project_registry(project_uid),
                FOREIGN KEY (project_id) REFERENCES mother_project_identities(project_id)
            );

            CREATE INDEX IF NOT EXISTS idx_mother_project_belief_state_status
            ON mother_project_belief_state(status, updated_at DESC);

            CREATE TABLE IF NOT EXISTS mother_child_sources (
                source_id TEXT PRIMARY KEY,
                provider_kind TEXT NOT NULL,
                provider_config_json TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_sync_at TEXT,
                last_sync_status TEXT,
                last_error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK (provider_kind IN ('github', 'gitea', 'custom')),
                CHECK (enabled IN (0, 1))
            );

            CREATE TABLE IF NOT EXISTS mother_child_registry_entries (
                entry_id TEXT PRIMARY KEY,
                child_name TEXT NOT NULL,
                version TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_release_ref TEXT NOT NULL,
                artifact_url TEXT NOT NULL,
                manifest_url TEXT NOT NULL,
                checksums_url TEXT,
                artifact_sha256 TEXT NOT NULL,
                manifest_sha256 TEXT NOT NULL,
                signature_ref TEXT,
                patina_min TEXT,
                operations_json TEXT,
                needs_toys_json TEXT,
                needs_scopes_json TEXT,
                state TEXT NOT NULL DEFAULT 'candidate',
                state_reason TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES mother_child_sources(source_id),
                CHECK (state IN ('candidate', 'approved', 'blocked', 'deprecated')),
                CHECK (LENGTH(TRIM(artifact_sha256)) > 0),
                CHECK (LENGTH(TRIM(manifest_sha256)) > 0)
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_mother_child_entry_name_version
            ON mother_child_registry_entries(child_name, version);

            CREATE INDEX IF NOT EXISTS idx_mother_child_entry_state
            ON mother_child_registry_entries(state, updated_at DESC);

            CREATE TABLE IF NOT EXISTS mother_child_installs (
                install_id TEXT PRIMARY KEY,
                entry_id TEXT NOT NULL,
                installed_name TEXT NOT NULL,
                installed_version TEXT NOT NULL,
                wasm_path TEXT NOT NULL,
                manifest_path TEXT NOT NULL,
                artifact_sha256_verified TEXT NOT NULL,
                manifest_sha256_verified TEXT NOT NULL,
                installed_at TEXT NOT NULL,
                installed_by TEXT,
                status TEXT NOT NULL DEFAULT 'installed',
                last_error TEXT,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (entry_id) REFERENCES mother_child_registry_entries(entry_id),
                CHECK (status IN ('installed', 'superseded', 'removed', 'failed')),
                CHECK (LENGTH(TRIM(artifact_sha256_verified)) > 0),
                CHECK (LENGTH(TRIM(manifest_sha256_verified)) > 0)
            );

            CREATE INDEX IF NOT EXISTS idx_mother_child_installs_status
            ON mother_child_installs(status, installed_at DESC);

            CREATE TABLE IF NOT EXISTS mother_project_child_assignments (
                assignment_id TEXT PRIMARY KEY,
                project_uid TEXT NOT NULL,
                project_id TEXT,
                child_name TEXT NOT NULL,
                entry_id TEXT NOT NULL,
                pinned_version TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                reason TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (project_uid) REFERENCES project_registry(project_uid),
                FOREIGN KEY (project_id) REFERENCES mother_project_identities(project_id),
                FOREIGN KEY (entry_id) REFERENCES mother_child_registry_entries(entry_id),
                CHECK (status IN ('active', 'revoked'))
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_mother_project_child_assignment_active
            ON mother_project_child_assignments(project_uid, child_name)
            WHERE status = 'active';

            CREATE INDEX IF NOT EXISTS idx_mother_project_child_assignment_project_status
            ON mother_project_child_assignments(project_uid, status, updated_at DESC);

            CREATE TABLE IF NOT EXISTS mother_child_registry_audit (
                id INTEGER PRIMARY KEY,
                event_kind TEXT NOT NULL,
                outcome TEXT NOT NULL,
                project_uid TEXT,
                child_name TEXT,
                entry_id TEXT,
                reason TEXT,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (project_uid) REFERENCES project_registry(project_uid),
                FOREIGN KEY (entry_id) REFERENCES mother_child_registry_entries(entry_id)
            );

            CREATE INDEX IF NOT EXISTS idx_mother_child_registry_audit_created
            ON mother_child_registry_audit(created_at DESC, id DESC);

            CREATE INDEX IF NOT EXISTS idx_project_registry_updated_at
            ON project_registry (updated_at DESC);

            CREATE TRIGGER IF NOT EXISTS trg_project_child_assignment_requires_approved_entry_insert
            BEFORE INSERT ON mother_project_child_assignments
            WHEN NEW.status = 'active'
            BEGIN
              SELECT CASE
                WHEN (
                  SELECT state FROM mother_child_registry_entries WHERE entry_id = NEW.entry_id
                ) <> 'approved'
                THEN RAISE(ABORT, 'project child assignment requires approved child registry entry')
              END;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_project_child_assignment_requires_approved_entry_update
            BEFORE UPDATE ON mother_project_child_assignments
            WHEN NEW.status = 'active'
            BEGIN
              SELECT CASE
                WHEN (
                  SELECT state FROM mother_child_registry_entries WHERE entry_id = NEW.entry_id
                ) <> 'approved'
                THEN RAISE(ABORT, 'project child assignment requires approved child registry entry')
              END;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_node_full_admin_guard_delete
            BEFORE DELETE ON mother_node_memberships
            WHEN OLD.role = 'full_admin' AND OLD.status = 'active'
            BEGIN
              SELECT CASE
                WHEN (
                  SELECT COUNT(*)
                  FROM mother_node_memberships
                  WHERE node_id = OLD.node_id AND role = 'full_admin' AND status = 'active'
                ) <= 1
                THEN RAISE(ABORT, 'node must retain at least one active full_admin')
              END;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_node_full_admin_guard_update
            BEFORE UPDATE ON mother_node_memberships
            WHEN OLD.role = 'full_admin' AND OLD.status = 'active'
              AND NOT (NEW.role = 'full_admin' AND NEW.status = 'active')
            BEGIN
              SELECT CASE
                WHEN (
                  SELECT COUNT(*)
                  FROM mother_node_memberships
                  WHERE node_id = OLD.node_id AND role = 'full_admin' AND status = 'active'
                ) <= 1
                THEN RAISE(ABORT, 'node must retain at least one active full_admin')
              END;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_node_vision_guard_delete
            BEFORE DELETE ON mother_node_visions
            WHEN OLD.status = 'active'
            BEGIN
              SELECT CASE
                WHEN (
                  SELECT COUNT(*)
                  FROM mother_node_visions
                  WHERE node_id = OLD.node_id AND status = 'active'
                ) <= 1
                THEN RAISE(ABORT, 'node must retain at least one active vision')
              END;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_node_vision_guard_update
            BEFORE UPDATE ON mother_node_visions
            WHEN OLD.status = 'active' AND NEW.status <> 'active'
            BEGIN
              SELECT CASE
                WHEN (
                  SELECT COUNT(*)
                  FROM mother_node_visions
                  WHERE node_id = OLD.node_id AND status = 'active'
                ) <= 1
                THEN RAISE(ABORT, 'node must retain at least one active vision')
              END;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_vision_admin_guard_delete
            BEFORE DELETE ON mother_vision_memberships
            WHEN OLD.role = 'admin' AND OLD.status = 'active'
            BEGIN
              SELECT CASE
                WHEN (
                  SELECT COUNT(*)
                  FROM mother_vision_memberships
                  WHERE vision_id = OLD.vision_id AND role = 'admin' AND status = 'active'
                ) <= 1
                THEN RAISE(ABORT, 'vision must retain at least one active admin')
              END;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_vision_admin_guard_update
            BEFORE UPDATE ON mother_vision_memberships
            WHEN OLD.role = 'admin' AND OLD.status = 'active'
              AND NOT (NEW.role = 'admin' AND NEW.status = 'active')
            BEGIN
              SELECT CASE
                WHEN (
                  SELECT COUNT(*)
                  FROM mother_vision_memberships
                  WHERE vision_id = OLD.vision_id AND role = 'admin' AND status = 'active'
                ) <= 1
                THEN RAISE(ABORT, 'vision must retain at least one active admin')
              END;
            END;
            "#,
        )?;

        let _ = conn.execute(
            "ALTER TABLE mother_sessions ADD COLUMN starting_commit TEXT",
            [],
        );
        if !column_exists(conn, "mother_sessions", "voice_uid")?
            && column_exists(conn, "mother_sessions", &["persona", "uid"].join("_"))?
        {
            let legacy_col = ["persona", "uid"].join("_");
            let sql = format!(
                "ALTER TABLE mother_sessions RENAME COLUMN {} TO voice_uid",
                legacy_col
            );
            let _ = conn.execute(&sql, []);
        }
        // Vocabulary migration: adapter_name → interface_name
        let _ = conn.execute(
            "ALTER TABLE mother_sessions RENAME COLUMN adapter_name TO interface_name",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE mother_session_participants RENAME COLUMN adapter_name TO interface_name",
            [],
        );
        view_buffer::store::init_schema(conn)?;
        Ok(())
    }

    fn init_project_runtime_schema(&self, conn: &Connection) -> Result<()> {
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
            "#,
        )?;
        Ok(())
    }

    pub fn register_project(&self, project_uid: &ProjectUid, project_path: &Path) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        let canonical =
            std::fs::canonicalize(project_path).unwrap_or_else(|_| project_path.to_path_buf());
        let path_text = canonical.to_string_lossy().to_string();

        let existing_path: Option<String> = conn
            .query_row(
                "SELECT project_path FROM project_registry WHERE project_uid = ?1",
                params![project_uid.as_str()],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(existing_path) = existing_path {
            let existing_canonical = std::fs::canonicalize(&existing_path)
                .unwrap_or_else(|_| PathBuf::from(&existing_path));
            if existing_canonical != canonical {
                anyhow::bail!(
                    "project_uid collision: {} already registered to {} (attempted {})",
                    project_uid.as_str(),
                    existing_path,
                    path_text
                );
            }
        }

        conn.execute(
            r#"
            INSERT INTO project_registry (project_uid, project_path, registered_at, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(project_uid) DO UPDATE SET
                updated_at = excluded.updated_at
            "#,
            params![project_uid.as_str(), path_text, now, now],
        )?;

        self.seed_project_identity_if_possible(&conn, project_uid, &path_text, &now)?;
        Ok(())
    }

    fn seed_project_identity_if_possible(
        &self,
        conn: &Connection,
        project_uid: &ProjectUid,
        project_path: &str,
        now: &str,
    ) -> Result<()> {
        let assignment = conn
            .query_row(
                r#"
                SELECT n.node_id, nm.user_id, nv.vision_id
                FROM mother_nodes n
                JOIN mother_node_memberships nm
                  ON nm.node_id = n.node_id
                 AND nm.role = 'full_admin'
                 AND nm.status = 'active'
                JOIN mother_node_visions nv
                  ON nv.node_id = n.node_id
                 AND nv.status = 'active'
                JOIN mother_vision_memberships vm
                  ON vm.vision_id = nv.vision_id
                 AND vm.user_id = nm.user_id
                 AND vm.role = 'admin'
                 AND vm.status = 'active'
                WHERE n.status = 'active'
                ORDER BY n.updated_at DESC, nv.updated_at DESC
                LIMIT 1
                "#,
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;

        let Some((node_id, user_id, vision_id)) = assignment else {
            return Ok(());
        };

        let project_id = conn
            .query_row(
                "SELECT project_id FROM mother_project_identities WHERE project_uid = ?1",
                params![project_uid.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| {
                format!(
                    "prj_{}",
                    uuid::Uuid::new_v4().simple().to_string().to_lowercase()
                )
            });

        conn.execute(
            r#"
            INSERT INTO mother_project_identities (
                project_uid, project_id, user_id, vision_id, node_id, status,
                project_path, registered_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?7)
            ON CONFLICT(project_uid) DO UPDATE SET
                project_id = excluded.project_id,
                user_id = excluded.user_id,
                vision_id = excluded.vision_id,
                node_id = excluded.node_id,
                status = excluded.status,
                project_path = excluded.project_path,
                updated_at = excluded.updated_at
            "#,
            params![
                project_uid.as_str(),
                project_id,
                user_id,
                vision_id,
                node_id,
                project_path,
                now,
            ],
        )?;

        Ok(())
    }

    pub fn record_startup_attempt(
        &self,
        stage: &str,
        status: &str,
        error_excerpt: Option<&str>,
    ) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();
        let excerpt = error_excerpt.map(|s| {
            let mut value = s.to_string();
            if value.len() > 280 {
                value.truncate(280);
            }
            value
        });

        conn.execute(
            r#"
            INSERT INTO mother_startup_attempts (id, stage, status, error_excerpt, updated_at)
            VALUES (1, ?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                stage = excluded.stage,
                status = excluded.status,
                error_excerpt = excluded.error_excerpt,
                updated_at = excluded.updated_at
            "#,
            params![stage, status, excerpt, now],
        )?;
        Ok(())
    }

    pub fn last_startup_failure(&self) -> Result<Option<StartupAttemptRecord>> {
        let conn = self.open()?;
        conn.query_row(
            r#"
            SELECT stage, status, error_excerpt, updated_at
            FROM mother_startup_attempts
            WHERE id = 1 AND status = 'failed'
            "#,
            [],
            |row| {
                Ok(StartupAttemptRecord {
                    stage: row.get(0)?,
                    status: row.get(1)?,
                    error_excerpt: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn list_registered_projects(&self) -> Result<Vec<ProjectRegistration>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT project_uid, project_path, registered_at, updated_at
            FROM project_registry
            ORDER BY updated_at DESC, project_uid ASC
            "#,
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ProjectRegistration {
                    project_uid: row.get(0)?,
                    project_path: row.get(1)?,
                    registered_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn upsert_project_belief_state(&self, update: &ProjectBeliefStateUpdate) -> Result<()> {
        let conn = self.open()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO mother_project_belief_state (
                project_uid,
                project_id,
                source_commit_sha,
                source_belief_count,
                source_value_count,
                source_fingerprint,
                source_last_activity,
                indexed_belief_count,
                indexed_value_count,
                indexed_fingerprint,
                status,
                last_error,
                last_verified_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
            ON CONFLICT(project_uid) DO UPDATE SET
                project_id = excluded.project_id,
                source_commit_sha = excluded.source_commit_sha,
                source_belief_count = excluded.source_belief_count,
                source_value_count = excluded.source_value_count,
                source_fingerprint = excluded.source_fingerprint,
                source_last_activity = excluded.source_last_activity,
                indexed_belief_count = excluded.indexed_belief_count,
                indexed_value_count = excluded.indexed_value_count,
                indexed_fingerprint = excluded.indexed_fingerprint,
                status = excluded.status,
                last_error = excluded.last_error,
                last_verified_at = excluded.last_verified_at,
                updated_at = excluded.updated_at
            "#,
            params![
                &update.project_uid,
                update.project_id.as_deref(),
                update.source_commit_sha.as_deref(),
                update.source_belief_count,
                update.source_value_count,
                update.source_fingerprint.as_deref(),
                update.source_last_activity.as_deref(),
                update.indexed_belief_count,
                update.indexed_value_count,
                update.indexed_fingerprint.as_deref(),
                &update.status,
                update.last_error.as_deref(),
                now,
            ],
        )?;

        Ok(())
    }

    pub fn get_project_belief_state(
        &self,
        project_uid: &str,
    ) -> Result<Option<ProjectBeliefStateRecord>> {
        let conn = self.open()?;
        conn.query_row(
            r#"
            SELECT project_uid, project_id, source_commit_sha,
                   source_belief_count, source_value_count,
                   source_fingerprint, source_last_activity,
                   indexed_belief_count, indexed_value_count,
                   indexed_fingerprint,
                   status, last_error, last_verified_at, updated_at
            FROM mother_project_belief_state
            WHERE project_uid = ?1
            "#,
            params![project_uid],
            |row| {
                Ok(ProjectBeliefStateRecord {
                    project_uid: row.get(0)?,
                    project_id: row.get(1)?,
                    source_commit_sha: row.get(2)?,
                    source_belief_count: row.get(3)?,
                    source_value_count: row.get(4)?,
                    source_fingerprint: row.get(5)?,
                    source_last_activity: row.get(6)?,
                    indexed_belief_count: row.get(7)?,
                    indexed_value_count: row.get(8)?,
                    indexed_fingerprint: row.get(9)?,
                    status: row.get(10)?,
                    last_error: row.get(11)?,
                    last_verified_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn get_state(&self, plugin_name: &str, key: &str) -> Result<Option<String>> {
        let conn = self.open_project_runtime()?;
        conn.query_row(
            "SELECT value_json FROM mother_child_state WHERE plugin_name = ?1 AND key = ?2",
            params![plugin_name, key],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn put_state(&self, plugin_name: &str, key: &str, value_json: &str) -> Result<()> {
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
        conn.execute(
            "DELETE FROM mother_child_state WHERE plugin_name = ?1 AND key = ?2",
            params![plugin_name, key],
        )?;
        Ok(())
    }

    pub fn list_state_prefix(&self, plugin_name: &str, prefix: &str) -> Result<Vec<String>> {
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE mother_child_tasks SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![task_id, TaskStatus::Running.as_str(), now],
        )?;
        Ok(())
    }

    pub fn mark_task_succeeded(&self, task_id: &str) -> Result<()> {
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
        let conn = self.open_project_runtime()?;
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
                runtime_id, project_uid, file_id, title, voice_uid, status,
                interface_kind, interface_name, branch, start_tag, end_tag,
                parent_runtime_id, handoff_from_runtime_id, starting_commit, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            "#,
            params![
                record.runtime_id,
                record.project_uid,
                record.file_id,
                record.title,
                record.voice_uid,
                record.status.as_str(),
                record.interface_kind,
                record.interface_name,
                record.branch,
                record.start_tag,
                record.end_tag,
                record.parent_runtime_id,
                record.handoff_from_runtime_id,
                record.starting_commit,
                record.created_at,
                record.updated_at,
            ],
        )?;

        for participant in participants {
            tx.execute(
                r#"
                INSERT INTO mother_session_participants (
                    session_runtime_id, participant_id, role, interface_kind,
                    interface_name, display_name, joined_at, left_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    participant.session_runtime_id,
                    participant.participant_id,
                    participant.role,
                    participant.interface_kind,
                    participant.interface_name,
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
            SELECT runtime_id, project_uid, file_id, title, voice_uid, status,
                   interface_kind, interface_name, branch, start_tag, end_tag,
                   parent_runtime_id, handoff_from_runtime_id, starting_commit, created_at, updated_at
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
            SELECT runtime_id, project_uid, file_id, title, voice_uid, status,
                   interface_kind, interface_name, branch, start_tag, end_tag,
                   parent_runtime_id, handoff_from_runtime_id, starting_commit, created_at, updated_at
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
            SELECT runtime_id, project_uid, file_id, title, voice_uid, status,
                   interface_kind, interface_name, branch, start_tag, end_tag,
                   parent_runtime_id, handoff_from_runtime_id, starting_commit, created_at, updated_at
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
        interface_name: &str,
        interface_kind: &InterfaceKindId,
        voice_uid: Option<&VoiceUid>,
    ) -> Result<Option<MotherSessionRecord>> {
        let conn = self.open()?;
        conn.query_row(
            r#"
            SELECT runtime_id, project_uid, file_id, title, voice_uid, status,
                   interface_kind, interface_name, branch, start_tag, end_tag,
                   parent_runtime_id, handoff_from_runtime_id, starting_commit, created_at, updated_at
            FROM mother_sessions
            WHERE project_uid = ?1
              AND status = 'active'
              AND interface_name = ?2
              AND interface_kind = ?3
              AND ((?4 IS NULL AND voice_uid IS NULL) OR voice_uid = ?4)
            ORDER BY updated_at DESC, created_at DESC
            LIMIT 1
            "#,
            params![
                project_uid.as_str(),
                interface_name,
                interface_kind.as_str(),
                voice_uid.map(VoiceUid::as_str)
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

    pub fn update_mother_session_title(
        &self,
        runtime_id: &str,
        title: &str,
        updated_at: &str,
    ) -> Result<()> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE mother_sessions SET title = ?2, updated_at = ?3 WHERE runtime_id = ?1",
            params![runtime_id, title, updated_at],
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

    pub fn save_view_display_request(&self, request: &DisplayRequest) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_display_request(&conn, request)
    }

    pub fn get_view_display_request(&self, request_id: &str) -> Result<Option<DisplayRequest>> {
        let conn = self.open()?;
        view_buffer::store::get_display_request(&conn, request_id)
    }

    pub fn list_view_display_requests(&self) -> Result<Vec<DisplayRequest>> {
        let conn = self.open()?;
        view_buffer::store::list_display_requests(&conn)
    }

    pub fn update_view_display_request_outcome(
        &self,
        request_id: &str,
        outcome: &DisplayRequestOutcome,
    ) -> Result<bool> {
        let conn = self.open()?;
        view_buffer::store::update_display_request_outcome(&conn, request_id, outcome)
    }

    pub fn save_view_shape_match(&self, shape_match: &ShapeMatch) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_shape_match(&conn, shape_match)
    }

    pub fn get_view_shape_match(&self, request_id: &str) -> Result<Option<ShapeMatch>> {
        let conn = self.open()?;
        view_buffer::store::get_shape_match(&conn, request_id)
    }

    pub fn list_view_shape_matches(&self) -> Result<Vec<ShapeMatch>> {
        let conn = self.open()?;
        view_buffer::store::list_shape_matches(&conn)
    }

    pub fn save_view_shape_adaptation(&self, adaptation: &ViewShapeAdaptation) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_shape_adaptation(&conn, adaptation)
    }

    pub fn get_view_shape_adaptation(
        &self,
        request_id: &str,
    ) -> Result<Option<ViewShapeAdaptation>> {
        let conn = self.open()?;
        view_buffer::store::get_shape_adaptation(&conn, request_id)
    }

    pub fn save_view_shape_creation(&self, creation: &ViewShapeCreation) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_shape_creation(&conn, creation)
    }

    pub fn get_view_shape_creation(&self, request_id: &str) -> Result<Option<ViewShapeCreation>> {
        let conn = self.open()?;
        view_buffer::store::get_shape_creation(&conn, request_id)
    }

    pub fn save_view_shape_revision(&self, revision: &ViewShapeRevision) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_shape_revision(&conn, revision)
    }

    pub fn get_view_shape_revision(&self, revision_id: &str) -> Result<Option<ViewShapeRevision>> {
        let conn = self.open()?;
        view_buffer::store::get_shape_revision(&conn, revision_id)
    }

    pub fn list_view_shape_revisions(&self) -> Result<Vec<ViewShapeRevision>> {
        let conn = self.open()?;
        view_buffer::store::list_shape_revisions(&conn)
    }

    pub fn upsert_view_shape(&self, shape: &ViewShape) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::upsert_shape(&conn, shape)
    }

    pub fn seed_view_shape(&self, shape: &ViewShape) -> Result<bool> {
        let conn = self.open()?;
        if view_buffer::store::get_shape(&conn, &shape.shape_id)?.is_some() {
            return Ok(false);
        }
        view_buffer::store::upsert_shape(&conn, shape)?;
        Ok(true)
    }

    pub fn get_view_shape(&self, shape_id: &str) -> Result<Option<ViewShape>> {
        let conn = self.open()?;
        view_buffer::store::get_shape(&conn, shape_id)
    }

    pub fn list_view_shapes(&self) -> Result<Vec<ViewShape>> {
        let conn = self.open()?;
        view_buffer::store::list_shapes(&conn)
    }

    pub fn deactivate_view_shape(&self, shape_id: &str) -> Result<bool> {
        let conn = self.open()?;
        view_buffer::store::deactivate_shape(&conn, shape_id)
    }

    pub fn upsert_view_derivation(&self, derivation: &ViewDerivation) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::upsert_derivation(&conn, derivation)
    }

    pub fn get_view_derivation(&self, derivation_id: &str) -> Result<Option<ViewDerivation>> {
        let conn = self.open()?;
        view_buffer::store::get_derivation(&conn, derivation_id)
    }

    pub fn list_view_derivations(&self) -> Result<Vec<ViewDerivation>> {
        let conn = self.open()?;
        view_buffer::store::list_derivations(&conn)
    }

    pub fn upsert_view_display_pattern(&self, pattern: &DisplayPattern) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::upsert_display_pattern(&conn, pattern)
    }

    pub fn get_view_display_pattern(&self, pattern_id: &str) -> Result<Option<DisplayPattern>> {
        let conn = self.open()?;
        view_buffer::store::get_display_pattern(&conn, pattern_id)
    }

    pub fn list_view_display_patterns(&self) -> Result<Vec<DisplayPattern>> {
        let conn = self.open()?;
        view_buffer::store::list_display_patterns(&conn)
    }

    pub fn save_view_maturation_event(&self, event: &ViewMaturationEvent) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_maturation_event(&conn, event)
    }

    pub fn get_view_maturation_event(
        &self,
        maturation_id: &str,
    ) -> Result<Option<ViewMaturationEvent>> {
        let conn = self.open()?;
        view_buffer::store::get_maturation_event(&conn, maturation_id)
    }

    pub fn list_view_maturation_events(&self) -> Result<Vec<ViewMaturationEvent>> {
        let conn = self.open()?;
        view_buffer::store::list_maturation_events(&conn)
    }

    pub fn save_view_observability_improvement(
        &self,
        artifact: &ObservabilityImprovementArtifact,
    ) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_observability_improvement(&conn, artifact)
    }

    pub fn get_view_observability_improvement(
        &self,
        artifact_id: &str,
    ) -> Result<Option<ObservabilityImprovementArtifact>> {
        let conn = self.open()?;
        view_buffer::store::get_observability_improvement(&conn, artifact_id)
    }

    pub fn list_view_observability_improvements(
        &self,
    ) -> Result<Vec<ObservabilityImprovementArtifact>> {
        let conn = self.open()?;
        view_buffer::store::list_observability_improvements(&conn)
    }

    pub fn save_view_buffer(&self, buffer: &Buffer) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_buffer(&conn, buffer)
    }

    pub fn list_view_buffers(&self) -> Result<Vec<Buffer>> {
        let conn = self.open()?;
        view_buffer::store::list_buffers(&conn)
    }

    pub fn save_view_frame(&self, frame: &Frame) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_frame(&conn, frame)
    }

    pub fn list_view_frames(&self) -> Result<Vec<Frame>> {
        let conn = self.open()?;
        view_buffer::store::list_frames(&conn)
    }

    pub fn save_view_window(&self, window: &Window) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_window(&conn, window)
    }

    pub fn list_view_windows(&self) -> Result<Vec<Window>> {
        let conn = self.open()?;
        view_buffer::store::list_windows(&conn)
    }

    pub fn save_view_observability_gap(&self, gap: &ObservabilityGap) -> Result<()> {
        let conn = self.open()?;
        view_buffer::store::save_gap(&conn, gap)
    }

    pub fn get_view_observability_gap(&self, gap_id: &str) -> Result<Option<ObservabilityGap>> {
        let conn = self.open()?;
        view_buffer::store::get_gap(&conn, gap_id)
    }

    pub fn list_view_observability_gaps(&self) -> Result<Vec<ObservabilityGap>> {
        let conn = self.open()?;
        view_buffer::store::list_gaps(&conn)
    }

    pub fn path(&self) -> &PathBuf {
        &self.state_path
    }
}

fn map_mother_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MotherSessionRecord> {
    let status: String = row.get(5)?;
    Ok(MotherSessionRecord {
        runtime_id: row.get(0)?,
        project_uid: row.get(1)?,
        file_id: row.get(2)?,
        title: row.get(3)?,
        voice_uid: row.get(4)?,
        status: MotherSessionStatus::from_db(&status),
        interface_kind: row.get(6)?,
        interface_name: row.get(7)?,
        branch: row.get(8)?,
        start_tag: row.get(9)?,
        end_tag: row.get(10)?,
        parent_runtime_id: row.get(11)?,
        handoff_from_runtime_id: row.get(12)?,
        starting_commit: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let pragma = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma)?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(names.iter().any(|name| name == column))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> MotherRuntimeStore {
        let root =
            std::env::temp_dir().join(format!("patina-knowledge-runtime-{}", uuid::Uuid::new_v4()));
        let path = root.join("mother/state.db");
        MotherRuntimeStore::new_with_project(path, ProjectUid::new("2bdc808e").unwrap())
    }

    #[test]
    fn view_display_requests_and_shape_matches_are_persistent() {
        // obligation: entity-state.DisplayRequest + entity-state.ShapeMatch
        // obligation: spec.mother-view-request-composer.mvrc2-request-persistence
        use crate::view_buffer::{
            DisplayRequest, DisplayRequestOutcome, ShapeMatch, ShapeMatchKind,
        };

        let store = temp_store();
        let requested_at = chrono::DateTime::parse_from_rfc3339("2026-05-09T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let request = DisplayRequest::pending(
            "req_1".to_string(),
            "local-user".to_string(),
            "pi".to_string(),
            "show mother status".to_string(),
            requested_at,
        );
        let shape_match = ShapeMatch {
            request_id: request.request_id.clone(),
            shape_id: Some("mother.status.default".to_string()),
            match_kind: ShapeMatchKind::ExplicitUserChoice,
            confidence: 1.0,
        };

        store.save_view_display_request(&request).unwrap();
        store.save_view_shape_match(&shape_match).unwrap();
        assert!(store
            .update_view_display_request_outcome(
                &request.request_id,
                &DisplayRequestOutcome::BufferOpened,
            )
            .unwrap());

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        let mut expected_request = request.clone();
        expected_request.outcome = DisplayRequestOutcome::BufferOpened;
        assert_eq!(
            reopened
                .get_view_display_request(&request.request_id)
                .unwrap(),
            Some(expected_request.clone())
        );
        assert_eq!(
            reopened.list_view_display_requests().unwrap(),
            vec![expected_request]
        );
        assert_eq!(
            reopened.get_view_shape_match(&request.request_id).unwrap(),
            Some(shape_match.clone())
        );
        assert_eq!(
            reopened.list_view_shape_matches().unwrap(),
            vec![shape_match]
        );
        assert!(!reopened
            .update_view_display_request_outcome("missing", &DisplayRequestOutcome::Unable)
            .unwrap());
    }

    #[test]
    fn view_request_ux_shape_artifacts_are_persistent() {
        // obligation: spec.mother-view-request-ux.mvru2-persist-request-artifacts
        use crate::view_buffer::{
            DisplayRequest, ViewRequirement, ViewShapeAdaptation, ViewShapeCreation,
        };

        let store = temp_store();
        let request = DisplayRequest::pending(
            "req_ux".to_string(),
            "local-user".to_string(),
            "pi".to_string(),
            "show runtime summary".to_string(),
            Utc::now(),
        );
        let adaptation = ViewShapeAdaptation::created_without_opening(
            request.request_id.clone(),
            "mother.status.default".to_string(),
            "mother.status.default::adapted::test".to_string(),
        );
        let creation = ViewShapeCreation::created_without_opening(
            request.request_id.clone(),
            "initial::req_ux::test".to_string(),
            vec![ViewRequirement {
                fact_path: "mother.status.version".to_string(),
                required: true,
                purpose: "display Mother version".to_string(),
            }],
        );

        store.save_view_display_request(&request).unwrap();
        store.save_view_shape_adaptation(&adaptation).unwrap();
        store.save_view_shape_creation(&creation).unwrap();

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        assert_eq!(
            reopened
                .get_view_shape_adaptation(&request.request_id)
                .unwrap(),
            Some(adaptation)
        );
        assert_eq!(
            reopened
                .get_view_shape_creation(&request.request_id)
                .unwrap(),
            Some(creation)
        );
    }

    #[test]
    fn view_buffer_revision_records_are_persistent() {
        // obligation: spec.mother-view-buffer-revision.mvbr5-persistence
        use crate::view_buffer::{
            Buffer, BufferState, ViewShapeRevision, ViewShapeRevisionOrigin,
            ViewShapeRevisionState, ViewShapeScope,
        };

        let store = temp_store();
        let shape = crate::view_buffer::mother_status_shape();
        let mut previous_buffer = Buffer::live_from_shape("buf_1".to_string(), &shape, Utc::now());
        previous_buffer.state = BufferState::Replaced;
        previous_buffer.replaced_at = Some(Utc::now());
        previous_buffer.replacement_buffer_id = Some("buf_2".to_string());
        let revision = ViewShapeRevision {
            revision_id: "rev_1".to_string(),
            user_id: "local-user".to_string(),
            agent_id: "pi".to_string(),
            previous_shape_id: shape.shape_id.clone(),
            revised_shape_id: "mother.status.default::revision::next".to_string(),
            previous_buffer_id: Some(previous_buffer.buffer_id.clone()),
            replacement_buffer_id: previous_buffer.replacement_buffer_id.clone(),
            revision_scope: ViewShapeScope::MotherUser,
            revision_origin: ViewShapeRevisionOrigin::UserCorrection,
            revision_state: ViewShapeRevisionState::Applied,
            reason: "show readiness first".to_string(),
            created_at: Utc::now(),
        };

        store.save_view_buffer(&previous_buffer).unwrap();
        store.save_view_shape_revision(&revision).unwrap();

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        assert_eq!(
            reopened
                .get_view_shape_revision(&revision.revision_id)
                .unwrap(),
            Some(revision.clone())
        );
        assert_eq!(
            reopened.list_view_shape_revisions().unwrap(),
            vec![revision]
        );
        assert_eq!(
            reopened
                .list_view_buffers()
                .unwrap()
                .first()
                .and_then(|buffer| buffer.replacement_buffer_id.as_deref()),
            Some("buf_2")
        );
    }

    #[test]
    fn view_maturation_artifacts_and_events_are_persistent() {
        // obligation: spec.mother-view-maturation.mvmat2-artifact-library
        // obligation: spec.mother-view-maturation.mvmat5-observability-improvement-artifact
        use crate::view_buffer::{
            DisplayPattern, DisplayPatternKind, ObservabilityImprovementArtifact, ViewDerivation,
            ViewMaturationEvent, ViewMaturationOrigin, ViewMaturationTargetKind, ViewShapeMaturity,
        };

        let store = temp_store();
        let derivation = ViewDerivation {
            derivation_id: "derivation_1".to_string(),
            shape_id: crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string(),
            label: "Memory Pressure Summary".to_string(),
            expression_ref: "allium://views/mother/status/memory-pressure".to_string(),
            input_fact_paths: vec!["mother.status.memory_pressure".to_string()],
            maturity: ViewShapeMaturity::Candidate,
        };
        let pattern = DisplayPattern {
            pattern_id: "pattern_1".to_string(),
            shape_id: crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string(),
            pattern_kind: DisplayPatternKind::Grouping,
            maturity: ViewShapeMaturity::Exploratory,
        };
        let event = ViewMaturationEvent {
            maturation_id: "maturation_1".to_string(),
            target_kind: ViewMaturationTargetKind::Derivation,
            shape_id: Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string()),
            derivation_id: Some(derivation.derivation_id.clone()),
            pattern_id: None,
            origin: ViewMaturationOrigin::UserRequested,
            from_maturity: ViewShapeMaturity::Candidate,
            to_maturity: ViewShapeMaturity::Stable,
            created_at: Utc::now(),
        };
        let artifact = ObservabilityImprovementArtifact {
            artifact_id: "maturation_1::observability-improvement".to_string(),
            source_gap_id: None,
            source_maturation_id: Some(event.maturation_id.clone()),
            desired_fact_path: "mother.status.memory_pressure.summary".to_string(),
            reason: "stable derivation should become observable".to_string(),
            created_at: Utc::now(),
            work_item_created: false,
        };

        store.upsert_view_derivation(&derivation).unwrap();
        store.upsert_view_display_pattern(&pattern).unwrap();
        store.save_view_maturation_event(&event).unwrap();
        store
            .save_view_observability_improvement(&artifact)
            .unwrap();

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        assert_eq!(
            reopened
                .get_view_derivation(&derivation.derivation_id)
                .unwrap(),
            Some(derivation.clone())
        );
        assert_eq!(reopened.list_view_derivations().unwrap(), vec![derivation]);
        assert_eq!(
            reopened
                .get_view_display_pattern(&pattern.pattern_id)
                .unwrap(),
            Some(pattern.clone())
        );
        assert_eq!(
            reopened.list_view_display_patterns().unwrap(),
            vec![pattern]
        );
        assert_eq!(
            reopened
                .get_view_maturation_event(&event.maturation_id)
                .unwrap(),
            Some(event.clone())
        );
        assert_eq!(reopened.list_view_maturation_events().unwrap(), vec![event]);
        assert_eq!(
            reopened
                .get_view_observability_improvement(&artifact.artifact_id)
                .unwrap(),
            Some(artifact.clone())
        );
        assert_eq!(
            reopened.list_view_observability_improvements().unwrap(),
            vec![artifact]
        );
    }

    #[test]
    fn view_shapes_and_requirements_are_persistent() {
        // obligation: entity-state.ViewShape + entity-state.ViewRequirement
        // obligation: spec.mother-view-shape-library.mvsl2-shape-persistence
        use crate::view_buffer::{
            MajorMode, MinorMode, PayloadContract, ViewRequirement, ViewShape, ViewShapeMaturity,
            ViewShapeScope,
        };

        let store = temp_store();
        let shape = ViewShape {
            shape_id: "test.shape.default".to_string(),
            title: "Test Shape".to_string(),
            source_ref: "local-allium-view-library".to_string(),
            scope: ViewShapeScope::Project,
            version: 7,
            active: true,
            major_mode: MajorMode::Table,
            minor_modes: vec![MinorMode::Pinned, MinorMode::Sorted],
            maturity: ViewShapeMaturity::Candidate,
            payload_contract: PayloadContract::FramedJson,
            payload_version: 3,
            vision_id: Some("vision-1".to_string()),
            project_uid: Some("2bdc808e".to_string()),
            replaced_by: Some("test.shape.v8".to_string()),
            requirements: vec![
                ViewRequirement {
                    fact_path: "alpha.fact".to_string(),
                    required: true,
                    purpose: "required display fact".to_string(),
                },
                ViewRequirement {
                    fact_path: "beta.fact".to_string(),
                    required: false,
                    purpose: "optional enrichment fact".to_string(),
                },
            ],
        };

        store.upsert_view_shape(&shape).unwrap();

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        assert_eq!(
            reopened.get_view_shape(&shape.shape_id).unwrap(),
            Some(shape.clone())
        );
        assert_eq!(reopened.list_view_shapes().unwrap(), vec![shape.clone()]);

        assert!(reopened.deactivate_view_shape(&shape.shape_id).unwrap());
        let deactivated = reopened
            .get_view_shape(&shape.shape_id)
            .unwrap()
            .expect("shape remains after deactivation");
        assert!(!deactivated.active);
        assert!(!reopened.deactivate_view_shape("missing.shape").unwrap());
    }

    #[test]
    fn view_initial_shape_creation_persists_created_shape_metadata() {
        // obligation: spec.mother-view-initial-shape-creation.mvisc4-persistence
        // obligation: rule-success.CreateInitialShapeWhenNoShapeMatches
        use crate::view_buffer::{
            ComposeViewRequest, DataCatalog, MajorMode, MinorMode, MotherStatusFacts,
            ProposedInitialShape, ProposedShapeMatch, ShapeMatchKind, ViewBufferService,
            ViewRequirement, ViewShapeMaturity,
        };

        let store = temp_store();
        let mut service =
            ViewBufferService::with_catalog(DataCatalog::mother_status(MotherStatusFacts {
                version: "0.70.1".to_string(),
                uptime_secs: 42,
                control_plane_ready: true,
                registered_projects: 2,
                children_ready_count: 1,
                children_total: 2,
                startup_profile: "full".to_string(),
                memory_pressure: "ok".to_string(),
                observed_at: Utc::now(),
            }));
        let requirements = vec![ViewRequirement {
            fact_path: "mother.status.version".to_string(),
            required: true,
            purpose: "display Mother version".to_string(),
        }];

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show runtime summary".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: None,
                    match_kind: ShapeMatchKind::None,
                    confidence: 0.0,
                }),
                proposed_initial_shape: Some(ProposedInitialShape {
                    title: "Mother Runtime Summary".to_string(),
                    major_mode: MajorMode::Table,
                    minor_modes: vec![MinorMode::Pinned],
                    requirements: requirements.clone(),
                    vision_id: Some("vision-1".to_string()),
                    project_uid: Some("2bdc808e".to_string()),
                }),
            })
            .unwrap();
        let created_shape = composed
            .created_shape
            .expect("no-match request should return created shape");

        store.upsert_view_shape(&created_shape).unwrap();

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        let persisted = reopened
            .get_view_shape(&created_shape.shape_id)
            .unwrap()
            .expect("created shape should persist");
        assert_eq!(persisted, created_shape);
        assert_eq!(persisted.maturity, ViewShapeMaturity::Exploratory);
        assert_eq!(persisted.source_ref, "local-allium-view-library");
        assert_eq!(persisted.major_mode, MajorMode::Table);
        assert_eq!(persisted.minor_modes, vec![MinorMode::Pinned]);
        assert_eq!(persisted.requirements, requirements);
    }

    #[test]
    fn view_shape_adaptation_persists_adapted_shape_metadata() {
        // obligation: spec.mother-view-shape-adaptation.mvsa3-adapted-shape-persistence
        // obligation: rule-success.AdaptSimilarShapeWhenNoExactShapeExists
        use crate::view_buffer::{
            ComposeViewRequest, DataCatalog, MotherStatusFacts, ProposedShapeMatch, ShapeMatchKind,
            ViewBufferService, ViewShapeMaturity, SHAPE_MATCH_CONFIDENCE_THRESHOLD,
        };

        let store = temp_store();
        let precedent = crate::view_buffer::mother_status_shape();
        let mut service = ViewBufferService::with_catalog_and_shapes(
            DataCatalog::mother_status(MotherStatusFacts {
                version: "0.70.0".to_string(),
                uptime_secs: 42,
                control_plane_ready: true,
                registered_projects: 2,
                children_ready_count: 1,
                children_total: 2,
                startup_profile: "full".to_string(),
                memory_pressure: "ok".to_string(),
                observed_at: Utc::now(),
            }),
            vec![precedent.clone()],
        );

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show something like mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: Some(precedent.shape_id.clone()),
                    match_kind: ShapeMatchKind::Similar,
                    confidence: SHAPE_MATCH_CONFIDENCE_THRESHOLD,
                }),
                proposed_initial_shape: None,
            })
            .unwrap();
        let adapted_shape = composed
            .adapted_shape
            .expect("similar match should return adapted shape");

        store.upsert_view_shape(&adapted_shape).unwrap();

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        let mut persisted = reopened
            .get_view_shape(&adapted_shape.shape_id)
            .unwrap()
            .expect("adapted shape should persist");
        let mut expected = adapted_shape;
        persisted
            .requirements
            .sort_by(|left, right| left.fact_path.cmp(&right.fact_path));
        expected
            .requirements
            .sort_by(|left, right| left.fact_path.cmp(&right.fact_path));
        assert_eq!(persisted, expected);
        assert_eq!(persisted.maturity, ViewShapeMaturity::Exploratory);
        assert_eq!(persisted.source_ref, precedent.source_ref);
        assert_eq!(persisted.scope, precedent.scope);
        assert_eq!(persisted.major_mode, precedent.major_mode);
        assert_eq!(persisted.minor_modes, precedent.minor_modes);
        assert_eq!(persisted.payload_contract, precedent.payload_contract);
        assert_eq!(persisted.payload_version, precedent.payload_version);
        let mut expected_requirements = precedent.requirements;
        expected_requirements.sort_by(|left, right| left.fact_path.cmp(&right.fact_path));
        assert_eq!(persisted.requirements, expected_requirements);
    }

    #[test]
    fn seed_view_shape_is_idempotent_and_preserves_existing_shape() {
        // obligation: spec.mother-view-shape-library.mvsl5-proof-shapes-seeded
        let store = temp_store();
        let mut shape = crate::view_buffer::mother_status_shape();
        shape
            .requirements
            .sort_by(|left, right| left.fact_path.cmp(&right.fact_path));

        assert!(store.seed_view_shape(&shape).unwrap());
        assert_eq!(
            store.get_view_shape(&shape.shape_id).unwrap(),
            Some(shape.clone())
        );

        shape.title = "User Edited Mother Status".to_string();
        store.upsert_view_shape(&shape).unwrap();
        assert!(!store
            .seed_view_shape(&crate::view_buffer::mother_status_shape())
            .unwrap());
        assert_eq!(
            store
                .get_view_shape(&shape.shape_id)
                .unwrap()
                .expect("seeded shape should exist")
                .title,
            "User Edited Mother Status"
        );
    }

    #[test]
    fn view_observability_workflow_gap_links_and_resolution_persist() {
        // obligation: spec.mother-view-observability-workflow.mvow4-persistence
        use crate::view_buffer::{ObservabilityGap, ObservabilityGapStatus};

        let store = temp_store();
        let mut gap = ObservabilityGap::open(
            "gap_1".to_string(),
            Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string()),
            "mother.status.version".to_string(),
            Some("mother.status".to_string()),
            "missing version".to_string(),
            Utc::now(),
        );
        gap.status = ObservabilityGapStatus::LinkedToWorkItem;
        gap.linked_work_item_id = Some("work/MOTHER-123".to_string());
        store.save_view_observability_gap(&gap).unwrap();

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        assert_eq!(
            reopened.get_view_observability_gap(&gap.gap_id).unwrap(),
            Some(gap.clone())
        );

        gap.status = ObservabilityGapStatus::Resolved;
        gap.resolved_at = Some(Utc::now());
        reopened.save_view_observability_gap(&gap).unwrap();
        assert_eq!(
            reopened
                .get_view_observability_gap(&gap.gap_id)
                .unwrap()
                .and_then(|gap| gap.linked_work_item_id),
            Some("work/MOTHER-123".to_string())
        );
    }

    #[test]
    fn view_buffer_records_are_persistent() {
        // obligation: entity-state.Buffer + entity-state.Frame + entity-state.Window
        // obligation: entity-state.ObservabilityGap
        use crate::view_buffer::{
            mother_status_shape, Buffer, BufferState, Frame, FrameKind, MinorMode,
            ObservabilityGap, ObservabilityGapStatus, Window, WindowConnectionState,
        };

        let store = temp_store();
        let now = chrono::DateTime::parse_from_rfc3339("2026-05-09T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let shape = mother_status_shape();
        let mut buffer = Buffer::live_from_shape("buf_persisted".to_string(), &shape, now);
        buffer.state = BufferState::Stale;
        buffer.stale_at = Some(now);
        buffer.minor_modes = vec![MinorMode::Pinned];
        let frame = Frame {
            frame_id: "frame_tui".to_string(),
            frame_kind: FrameKind::Tui,
            connected_at: now,
        };
        let window = Window {
            window_id: "win_1".to_string(),
            frame_id: frame.frame_id.clone(),
            buffer_id: Some(buffer.buffer_id.clone()),
            connection_state: WindowConnectionState::Connected,
            connected_at: Some(now),
            disconnected_at: None,
        };
        let gap = ObservabilityGap {
            gap_id: "gap_1".to_string(),
            shape_id: Some(shape.shape_id.clone()),
            missing_fact_path: "mother.status.children_total".to_string(),
            missing_source_id: Some("mother.status".to_string()),
            reason: "test gap".to_string(),
            status: ObservabilityGapStatus::Open,
            linked_work_item_id: None,
            created_at: now,
            resolved_at: None,
        };

        store.save_view_buffer(&buffer).unwrap();
        store.save_view_frame(&frame).unwrap();
        store.save_view_window(&window).unwrap();
        store.save_view_observability_gap(&gap).unwrap();

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        assert_eq!(reopened.list_view_buffers().unwrap(), vec![buffer]);
        assert_eq!(reopened.list_view_frames().unwrap(), vec![frame]);
        assert_eq!(reopened.list_view_windows().unwrap(), vec![window]);
        assert_eq!(reopened.list_view_observability_gaps().unwrap(), vec![gap]);
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

        let reopened = MotherRuntimeStore::new_with_project(
            store.path().clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
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
            voice_uid: None,
            status: MotherSessionStatus::Active,
            interface_kind: "opencode".to_string(),
            interface_name: "opencode".to_string(),
            branch: Some("patina".to_string()),
            start_tag: Some("session-20260311-100000-ABCD-opencode-start".to_string()),
            end_tag: None,
            parent_runtime_id: None,
            handoff_from_runtime_id: None,
            starting_commit: Some("deadbeef".to_string()),
            created_at: created_at.clone(),
            updated_at: created_at.clone(),
        };
        let second = MotherSessionRecord {
            runtime_id: uuid::Uuid::new_v4().to_string(),
            project_uid: "proj-1234".to_string(),
            file_id: "20260311-100001-EFGH".to_string(),
            title: "Gemini session".to_string(),
            voice_uid: Some("voice-1".to_string()),
            status: MotherSessionStatus::Active,
            interface_kind: "gemini".to_string(),
            interface_name: "gemini".to_string(),
            branch: Some("patina".to_string()),
            start_tag: Some("session-20260311-100001-EFGH-gemini-start".to_string()),
            end_tag: None,
            parent_runtime_id: None,
            handoff_from_runtime_id: None,
            starting_commit: Some("feedface".to_string()),
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
        let voice_uid = VoiceUid::new("voice-1").unwrap();
        let missing_voice_uid = VoiceUid::new("voice-missing").unwrap();
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
                Some(&voice_uid),
            )
            .unwrap()
            .is_some());
        assert!(store
            .find_active_mother_session_for_interface(
                &project_uid,
                "gemini",
                &gemini_kind,
                Some(&missing_voice_uid),
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
        assert_eq!(reloaded.starting_commit(), "deadbeef");
        assert_eq!(
            reloaded.end_tag.as_deref(),
            Some("session-20260311-100000-ABCD-opencode-end")
        );
    }

    #[test]
    fn project_registration_roundtrips_and_rejects_uid_path_collisions() {
        let store = temp_store();
        let uid = ProjectUid::new("2bdc808e").unwrap();

        let first_dir = tempfile::tempdir().unwrap();
        store.register_project(&uid, first_dir.path()).unwrap();

        let listed = store.list_registered_projects().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].project_uid, "2bdc808e");
        assert!(listed[0]
            .project_path
            .contains(first_dir.path().to_string_lossy().as_ref()));

        let first_registered_at = listed[0].registered_at.clone();

        // Re-registering the same path is idempotent.
        store.register_project(&uid, first_dir.path()).unwrap();
        let listed = store.list_registered_projects().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].registered_at, first_registered_at);

        // Registering a different path with the same UID is a hard error.
        let second_dir = tempfile::tempdir().unwrap();
        let collision = store.register_project(&uid, second_dir.path());
        assert!(collision.is_err());
        assert!(collision
            .unwrap_err()
            .to_string()
            .contains("project_uid collision"));
    }

    #[test]
    fn project_belief_state_roundtrip() {
        let store = temp_store();
        let uid = ProjectUid::new("2bdc808e").unwrap();
        let dir = tempfile::tempdir().unwrap();
        store.register_project(&uid, dir.path()).unwrap();

        let update = ProjectBeliefStateUpdate {
            project_uid: "2bdc808e".to_string(),
            project_id: None,
            source_commit_sha: Some("deadbeef".to_string()),
            source_belief_count: Some(12),
            source_value_count: Some(3),
            source_fingerprint: Some("abc123".to_string()),
            source_last_activity: Some("2026-04-24".to_string()),
            indexed_belief_count: Some(12),
            indexed_value_count: Some(3),
            indexed_fingerprint: Some("abc123".to_string()),
            status: "fresh".to_string(),
            last_error: None,
        };

        store.upsert_project_belief_state(&update).unwrap();
        let record = store
            .get_project_belief_state("2bdc808e")
            .unwrap()
            .expect("belief state should exist");
        assert_eq!(record.project_uid, "2bdc808e");
        assert_eq!(record.status, "fresh");
        assert_eq!(record.source_belief_count, Some(12));
        assert_eq!(record.indexed_belief_count, Some(12));
        assert_eq!(record.source_commit_sha.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn child_registry_roundtrip_and_assignment_requires_approved_entry() {
        let store = temp_store();

        store
            .upsert_child_registry_source(&ChildRegistrySourceUpdate {
                source_id: "src_github_slate".to_string(),
                provider_kind: "github".to_string(),
                provider_config_json: r#"{"owner":"NicabarNimble","repo":"patina-child-slate"}"#
                    .to_string(),
                enabled: true,
            })
            .unwrap();

        let sources = store.list_child_registry_sources().unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].provider_kind, "github");
        assert!(sources[0].enabled);

        store
            .upsert_child_registry_entry(&ChildRegistryEntryUpdate {
                entry_id: "entry_slate_v0_1_0".to_string(),
                child_name: "slate-manager".to_string(),
                version: "0.1.0".to_string(),
                source_id: "src_github_slate".to_string(),
                source_release_ref: "v0.1.0".to_string(),
                artifact_url: "https://example.invalid/slate-manager.wasm".to_string(),
                manifest_url: "https://example.invalid/slate-manager.toml".to_string(),
                checksums_url: None,
                artifact_sha256: "a3d24c4036f88fe4ca64f70556f0eae2e4ef6f878b6c51481e4a4e5c4b2b8f66"
                    .to_string(),
                manifest_sha256: "c26bcdf6529d8adf4ceac76714566491582f59d0bc889ef9e4d8ce96aa95f4c4"
                    .to_string(),
                signature_ref: None,
                patina_min: Some("0.64.4".to_string()),
                operations_json: Some(r#"["patina:slate/control.list-specs"]"#.to_string()),
                needs_toys_json: Some(r#"["logging","measure","git"]"#.to_string()),
                needs_scopes_json: None,
                state: "candidate".to_string(),
                state_reason: Some("newly discovered".to_string()),
            })
            .unwrap();

        let entries = store
            .list_child_registry_entries(Some("slate-manager"))
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].state, "candidate");

        let project_uid = ProjectUid::new("2bdc808e").unwrap();
        let project_dir = tempfile::tempdir().unwrap();
        store
            .register_project(&project_uid, project_dir.path())
            .unwrap();

        let denied = store.upsert_project_child_assignment(&ProjectChildAssignmentUpdate {
            assignment_id: "asg_slate_project".to_string(),
            project_uid: "2bdc808e".to_string(),
            project_id: None,
            child_name: "slate-manager".to_string(),
            entry_id: "entry_slate_v0_1_0".to_string(),
            pinned_version: "0.1.0".to_string(),
            status: "active".to_string(),
            reason: Some("initial assignment".to_string()),
        });
        assert!(denied.is_err());
        assert!(denied
            .unwrap_err()
            .to_string()
            .contains("requires approved child registry entry"));

        store
            .transition_child_registry_entry_state(
                "entry_slate_v0_1_0",
                "approved",
                Some("security review complete"),
                false,
            )
            .unwrap();

        let invalid_reverse = store.transition_child_registry_entry_state(
            "entry_slate_v0_1_0",
            "candidate",
            Some("should fail"),
            false,
        );
        assert!(invalid_reverse.is_err());

        store
            .upsert_project_child_assignment(&ProjectChildAssignmentUpdate {
                assignment_id: "asg_slate_project".to_string(),
                project_uid: "2bdc808e".to_string(),
                project_id: None,
                child_name: "slate-manager".to_string(),
                entry_id: "entry_slate_v0_1_0".to_string(),
                pinned_version: "0.1.0".to_string(),
                status: "active".to_string(),
                reason: Some("approved for project".to_string()),
            })
            .unwrap();

        let assignments = store
            .list_project_child_assignments(Some("2bdc808e"))
            .unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].child_name, "slate-manager");
        assert_eq!(assignments[0].status, "active");

        store
            .upsert_child_install(&ChildInstallUpdate {
                install_id: "install_slate_v0_1_0".to_string(),
                entry_id: "entry_slate_v0_1_0".to_string(),
                installed_name: "slate-manager".to_string(),
                installed_version: "0.1.0".to_string(),
                wasm_path: "/Users/nicabar/.patina/children/slate-manager.wasm".to_string(),
                manifest_path: "/Users/nicabar/.patina/children/slate-manager.toml".to_string(),
                artifact_sha256_verified:
                    "a3d24c4036f88fe4ca64f70556f0eae2e4ef6f878b6c51481e4a4e5c4b2b8f66".to_string(),
                manifest_sha256_verified:
                    "c26bcdf6529d8adf4ceac76714566491582f59d0bc889ef9e4d8ce96aa95f4c4".to_string(),
                installed_by: Some("usr_3c87424dc90e4d43b61c47dacf43ab9b".to_string()),
                status: "installed".to_string(),
                last_error: None,
            })
            .unwrap();

        let installs = store.list_child_installs(Some("slate-manager")).unwrap();
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0].status, "installed");
    }

    #[test]
    fn child_install_hashes_are_non_empty() {
        let store = temp_store();

        let err = store.upsert_child_install(&ChildInstallUpdate {
            install_id: "install_bad".to_string(),
            entry_id: "entry_missing".to_string(),
            installed_name: "slate-manager".to_string(),
            installed_version: "0.1.0".to_string(),
            wasm_path: "/tmp/slate-manager.wasm".to_string(),
            manifest_path: "/tmp/slate-manager.toml".to_string(),
            artifact_sha256_verified: "".to_string(),
            manifest_sha256_verified: "".to_string(),
            installed_by: None,
            status: "failed".to_string(),
            last_error: Some("checksum missing".to_string()),
        });

        assert!(err.is_err());
    }

    #[test]
    fn identity_plane_guardrails_prevent_orphaned_authority_rows() {
        let store = temp_store();
        let conn = store.open().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO mother_users (user_id, user_handle, display_name, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', ?4, ?4)",
            rusqlite::params!["usr_test", "nicabar", "nicabar", now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mother_nodes (node_id, node_slug, hostname, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', ?4, ?4)",
            rusqlite::params!["nod_test", "node-a", "node-a", now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mother_visions (vision_id, vision_slug, owner_user_id, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', ?4, ?4)",
            rusqlite::params!["vis_test", "vision", "usr_test", now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mother_node_memberships (node_id, user_id, role, status, created_at, updated_at)
             VALUES (?1, ?2, 'full_admin', 'active', ?3, ?3)",
            rusqlite::params!["nod_test", "usr_test", now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mother_vision_memberships (vision_id, user_id, role, status, created_at, updated_at)
             VALUES (?1, ?2, 'admin', 'active', ?3, ?3)",
            rusqlite::params!["vis_test", "usr_test", now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mother_node_visions (node_id, vision_id, status, created_at, updated_at)
             VALUES (?1, ?2, 'active', ?3, ?3)",
            rusqlite::params!["nod_test", "vis_test", now],
        )
        .unwrap();

        let demote_admin = conn.execute(
            "UPDATE mother_node_memberships
             SET role = 'admin'
             WHERE node_id = 'nod_test' AND user_id = 'usr_test'",
            [],
        );
        assert!(demote_admin.is_err());

        let disable_vision = conn.execute(
            "UPDATE mother_node_visions
             SET status = 'disabled'
             WHERE node_id = 'nod_test' AND vision_id = 'vis_test'",
            [],
        );
        assert!(disable_vision.is_err());

        let demote_vision_admin = conn.execute(
            "UPDATE mother_vision_memberships
             SET role = 'member'
             WHERE vision_id = 'vis_test' AND user_id = 'usr_test'",
            [],
        );
        assert!(demote_vision_admin.is_err());
    }

    #[test]
    fn child_state_isolated_by_project_uid() {
        let temp = tempfile::tempdir().unwrap();
        let state_path = temp.path().join("mother/state.db");
        let project_a = MotherRuntimeStore::new_with_project(
            state_path.clone(),
            ProjectUid::new("2bdc808e").unwrap(),
        );
        let project_b =
            MotherRuntimeStore::new_with_project(state_path, ProjectUid::new("1a2b3c4d").unwrap());

        project_a
            .put_state("ducklake", "shared-key", r#"{"project":"a"}"#)
            .unwrap();
        project_b
            .put_state("ducklake", "shared-key", r#"{"project":"b"}"#)
            .unwrap();

        assert_eq!(
            project_a
                .get_state("ducklake", "shared-key")
                .unwrap()
                .as_deref(),
            Some(r#"{"project":"a"}"#)
        );
        assert_eq!(
            project_b
                .get_state("ducklake", "shared-key")
                .unwrap()
                .as_deref(),
            Some(r#"{"project":"b"}"#)
        );
    }
}
