use chrono::Utc;
use patina_sdk::toys;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

wit_bindgen::generate!({
    path: "wit",
    world: "folder-watch-actor",
    generate_all,
});

use patina::watch::types as watch_types;

const CONFIG_KEY: &str = "folder-watch.config.v1";
const SNAPSHOT_KEY: &str = "folder-watch.snapshot.v1";
const STATS_KEY: &str = "folder-watch.stats.v1";
const DEFAULT_STREAM: &str = "watch.folder";
const DEFAULT_WATCH_PATH: &str = "/input";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredConfig {
    watch_path: String,
    stream_name: String,
    recursive: bool,
    include_hidden: bool,
    emit_existing_on_start: bool,
    extensions: Vec<String>,
}

impl Default for StoredConfig {
    fn default() -> Self {
        Self {
            watch_path: DEFAULT_WATCH_PATH.to_string(),
            stream_name: DEFAULT_STREAM.to_string(),
            recursive: true,
            include_hidden: false,
            emit_existing_on_start: false,
            extensions: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredStats {
    ticks: u64,
    scans: u64,
    files_seen_total: u64,
    changes_detected_total: u64,
    events_emitted_total: u64,
    last_scan_at: Option<String>,
    last_error: Option<String>,
    last_scan_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredFingerprint {
    size_bytes: u64,
    modified_unix_ms: u64,
    sha256: String,
}

#[derive(Debug, Clone)]
struct ObservedFile {
    absolute_path: String,
    relative_path: String,
    fingerprint: StoredFingerprint,
}

#[derive(Debug, Clone, Deserialize)]
struct ConfigurePatch {
    watch_path: Option<String>,
    stream_name: Option<String>,
    recursive: Option<bool>,
    include_hidden: Option<bool>,
    emit_existing_on_start: Option<bool>,
    extensions: Option<Vec<String>>,
    reset_snapshot: Option<bool>,
}

fn state_bucket() -> Result<toys::keyvalue::Bucket, String> {
    toys::keyvalue::open("default")
}

fn state_get_json<T: for<'de> Deserialize<'de>>(key: &str) -> Option<T> {
    let bucket = state_bucket().ok()?;
    let bytes = bucket.get(key).ok()??;
    let raw = String::from_utf8(bytes).ok()?;
    serde_json::from_str::<T>(&raw).ok()
}

fn state_set_json<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let bucket = state_bucket()?;
    bucket.set(key, &raw)
}

fn stored_to_contract_config(stored: &StoredConfig) -> watch_types::WatcherConfig {
    watch_types::WatcherConfig {
        watch_path: stored.watch_path.clone(),
        stream_name: stored.stream_name.clone(),
        recursive: stored.recursive,
        include_hidden: stored.include_hidden,
        emit_existing_on_start: stored.emit_existing_on_start,
        extensions: stored.extensions.clone(),
    }
}

fn contract_to_stored_config(config: &watch_types::WatcherConfig) -> StoredConfig {
    StoredConfig {
        watch_path: config.watch_path.clone(),
        stream_name: config.stream_name.clone(),
        recursive: config.recursive,
        include_hidden: config.include_hidden,
        emit_existing_on_start: config.emit_existing_on_start,
        extensions: config.extensions.clone(),
    }
}

fn stored_to_contract_stats(stored: &StoredStats) -> watch_types::WatcherStats {
    watch_types::WatcherStats {
        ticks: stored.ticks,
        scans: stored.scans,
        files_seen_total: stored.files_seen_total,
        changes_detected_total: stored.changes_detected_total,
        events_emitted_total: stored.events_emitted_total,
        last_scan_at: stored.last_scan_at.clone(),
        last_error: stored.last_error.clone(),
        last_scan_duration_ms: stored.last_scan_duration_ms,
    }
}

fn contract_to_json_config(config: &watch_types::WatcherConfig) -> serde_json::Value {
    serde_json::json!({
        "watch_path": config.watch_path,
        "stream_name": config.stream_name,
        "recursive": config.recursive,
        "include_hidden": config.include_hidden,
        "emit_existing_on_start": config.emit_existing_on_start,
        "extensions": config.extensions,
    })
}

fn contract_to_json_stats(stats: &watch_types::WatcherStats) -> serde_json::Value {
    serde_json::json!({
        "ticks": stats.ticks,
        "scans": stats.scans,
        "files_seen_total": stats.files_seen_total,
        "changes_detected_total": stats.changes_detected_total,
        "events_emitted_total": stats.events_emitted_total,
        "last_scan_at": stats.last_scan_at,
        "last_error": stats.last_error,
        "last_scan_duration_ms": stats.last_scan_duration_ms,
    })
}

fn contract_to_json_scan(outcome: &watch_types::ScanOutcome) -> serde_json::Value {
    serde_json::json!({
        "files_seen": outcome.files_seen,
        "created": outcome.created,
        "modified": outcome.modified,
        "deleted": outcome.deleted,
        "emitted": outcome.emitted,
        "duration_ms": outcome.duration_ms,
    })
}

fn contract_to_json_change(change: &watch_types::FileChange) -> serde_json::Value {
    let change_kind = match change.change_kind {
        watch_types::ChangeKind::Created => "created",
        watch_types::ChangeKind::Modified => "modified",
        watch_types::ChangeKind::Deleted => "deleted",
    };

    serde_json::json!({
        "watcher": change.watcher,
        "stream": change.stream_name,
        "change_kind": change_kind,
        "absolute_path": change.absolute_path,
        "relative_path": change.relative_path,
        "size_bytes": change.size_bytes,
        "modified_unix_ms": change.modified_unix_ms,
        "sha256": change.sha256,
        "detected_at": change.detected_at,
    })
}

fn load_config() -> StoredConfig {
    state_get_json::<StoredConfig>(CONFIG_KEY).unwrap_or_default()
}

fn save_config(config: &StoredConfig) -> Result<(), String> {
    state_set_json(CONFIG_KEY, config)
}

fn load_snapshot() -> HashMap<String, StoredFingerprint> {
    state_get_json::<HashMap<String, StoredFingerprint>>(SNAPSHOT_KEY).unwrap_or_default()
}

fn save_snapshot(snapshot: &HashMap<String, StoredFingerprint>) -> Result<(), String> {
    state_set_json(SNAPSHOT_KEY, snapshot)
}

fn clear_snapshot() -> Result<(), String> {
    save_snapshot(&HashMap::<String, StoredFingerprint>::new())
}

fn load_stats() -> StoredStats {
    state_get_json::<StoredStats>(STATS_KEY).unwrap_or(StoredStats {
        ticks: 0,
        scans: 0,
        files_seen_total: 0,
        changes_detected_total: 0,
        events_emitted_total: 0,
        last_scan_at: None,
        last_error: None,
        last_scan_duration_ms: None,
    })
}

fn save_stats(stats: &StoredStats) -> Result<(), String> {
    state_set_json(STATS_KEY, stats)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn has_hidden_component(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|part| part.starts_with('.'))
            .unwrap_or(false)
    })
}

fn should_include_file(path: &Path, config: &StoredConfig) -> bool {
    if !config.include_hidden && has_hidden_component(path) {
        return false;
    }

    if config.extensions.is_empty() {
        return true;
    }

    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };

    let ext = ext.to_ascii_lowercase();
    config
        .extensions
        .iter()
        .map(|item| item.trim().trim_start_matches('.').to_ascii_lowercase())
        .any(|allowed| !allowed.is_empty() && allowed == ext)
}

fn fingerprint_for(path: &Path) -> Result<StoredFingerprint, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("failed to stat '{}': {}", path.display(), e))?;
    let bytes = std::fs::read(path)
        .map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;

    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_millis() as u64)
        .unwrap_or(0);

    Ok(StoredFingerprint {
        size_bytes: metadata.len(),
        modified_unix_ms,
        sha256: sha256_hex(&bytes),
    })
}

fn visit_files(
    root: &Path,
    dir: &Path,
    recursive: bool,
    config: &StoredConfig,
    out: &mut HashMap<String, ObservedFile>,
) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("failed to read '{}': {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed reading dir entry: {}", e))?;
        let path = entry.path();

        let file_type = std::fs::symlink_metadata(&path)
            .map_err(|e| format!("failed to stat '{}': {}", path.display(), e))?
            .file_type();

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            if recursive {
                visit_files(root, &path, recursive, config, out)?;
            }
            continue;
        }

        if !file_type.is_file() || !should_include_file(&path, config) {
            continue;
        }

        let fingerprint = fingerprint_for(&path)?;
        let absolute_path = path.to_string_lossy().to_string();
        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        out.insert(
            absolute_path.clone(),
            ObservedFile {
                absolute_path,
                relative_path,
                fingerprint,
            },
        );
    }

    Ok(())
}

fn scan_files(config: &StoredConfig) -> Result<HashMap<String, ObservedFile>, String> {
    let root = PathBuf::from(&config.watch_path);
    if !root.exists() {
        return Err(format!("watch path does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("watch path is not a directory: {}", root.display()));
    }

    let mut out = HashMap::new();
    visit_files(&root, &root, config.recursive, config, &mut out)?;
    Ok(out)
}

fn derive_changes(
    stream: &str,
    previous: &HashMap<String, StoredFingerprint>,
    current: &HashMap<String, ObservedFile>,
    emit_existing_on_start: bool,
) -> Vec<watch_types::FileChange> {
    let mut events = Vec::new();
    let now = Utc::now().to_rfc3339();
    let previous_empty = previous.is_empty();

    let mut current_keys = current.keys().cloned().collect::<Vec<_>>();
    current_keys.sort();

    for key in current_keys {
        let observed = current.get(&key).expect("existing key");
        match previous.get(&key) {
            None => {
                if !previous_empty || emit_existing_on_start {
                    events.push(watch_types::FileChange {
                        watcher: "folder-watch-actor".to_string(),
                        stream_name: stream.to_string(),
                        change_kind: watch_types::ChangeKind::Created,
                        absolute_path: observed.absolute_path.clone(),
                        relative_path: observed.relative_path.clone(),
                        size_bytes: Some(observed.fingerprint.size_bytes),
                        modified_unix_ms: Some(observed.fingerprint.modified_unix_ms),
                        sha256: Some(observed.fingerprint.sha256.clone()),
                        detected_at: now.clone(),
                    });
                }
            }
            Some(old) => {
                if old.sha256 != observed.fingerprint.sha256
                    || old.size_bytes != observed.fingerprint.size_bytes
                    || old.modified_unix_ms != observed.fingerprint.modified_unix_ms
                {
                    events.push(watch_types::FileChange {
                        watcher: "folder-watch-actor".to_string(),
                        stream_name: stream.to_string(),
                        change_kind: watch_types::ChangeKind::Modified,
                        absolute_path: observed.absolute_path.clone(),
                        relative_path: observed.relative_path.clone(),
                        size_bytes: Some(observed.fingerprint.size_bytes),
                        modified_unix_ms: Some(observed.fingerprint.modified_unix_ms),
                        sha256: Some(observed.fingerprint.sha256.clone()),
                        detected_at: now.clone(),
                    });
                }
            }
        }
    }

    let mut old_keys = previous.keys().cloned().collect::<Vec<_>>();
    old_keys.sort();

    for key in old_keys {
        if current.contains_key(&key) {
            continue;
        }

        let relative_path = Path::new(&key)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&key)
            .to_string();

        events.push(watch_types::FileChange {
            watcher: "folder-watch-actor".to_string(),
            stream_name: stream.to_string(),
            change_kind: watch_types::ChangeKind::Deleted,
            absolute_path: key,
            relative_path,
            size_bytes: None,
            modified_unix_ms: None,
            sha256: None,
            detected_at: now.clone(),
        });
    }

    events
}

fn emit_events(
    stream_name: &str,
    events: &[watch_types::FileChange],
) -> Result<(u64, u64, u64, u64), String> {
    if events.is_empty() {
        return Ok((0, 0, 0, 0));
    }

    let client = wasi::messaging::producer::connect(stream_name)?;
    let mut created = 0_u64;
    let mut modified = 0_u64;
    let mut deleted = 0_u64;

    for change in events {
        let topic = match change.change_kind {
            watch_types::ChangeKind::Created => {
                created += 1;
                "file-created"
            }
            watch_types::ChangeKind::Modified => {
                modified += 1;
                "file-modified"
            }
            watch_types::ChangeKind::Deleted => {
                deleted += 1;
                "file-deleted"
            }
        };

        let payload = serde_json::to_vec(&contract_to_json_change(change)).map_err(|e| e.to_string())?;
        let message = wasi::messaging::types::Message {
            topic: topic.to_string(),
            content_type: Some("application/json".to_string()),
            data: payload,
            metadata: vec![
                ("watcher".to_string(), "folder-watch-actor".to_string()),
                ("stream".to_string(), stream_name.to_string()),
            ],
        };
        let _ = wasi::messaging::producer::send(&client, &message)?;
    }

    Ok((events.len() as u64, created, modified, deleted))
}

fn control_configure(
    config: watch_types::WatcherConfig,
    reset_snapshot: bool,
) -> Result<watch_types::WatcherConfig, String> {
    let stored = contract_to_stored_config(&config);
    save_config(&stored)?;
    if reset_snapshot {
        clear_snapshot()?;
    }
    Ok(config)
}

fn control_status() -> watch_types::WatcherStats {
    stored_to_contract_stats(&load_stats())
}

fn scan_and_emit(reason: &str) -> Result<watch_types::ScanOutcome, String> {
    let started = Instant::now();
    let config = load_config();
    let previous = load_snapshot();
    let current = scan_files(&config)?;

    let changes = derive_changes(
        &config.stream_name,
        &previous,
        &current,
        config.emit_existing_on_start,
    );

    let (emitted, created, modified, deleted) = emit_events(&config.stream_name, &changes)?;

    let snapshot = current
        .iter()
        .map(|(path, observed)| {
            (
                path.clone(),
                StoredFingerprint {
                    size_bytes: observed.fingerprint.size_bytes,
                    modified_unix_ms: observed.fingerprint.modified_unix_ms,
                    sha256: observed.fingerprint.sha256.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    save_snapshot(&snapshot)?;

    let duration_ms = started.elapsed().as_millis() as u64;

    let mut stats = load_stats();
    stats.scans += 1;
    stats.files_seen_total += current.len() as u64;
    stats.changes_detected_total += changes.len() as u64;
    stats.events_emitted_total += emitted;
    stats.last_scan_at = Some(Utc::now().to_rfc3339());
    stats.last_scan_duration_ms = Some(duration_ms);
    stats.last_error = None;
    save_stats(&stats)?;

    let _ = toys::measure::counter("files_seen", current.len() as f64);
    let _ = toys::measure::counter("changes_detected", changes.len() as f64);
    let _ = toys::measure::counter("events_emitted", emitted as f64);
    let _ = toys::measure::gauge("scan_duration_ms", duration_ms as f64);

    toys::log::info(
        "folder-watch-actor",
        &format!(
            "{} scan: seen={} created={} modified={} deleted={} emitted={} duration_ms={}",
            reason,
            current.len(),
            created,
            modified,
            deleted,
            emitted,
            duration_ms,
        ),
    );

    Ok(watch_types::ScanOutcome {
        files_seen: current.len() as u64,
        created,
        modified,
        deleted,
        emitted,
        duration_ms,
    })
}

fn control_scan_now() -> Result<watch_types::ScanOutcome, String> {
    scan_and_emit("scan-now")
}

fn control_reset() -> Result<(), String> {
    clear_snapshot()?;
    save_stats(&StoredStats {
        ticks: 0,
        scans: 0,
        files_seen_total: 0,
        changes_detected_total: 0,
        events_emitted_total: 0,
        last_scan_at: None,
        last_error: None,
        last_scan_duration_ms: None,
    })
}

impl exports::patina::watch::control::Guest for FolderWatchActor {
    fn configure(
        config: watch_types::WatcherConfig,
        reset_snapshot: bool,
    ) -> Result<watch_types::WatcherConfig, String> {
        control_configure(config, reset_snapshot)
    }

    fn status() -> watch_types::WatcherStats {
        control_status()
    }

    fn scan_now() -> Result<watch_types::ScanOutcome, String> {
        control_scan_now()
    }

    fn reset() -> Result<(), String> {
        control_reset()
    }
}

fn handle_configure(payload: &str) -> Result<String, String> {
    let patch: ConfigurePatch = if payload.trim().is_empty() || payload.trim() == "{}" {
        ConfigurePatch {
            watch_path: None,
            stream_name: None,
            recursive: None,
            include_hidden: None,
            emit_existing_on_start: None,
            extensions: None,
            reset_snapshot: None,
        }
    } else {
        serde_json::from_str(payload)
            .map_err(|e| format!("folder-watch-actor configure payload invalid: {}", e))?
    };

    let mut stored = load_config();
    if let Some(value) = patch.watch_path {
        stored.watch_path = value;
    }
    if let Some(value) = patch.stream_name {
        stored.stream_name = value;
    }
    if let Some(value) = patch.recursive {
        stored.recursive = value;
    }
    if let Some(value) = patch.include_hidden {
        stored.include_hidden = value;
    }
    if let Some(value) = patch.emit_existing_on_start {
        stored.emit_existing_on_start = value;
    }
    if let Some(value) = patch.extensions {
        stored.extensions = value;
    }

    let contract = stored_to_contract_config(&stored);
    let applied = control_configure(contract, patch.reset_snapshot.unwrap_or(false))?;

    Ok(serde_json::json!({
        "status": "ok",
        "configure_contract": "patina:watch/control.configure",
        "config": contract_to_json_config(&applied),
    })
    .to_string())
}

fn handle_status() -> Result<String, String> {
    let status = control_status();
    let config = stored_to_contract_config(&load_config());

    Ok(serde_json::json!({
        "name": "folder-watch-actor",
        "status_contract": "patina:watch/control.status",
        "config": contract_to_json_config(&config),
        "stats": contract_to_json_stats(&status),
        "snapshot_entries": load_snapshot().len(),
    })
    .to_string())
}

fn handle_scan_now() -> Result<String, String> {
    let outcome = control_scan_now()?;
    Ok(serde_json::json!({
        "status": "ok",
        "scan_contract": "patina:watch/control.scan-now",
        "outcome": contract_to_json_scan(&outcome),
    })
    .to_string())
}

fn handle_reset() -> Result<String, String> {
    control_reset()?;
    Ok(serde_json::json!({
        "status": "ok",
        "reset_contract": "patina:watch/control.reset",
    })
    .to_string())
}

struct FolderWatchActor;

impl Guest for FolderWatchActor {
    fn init() {}

    fn name() -> String {
        "folder-watch-actor".to_string()
    }

    fn on_load() -> Result<(), String> {
        if state_get_json::<StoredConfig>(CONFIG_KEY).is_none() {
            save_config(&StoredConfig::default())?;
        }
        if state_get_json::<StoredStats>(STATS_KEY).is_none() {
            save_stats(&StoredStats {
                ticks: 0,
                scans: 0,
                files_seen_total: 0,
                changes_detected_total: 0,
                events_emitted_total: 0,
                last_scan_at: None,
                last_error: None,
                last_scan_duration_ms: None,
            })?;
        }
        toys::log::info(
            "folder-watch-actor",
            "loaded (non-legacy sdk + WIT business contract lane)",
        );
        Ok(())
    }

    fn on_unload() {}

    fn health() -> patina::child::runtime_types::ChildHealth {
        let stats = load_stats();
        patina::child::runtime_types::ChildHealth {
            status: patina::child::runtime_types::HealthStatus::Healthy,
            reason: Some(format!(
                "scans={} emitted={} last_error={}",
                stats.scans,
                stats.events_emitted_total,
                stats.last_error.unwrap_or_else(|| "none".to_string())
            )),
        }
    }

    fn handle(action: String, payload: String) -> Result<String, String> {
        match action.as_str() {
            "configure" => handle_configure(&payload),
            "status" => handle_status(),
            "scan-now" => handle_scan_now(),
            "reset" => handle_reset(),
            other => Err(format!("folder-watch-actor: unknown action '{}'", other)),
        }
    }

    fn drain(_limit: u32) -> Result<Vec<patina::child::runtime_types::PendingEvent>, String> {
        Ok(vec![])
    }

    fn tick() -> Vec<patina::child::runtime_types::TaskIntent> {
        let mut stats = load_stats();
        stats.ticks += 1;

        match scan_and_emit("tick") {
            Ok(_) => {
                let _ = save_stats(&stats);
            }
            Err(error) => {
                stats.last_error = Some(error.clone());
                let _ = save_stats(&stats);
                toys::log::warn("folder-watch-actor", &format!("tick failed: {}", error));
            }
        }

        vec![]
    }
}

export!(FolderWatchActor);
