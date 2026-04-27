//! Internal implementation for project module
//!
//! Handles .patina/config.toml - unified project configuration.
//! Supports migration from legacy config.json format.

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// Config Types - Unified Schema
// =============================================================================

/// Project configuration stored in .patina/config.toml
/// All sections are optional with defaults for backward compatibility
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub project: ProjectSection,
    /// Deprecated: dev environment config (kept for backwards compat on load)
    #[serde(default, skip_serializing)]
    pub dev: DevSection,
    #[serde(default, rename = "interfaces", alias = "adapters")]
    pub interfaces: InterfacesSection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<UpstreamSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci: Option<CiSection>,
    #[serde(default)]
    pub embeddings: EmbeddingsSection,
    #[serde(default)]
    pub search: SearchSection,
    #[serde(default)]
    pub retrieval: RetrievalSection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentSection>,
    #[serde(default)]
    pub beliefs: BeliefsSection,
}

impl ProjectConfig {
    /// Create config with project name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            project: ProjectSection {
                name: name.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    /// Project name
    #[serde(default = "default_name")]
    pub name: String,
    /// Creation timestamp (ISO 8601)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Development branch name (default: "work")
    #[serde(default = "default_branch")]
    pub branch: String,
}

fn default_name() -> String {
    "unnamed".to_string()
}

fn default_branch() -> String {
    "work".to_string()
}

impl Default for ProjectSection {
    fn default() -> Self {
        Self {
            name: default_name(),
            created: None,
            branch: default_branch(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevSection {
    /// Dev environment type: "docker" | "native"
    #[serde(default = "default_dev_type", rename = "type")]
    pub dev_type: String,
    /// Dev environment version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

fn default_dev_type() -> String {
    "docker".to_string()
}

impl Default for DevSection {
    fn default() -> Self {
        Self {
            dev_type: default_dev_type(),
            version: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfacesSection {
    /// Allowed interfaces for this project
    #[serde(default = "default_allowed")]
    pub allowed: Vec<String>,
    /// Default interface for this project
    #[serde(default = "default_interface")]
    pub default: String,
}

fn default_allowed() -> Vec<String> {
    vec!["claude".to_string()]
}
fn default_interface() -> String {
    "claude".to_string()
}

impl Default for InterfacesSection {
    fn default() -> Self {
        Self {
            allowed: default_allowed(),
            default: default_interface(),
        }
    }
}

/// Upstream repository configuration for contributions
/// Helps LLM create clean PRs that won't get rejected
///
/// Every repo has an upstream - even owned repos (upstream = yourself).
/// The key difference is what gets included in PRs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamSection {
    /// GitHub repo in "owner/repo" format (for gh pr create --repo)
    pub repo: String,
    /// Target branch for PRs (default: main)
    #[serde(default = "default_upstream_branch")]
    pub branch: String,
    /// Git remote name: "upstream" for forks, "origin" if you own the repo
    #[serde(default = "default_upstream_remote")]
    pub remote: String,
    /// Include .patina/ directory in PRs (default: false)
    /// Set true for owned repos where you want to share knowledge
    #[serde(default)]
    pub include_patina: bool,
    /// Include interface files (CLAUDE.md, .claude/, etc.) in PRs (default: false)
    /// Set true for owned repos to share with collaborators
    #[serde(default, alias = "include_adapters")]
    pub include_interfaces: bool,
}

fn default_upstream_branch() -> String {
    "main".to_string()
}

fn default_upstream_remote() -> String {
    "upstream".to_string()
}

/// CI checks to run before creating a PR
/// Ensures PR won't fail upstream CI
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CiSection {
    /// Commands to run before PR (format, lint, test)
    #[serde(default)]
    pub checks: Vec<String>,
    /// Branch naming convention (e.g., "feat/", "fix/")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsSection {
    /// Embedding model to use
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_model() -> String {
    "e5-base-v2".to_string()
}

impl Default for EmbeddingsSection {
    fn default() -> Self {
        Self {
            model: default_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSection {
    /// Operating system
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Detected tools
    #[serde(default)]
    pub detected_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefsSection {
    #[serde(default = "default_stale_days")]
    pub stale_days: u32,
}

fn default_stale_days() -> u32 {
    90
}

impl Default for BeliefsSection {
    fn default() -> Self {
        Self {
            stale_days: default_stale_days(),
        }
    }
}

/// Retrieval configuration - RRF fusion parameters
///
/// These are algorithm constants from the literature (Cormack et al., 2009).
/// Most users should not change these unless experimenting with retrieval quality.
///
/// - **rrf_k** (60): Smoothing constant for RRF. Higher values reduce the
///   impact of top ranks. k=60 is standard from the original paper.
/// - **fetch_multiplier** (2): Over-fetch factor for fusion. Fetches limit * N
///   results from each oracle before fusion to improve diversity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalSection {
    /// RRF smoothing constant (default: 60)
    #[serde(default = "default_rrf_k")]
    pub rrf_k: usize,

    /// Over-fetch multiplier for fusion (default: 2)
    #[serde(default = "default_fetch_multiplier")]
    pub fetch_multiplier: usize,
}

fn default_rrf_k() -> usize {
    60
}

fn default_fetch_multiplier() -> usize {
    2
}

impl Default for RetrievalSection {
    fn default() -> Self {
        Self {
            rrf_k: default_rrf_k(),
            fetch_multiplier: default_fetch_multiplier(),
        }
    }
}

/// Search configuration - ML thresholds for different use cases
///
/// Different commands have different min_score defaults because they serve
/// different purposes:
/// - **scry** (0.0): Cast a wide net, let user filter results
/// - **query semantic** (0.35): Balance relevance vs recall for exploration
/// - **belief validate** (0.50): Only strong evidence for validation claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSection {
    /// Default threshold for scry command (broad search, low filter)
    #[serde(default = "default_scry_threshold")]
    pub scry_threshold: f32,

    /// Default threshold for semantic queries (moderate filter)
    #[serde(default = "default_semantic_threshold")]
    pub semantic_threshold: f32,

    /// Default threshold for belief validation (strict evidence)
    #[serde(default = "default_belief_threshold")]
    pub belief_threshold: f32,
}

fn default_scry_threshold() -> f32 {
    0.0
}
fn default_semantic_threshold() -> f32 {
    0.35
}
fn default_belief_threshold() -> f32 {
    0.50
}

impl Default for SearchSection {
    fn default() -> Self {
        Self {
            scry_threshold: default_scry_threshold(),
            semantic_threshold: default_semantic_threshold(),
            belief_threshold: default_belief_threshold(),
        }
    }
}

// =============================================================================
// Path Functions
// =============================================================================

/// Get the .patina directory for a project
pub fn patina_dir(project_path: &Path) -> PathBuf {
    crate::paths::project::patina_dir(project_path)
}

/// Get the config file path for a project
pub fn config_path(project_path: &Path) -> PathBuf {
    crate::paths::project::config_path(project_path)
}

/// Get the legacy config.json path
pub fn legacy_config_path(project_path: &Path) -> PathBuf {
    crate::paths::project::legacy_config_path(project_path)
}

/// Get the local state directory for a project (gitignored)
pub fn local_dir(project_path: &Path) -> PathBuf {
    crate::paths::project::local_dir(project_path)
}

/// Get the backups directory for a project
pub fn backups_dir(project_path: &Path) -> PathBuf {
    crate::paths::project::backups_dir(project_path)
}

// =============================================================================
// UID (Project Identity)
// =============================================================================

/// Get the UID file path for a project
pub fn uid_path(project_path: &Path) -> PathBuf {
    crate::paths::project::uid_path(project_path)
}

/// Get the voice binding file path for a project
pub fn voice_path(project_path: &Path) -> PathBuf {
    crate::paths::project::voice_path(project_path)
}

/// Get the project voice binding (returns None if not set)
pub fn get_voice(project_path: &Path) -> Option<String> {
    let path = voice_path(project_path);
    if !path.exists() {
        return None;
    }
    let value = fs::read_to_string(path).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Create a unique project identifier if it doesn't exist
/// Returns the UID (8 hex characters, created once, never modified)
pub fn create_uid_if_missing(project_path: &Path) -> Result<String> {
    let uid_file = uid_path(project_path);

    // If UID exists, read and return it
    if uid_file.exists() {
        return Ok(fs::read_to_string(&uid_file)?.trim().to_string());
    }

    // Generate new UID (8 hex chars from random u32)
    let uid = format!("{:08x}", fastrand::u32(..));

    // Ensure .patina directory exists
    if let Some(parent) = uid_file.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write UID
    fs::write(&uid_file, &uid)
        .with_context(|| format!("Failed to create UID file: {}", uid_file.display()))?;

    Ok(uid)
}

/// Get the UID for a project (returns None if not initialized)
pub fn get_uid(project_path: &Path) -> Option<String> {
    let uid_file = uid_path(project_path);
    if uid_file.exists() {
        fs::read_to_string(&uid_file)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Ensure project is registered in Mother's project registry.
pub fn register_with_mother(project_path: &Path) -> Result<String> {
    let uid = create_uid_if_missing(project_path)?;
    let uid_typed = crate::mother::ProjectUid::new(uid.clone())?;
    let store = crate::mother::MotherRuntimeStore::default();
    store.register_project(&uid_typed, project_path)?;

    // Ensure default voice store structure exists (GMDP-G9).
    let _voice_dir =
        crate::paths::mother::voice::ensure_voice_dir("default").map_err(anyhow::Error::msg)?;
    let beliefs_db =
        crate::paths::mother::voice::beliefs_db("default").map_err(anyhow::Error::msg)?;
    rusqlite::Connection::open(&beliefs_db)
        .with_context(|| format!("initializing voice store at {}", beliefs_db.display()))?;

    if let Err(error) = verify_project_belief_state(&store, project_path, &uid) {
        eprintln!(
            "⚠️  mother belief verification failed for {}: {}",
            project_path.display(),
            error
        );
    }

    Ok(uid)
}

#[derive(Debug, Clone)]
struct BeliefInventory {
    belief_count: i64,
    value_count: i64,
    fingerprint: String,
    last_activity: Option<String>,
}

fn verify_project_belief_state(
    store: &crate::mother::MotherRuntimeStore,
    project_path: &Path,
    project_uid: &str,
) -> Result<()> {
    let project_id = lookup_project_id(store.path(), project_uid)?;
    let source_commit_sha = resolve_project_head_sha(project_path);

    let source_inventory =
        read_source_belief_inventory(&crate::eventlog::resolve_patina_db_path(project_path));
    let indexed_inventory = read_indexed_belief_inventory(project_uid, project_path);

    let mut update = crate::mother::ProjectBeliefStateUpdate {
        project_uid: project_uid.to_string(),
        project_id,
        source_commit_sha,
        source_belief_count: None,
        source_value_count: None,
        source_fingerprint: None,
        source_last_activity: None,
        indexed_belief_count: None,
        indexed_value_count: None,
        indexed_fingerprint: None,
        status: "unknown".to_string(),
        last_error: None,
    };

    match source_inventory {
        Ok(Some(source)) => {
            update.source_belief_count = Some(source.belief_count);
            update.source_value_count = Some(source.value_count);
            update.source_fingerprint = Some(source.fingerprint.clone());
            update.source_last_activity = source.last_activity.clone();

            match indexed_inventory {
                Ok(Some(indexed)) => {
                    update.indexed_belief_count = Some(indexed.belief_count);
                    update.indexed_value_count = Some(indexed.value_count);
                    update.indexed_fingerprint = Some(indexed.fingerprint.clone());

                    let counts_match = source.belief_count == indexed.belief_count
                        && source.value_count == indexed.value_count;
                    let fingerprint_match = source.fingerprint == indexed.fingerprint;
                    update.status = if counts_match && fingerprint_match {
                        "fresh".to_string()
                    } else {
                        "drifted".to_string()
                    };
                }
                Ok(None) => {
                    update.status = "index_missing".to_string();
                }
                Err(error) => {
                    update.status = "error".to_string();
                    update.last_error = Some(format!("indexed inventory failed: {error}"));
                }
            }
        }
        Ok(None) => {
            update.status = "source_missing".to_string();
        }
        Err(error) => {
            update.status = "error".to_string();
            update.last_error = Some(format!("source inventory failed: {error}"));
        }
    }

    store.upsert_project_belief_state(&update)?;
    Ok(())
}

fn lookup_project_id(state_path: &Path, project_uid: &str) -> Result<Option<String>> {
    let conn = rusqlite::Connection::open(state_path)
        .with_context(|| format!("opening mother state db {}", state_path.display()))?;
    conn.query_row(
        "SELECT project_id FROM mother_project_identities WHERE project_uid = ?1",
        rusqlite::params![project_uid],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(Into::into)
}

fn resolve_project_head_sha(project_path: &Path) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(project_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            } else {
                None
            }
        })
}

fn read_source_belief_inventory(db_path: &Path) -> Result<Option<BeliefInventory>> {
    if !db_path.exists() {
        return Ok(None);
    }

    let conn = rusqlite::Connection::open(db_path)
        .with_context(|| format!("opening project db {}", db_path.display()))?;

    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='beliefs'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(None);
    }

    let has_kind = conn.prepare("SELECT kind FROM beliefs LIMIT 0").is_ok();
    let has_status = conn.prepare("SELECT status FROM beliefs LIMIT 0").is_ok();
    let has_last_activity = conn
        .prepare("SELECT last_activity FROM beliefs LIMIT 0")
        .is_ok();

    let status_filter = if has_status {
        "COALESCE(status, 'active') <> 'archived'"
    } else {
        "1=1"
    };
    let belief_kind_filter = if has_kind {
        "(kind IS NULL OR kind <> 'value')"
    } else {
        "1=1"
    };
    let value_kind_filter = if has_kind { "kind = 'value'" } else { "0=1" };

    let source_belief_count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM beliefs WHERE {} AND {}",
            status_filter, belief_kind_filter
        ),
        [],
        |row| row.get(0),
    )?;
    let source_value_count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM beliefs WHERE {} AND {}",
            status_filter, value_kind_filter
        ),
        [],
        |row| row.get(0),
    )?;

    let kind_select = if has_kind {
        "COALESCE(kind, 'belief')"
    } else {
        "'belief'"
    };
    let status_select = if has_status {
        "COALESCE(status, 'active')"
    } else {
        "'active'"
    };
    let last_activity_select = if has_last_activity {
        "COALESCE(last_activity, '')"
    } else {
        "''"
    };

    let mut hasher = Sha256::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT id, {}, {}, {} FROM beliefs WHERE {} ORDER BY id",
        kind_select, status_select, last_activity_select, status_filter
    ))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let kind: String = row.get(1)?;
        let status: String = row.get(2)?;
        let last_activity: String = row.get(3)?;
        hasher.update(id.as_bytes());
        hasher.update(b"|");
        hasher.update(kind.as_bytes());
        hasher.update(b"|");
        hasher.update(status.as_bytes());
        hasher.update(b"|");
        hasher.update(last_activity.as_bytes());
        hasher.update(b"\n");
    }
    let source_fingerprint = format!("{:x}", hasher.finalize());

    let source_last_activity = if has_last_activity {
        conn.query_row(
            &format!(
                "SELECT MAX(last_activity) FROM beliefs WHERE {}",
                status_filter
            ),
            [],
            |row| row.get::<_, Option<String>>(0),
        )?
    } else {
        None
    };

    Ok(Some(BeliefInventory {
        belief_count: source_belief_count,
        value_count: source_value_count,
        fingerprint: source_fingerprint,
        last_activity: source_last_activity,
    }))
}

fn read_indexed_belief_inventory(
    project_uid: &str,
    project_path: &Path,
) -> Result<Option<BeliefInventory>> {
    let db_path = crate::paths::mother::graph_db();
    if !db_path.exists() {
        return Ok(None);
    }

    let conn = rusqlite::Connection::open(&db_path)
        .with_context(|| format!("opening mother graph db {}", db_path.display()))?;
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='beliefs'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(None);
    }

    let has_uid = conn
        .prepare("SELECT source_project_uid FROM beliefs LIMIT 0")
        .is_ok();
    let has_kind = conn.prepare("SELECT kind FROM beliefs LIMIT 0").is_ok();
    let has_status = conn.prepare("SELECT status FROM beliefs LIMIT 0").is_ok();
    let has_last_activity = conn
        .prepare("SELECT last_activity FROM beliefs LIMIT 0")
        .is_ok();

    let fallback_source = project_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let (source_filter, source_value) = if has_uid {
        ("source_project_uid", project_uid.to_string())
    } else {
        ("source", fallback_source)
    };

    let status_filter = if has_status {
        "COALESCE(status, 'active') <> 'archived'"
    } else {
        "1=1"
    };
    let belief_kind_filter = if has_kind {
        "(kind IS NULL OR kind <> 'value')"
    } else {
        "1=1"
    };
    let value_kind_filter = if has_kind { "kind = 'value'" } else { "0=1" };

    let indexed_belief_count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM beliefs WHERE {} = ?1 AND {} AND {}",
            source_filter, status_filter, belief_kind_filter
        ),
        rusqlite::params![&source_value],
        |row| row.get(0),
    )?;
    let indexed_value_count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM beliefs WHERE {} = ?1 AND {} AND {}",
            source_filter, status_filter, value_kind_filter
        ),
        rusqlite::params![&source_value],
        |row| row.get(0),
    )?;

    if indexed_belief_count == 0 && indexed_value_count == 0 {
        return Ok(None);
    }

    let kind_select = if has_kind {
        "COALESCE(kind, 'belief')"
    } else {
        "'belief'"
    };
    let status_select = if has_status {
        "COALESCE(status, 'active')"
    } else {
        "'active'"
    };
    let last_activity_select = if has_last_activity {
        "COALESCE(last_activity, '')"
    } else {
        "''"
    };

    let mut hasher = Sha256::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT id, {}, {}, {} FROM beliefs
         WHERE {} = ?1 AND {}
         ORDER BY id",
        kind_select, status_select, last_activity_select, source_filter, status_filter
    ))?;
    let mut rows = stmt.query(rusqlite::params![&source_value])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let kind: String = row.get(1)?;
        let status: String = row.get(2)?;
        let last_activity: String = row.get(3)?;
        hasher.update(id.as_bytes());
        hasher.update(b"|");
        hasher.update(kind.as_bytes());
        hasher.update(b"|");
        hasher.update(status.as_bytes());
        hasher.update(b"|");
        hasher.update(last_activity.as_bytes());
        hasher.update(b"\n");
    }

    let indexed_last_activity = if has_last_activity {
        conn.query_row(
            &format!(
                "SELECT MAX(last_activity) FROM beliefs WHERE {} = ?1 AND {}",
                source_filter, status_filter
            ),
            rusqlite::params![&source_value],
            |row| row.get::<_, Option<String>>(0),
        )?
    } else {
        None
    };

    Ok(Some(BeliefInventory {
        belief_count: indexed_belief_count,
        value_count: indexed_value_count,
        fingerprint: format!("{:x}", hasher.finalize()),
        last_activity: indexed_last_activity,
    }))
}

// =============================================================================
// Detection
// =============================================================================

/// Check if a directory is a patina project
pub fn is_patina_project(path: &Path) -> bool {
    config_path(path).exists() && path.join("layer").is_dir()
}

/// Check if legacy config.json exists
pub fn has_legacy_config(project_path: &Path) -> bool {
    legacy_config_path(project_path).exists()
}

// =============================================================================
// Migration
// =============================================================================

/// Migrate from legacy config.json to unified config.toml
/// Returns true if migration was performed
pub fn migrate_legacy_config(project_path: &Path) -> Result<bool> {
    let json_path = legacy_config_path(project_path);
    if !json_path.exists() {
        return Ok(false);
    }

    // Load existing TOML config (may have [embeddings] section)
    let mut config = load(project_path)?;

    // Read legacy JSON
    let json_content = fs::read_to_string(&json_path)
        .with_context(|| format!("Failed to read legacy config: {}", json_path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&json_content)
        .with_context(|| "Failed to parse legacy config.json")?;

    // Extract fields from JSON
    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
        config.project.name = name.to_string();
    }
    if let Some(created) = json.get("created").and_then(|v| v.as_str()) {
        config.project.created = Some(created.to_string());
    }
    // Note: dev field from legacy config is ignored (dev_env subsystem removed)
    if let Some(llm) = json.get("llm").and_then(|v| v.as_str()) {
        // Map llm to interfaces.default and ensure it's in allowed list
        config.interfaces.default = llm.to_string();
        if !config.interfaces.allowed.contains(&llm.to_string()) {
            config.interfaces.allowed.push(llm.to_string());
        }
    }

    // Extract environment snapshot if present
    if let Some(env) = json.get("environment_snapshot") {
        let os = env.get("os").and_then(|v| v.as_str()).unwrap_or("unknown");
        let arch = env
            .get("arch")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let tools = env
            .get("detected_tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        config.environment = Some(EnvironmentSection {
            os: os.to_string(),
            arch: arch.to_string(),
            detected_tools: tools,
        });
    }

    // Save unified config
    save(project_path, &config)?;

    // Backup and remove legacy config
    backup_file(project_path, &json_path)?;
    fs::remove_file(&json_path)?;

    Ok(true)
}

// =============================================================================
// Config Load/Save
// =============================================================================

/// Load project config from .patina/config.toml
/// Automatically migrates from legacy config.json if needed
pub fn load(project_path: &Path) -> Result<ProjectConfig> {
    let path = config_path(project_path);

    if !path.exists() {
        return Ok(ProjectConfig::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read project config: {}", path.display()))?;

    toml::from_str(&contents)
        .with_context(|| format!("Failed to parse project config: {}", path.display()))
}

/// Load project config with automatic migration
pub fn load_with_migration(project_path: &Path) -> Result<ProjectConfig> {
    // Try migration first (short-circuit: only migrate if legacy config exists)
    if has_legacy_config(project_path) && migrate_legacy_config(project_path)? {
        eprintln!("  ✓ Migrated config.json → config.toml");
    }
    load(project_path)
}

/// Save project config to .patina/config.toml
pub fn save(project_path: &Path, config: &ProjectConfig) -> Result<()> {
    let path = config_path(project_path);

    // Ensure .patina directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, contents)?;
    Ok(())
}

// =============================================================================
// Backup
// =============================================================================

/// Create a backup of a file before modifying it
/// Returns the backup path if a backup was created
pub fn backup_file(project_path: &Path, file_path: &Path) -> Result<Option<PathBuf>> {
    if !file_path.exists() {
        return Ok(None);
    }

    let backups = backups_dir(project_path);
    fs::create_dir_all(&backups)?;

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let backup_path = backups.join(format!("{}-{}", filename, timestamp));

    fs::copy(file_path, &backup_path).with_context(|| {
        format!(
            "Failed to backup {} to {}",
            file_path.display(),
            backup_path.display()
        )
    })?;

    Ok(Some(backup_path))
}

// =============================================================================
// Versioning Control
// =============================================================================

/// Check if versioning is enabled for this project.
///
/// Versioning is enabled when:
/// - No `[upstream]` section exists (local/owned project)
/// - `upstream.remote = "origin"` (owned repo)
///
/// Versioning is disabled when:
/// - `upstream.remote = "upstream"` (fork/contrib project)
///
/// For forks, Cargo.toml version is controlled by upstream,
/// not by `patina version milestone`.
pub fn is_versioning_enabled(project_path: &Path) -> bool {
    let config = match load(project_path) {
        Ok(c) => c,
        Err(_) => return true, // Default to enabled if can't load config
    };

    match &config.upstream {
        None => true,                      // No upstream = owned
        Some(up) => up.remote == "origin", // origin = owned, upstream = fork
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert_eq!(config.project.name, "unnamed");
        // Note: dev section is deprecated and skipped on serialization
        assert_eq!(config.interfaces.default, "claude");
        assert!(config.interfaces.allowed.contains(&"claude".to_string()));
        assert_eq!(config.embeddings.model, "e5-base-v2");
        assert!(config.upstream.is_none()); // No upstream by default (owned repo)
        assert!(config.ci.is_none()); // No CI checks by default
                                      // Retrieval defaults (from Cormack et al. 2009)
        assert_eq!(config.retrieval.rrf_k, 60);
        assert_eq!(config.retrieval.fetch_multiplier, 2);
    }

    #[test]
    fn test_retrieval_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".patina/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        // Test custom retrieval config
        fs::write(
            &config_path,
            "[retrieval]\nrrf_k = 30\nfetch_multiplier = 3\n",
        )
        .unwrap();

        let config = load(tmp.path()).unwrap();
        assert_eq!(config.retrieval.rrf_k, 30);
        assert_eq!(config.retrieval.fetch_multiplier, 3);
    }

    #[test]
    fn test_config_with_name() {
        let config = ProjectConfig::with_name("my-project");
        assert_eq!(config.project.name, "my-project");
    }

    #[test]
    fn test_config_serialization() {
        let config = ProjectConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[project]"));
        // Note: [dev] is deprecated and skipped on serialization
        assert!(!toml_str.contains("[dev]"));
        assert!(toml_str.contains("[interfaces]"));
        assert!(toml_str.contains("[embeddings]"));
    }

    #[test]
    fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let project_path = tmp.path();

        let mut config = ProjectConfig::with_name("test-project");
        config.interfaces.allowed = vec!["claude".to_string(), "gemini".to_string()];

        save(project_path, &config).unwrap();
        let loaded = load(project_path).unwrap();

        assert_eq!(loaded.project.name, "test-project");
        assert_eq!(loaded.interfaces.allowed.len(), 2);
    }

    #[test]
    fn test_load_missing_returns_default() {
        let tmp = TempDir::new().unwrap();
        let config = load(tmp.path()).unwrap();
        assert_eq!(config.project.name, "unnamed");
    }

    #[test]
    fn test_load_partial_config() {
        // Test that loading a config with only [embeddings] works (backward compat)
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".patina/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "[embeddings]\nmodel = \"all-minilm-l6-v2\"\n").unwrap();

        let config = load(tmp.path()).unwrap();
        assert_eq!(config.embeddings.model, "all-minilm-l6-v2");
        // Other sections should have defaults
        assert_eq!(config.project.name, "unnamed");
        assert_eq!(config.interfaces.default, "claude");
    }

    #[test]
    fn test_is_patina_project() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_patina_project(tmp.path()));

        fs::create_dir_all(patina_dir(tmp.path())).unwrap();
        fs::write(config_path(tmp.path()), "").unwrap();
        assert!(!is_patina_project(tmp.path()));

        fs::create_dir_all(tmp.path().join("layer")).unwrap();
        assert!(is_patina_project(tmp.path()));
    }

    #[test]
    fn test_migrate_legacy_config() {
        let tmp = TempDir::new().unwrap();
        let patina = patina_dir(tmp.path());
        fs::create_dir_all(&patina).unwrap();

        // Create legacy config.json
        let json = r#"{
            "name": "test-project",
            "llm": "gemini",
            "dev": "native",
            "created": "2025-01-01T00:00:00Z",
            "environment_snapshot": {
                "os": "linux",
                "arch": "x86_64",
                "detected_tools": ["cargo", "git"]
            }
        }"#;
        fs::write(patina.join("config.json"), json).unwrap();

        // Create existing config.toml with just embeddings
        fs::write(
            patina.join("config.toml"),
            "[embeddings]\nmodel = \"bge-base\"\n",
        )
        .unwrap();

        // Migrate
        let migrated = migrate_legacy_config(tmp.path()).unwrap();
        assert!(migrated);

        // Verify migration
        let config = load(tmp.path()).unwrap();
        assert_eq!(config.project.name, "test-project");
        // Note: dev field from legacy config is ignored (dev_env subsystem removed)
        assert_eq!(config.interfaces.default, "gemini");
        assert!(config.interfaces.allowed.contains(&"gemini".to_string()));
        assert_eq!(config.embeddings.model, "bge-base"); // preserved from existing toml

        // Verify JSON was removed
        assert!(!legacy_config_path(tmp.path()).exists());

        // Verify backup was created
        assert!(backups_dir(tmp.path()).exists());
    }

    #[test]
    fn test_upstream_config() {
        let tmp = TempDir::new().unwrap();
        let project_path = tmp.path();

        // Create config with upstream (contribution mode)
        let mut config = ProjectConfig::with_name("death-mountain");
        config.upstream = Some(UpstreamSection {
            repo: "Provable-Games/death-mountain".to_string(),
            branch: "main".to_string(),
            remote: "upstream".to_string(),
            include_patina: false,
            include_interfaces: false,
        });
        config.ci = Some(CiSection {
            checks: vec!["sozo build".to_string(), "scarb test".to_string()],
            branch_prefix: Some("feat/".to_string()),
        });

        save(project_path, &config).unwrap();
        let loaded = load(project_path).unwrap();

        // Verify upstream
        let upstream = loaded.upstream.unwrap();
        assert_eq!(upstream.repo, "Provable-Games/death-mountain");
        assert_eq!(upstream.branch, "main");
        assert_eq!(upstream.remote, "upstream");
        assert!(!upstream.include_patina);
        assert!(!upstream.include_interfaces);

        // Verify CI
        let ci = loaded.ci.unwrap();
        assert_eq!(ci.checks.len(), 2);
        assert_eq!(ci.branch_prefix, Some("feat/".to_string()));
    }

    #[test]
    fn test_upstream_config_owned_repo() {
        let tmp = TempDir::new().unwrap();
        let project_path = tmp.path();

        // Create config for owned repo (include artifacts)
        let mut config = ProjectConfig::with_name("patina");
        config.upstream = Some(UpstreamSection {
            repo: "nicabar/patina".to_string(),
            branch: "main".to_string(),
            remote: "origin".to_string(), // origin because we own it
            include_patina: true,         // share knowledge
            include_interfaces: true,     // share with collaborators
        });

        save(project_path, &config).unwrap();
        let loaded = load(project_path).unwrap();

        let upstream = loaded.upstream.unwrap();
        assert_eq!(upstream.remote, "origin");
        assert!(upstream.include_patina);
        assert!(upstream.include_interfaces);
    }

    #[test]
    fn test_register_with_mother_creates_default_voice_store() {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let home = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();
        fs::create_dir_all(project.path().join(".patina")).unwrap();
        fs::write(
            project.path().join(".patina/config.toml"),
            "[project]\nname='x'\n",
        )
        .unwrap();

        let old = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", home.path());
        }

        register_with_mother(project.path()).unwrap();

        let voice_dir = home.path().join("mother/voice/default");
        assert!(voice_dir.exists());
        assert!(voice_dir.join("beliefs.db").exists());

        match old {
            Some(value) => unsafe { std::env::set_var("PATINA_HOME", value) },
            None => unsafe { std::env::remove_var("PATINA_HOME") },
        }
    }
}
