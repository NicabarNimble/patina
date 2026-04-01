use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::{registry::ChildRegistry, KnowledgeRuntimeStore};

pub struct HeartbeatHandle {
    stop_requested: Arc<AtomicBool>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl HeartbeatHandle {
    pub fn stop_and_join(mut self) {
        self.stop_requested.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Heartbeat interval in seconds.
const HEARTBEAT_INTERVAL_SECS: u64 = 60;
const DEFAULT_WAL_CHECKPOINT_INTERVAL_SECS: u64 = 300;

fn checkpoint_interval_ticks(interval_secs: u64) -> u64 {
    let secs = interval_secs.max(HEARTBEAT_INTERVAL_SECS);
    secs.div_ceil(HEARTBEAT_INTERVAL_SECS)
}

fn run_passive_checkpoint(db_path: &std::path::Path) -> anyhow::Result<()> {
    if !db_path.exists() {
        return Ok(());
    }
    let conn = rusqlite::Connection::open(db_path)?;
    conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |_row| Ok(()))?;
    Ok(())
}

fn checkpoint_project_databases(runtime: &KnowledgeRuntimeStore) -> anyhow::Result<usize> {
    let state_parent = runtime
        .path()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid mother state path"))?;
    let projects = runtime.list_registered_projects()?;

    let mut checkpointed = 0usize;
    for project in &projects {
        let project_dir = state_parent.join("projects").join(&project.project_uid);
        for db_name in ["events.db", "patina.db", "runtime.db"] {
            let db_path = project_dir.join(db_name);
            match run_passive_checkpoint(&db_path) {
                Ok(()) => {
                    if db_path.exists() {
                        checkpointed += 1;
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        project_uid = %project.project_uid,
                        db = %db_name,
                        %error,
                        "wal checkpoint failed"
                    );
                }
            }
        }
    }

    tracing::info!(
        registered_projects = projects.len(),
        checkpointed_databases = checkpointed,
        "wal checkpoint cycle complete"
    );
    Ok(checkpointed)
}

/// Spawn the heartbeat thread.
///
/// Mother heartbeat advances knowledge children only.
pub fn spawn_heartbeat(
    registry: Arc<ChildRegistry>,
    wal_checkpoint_interval_secs: u64,
) -> HeartbeatHandle {
    let runtime = KnowledgeRuntimeStore::default();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop_requested);
    let checkpoint_interval_secs = if wal_checkpoint_interval_secs == 0 {
        DEFAULT_WAL_CHECKPOINT_INTERVAL_SECS
    } else {
        wal_checkpoint_interval_secs
    };
    let checkpoint_every_ticks = checkpoint_interval_ticks(checkpoint_interval_secs);

    let join_handle = std::thread::Builder::new()
        .name("mother-heartbeat".to_string())
        .spawn(move || {
            let mut ticks = 0u64;
            loop {
                for _ in 0..HEARTBEAT_INTERVAL_SECS {
                    if stop_flag.load(Ordering::Relaxed) {
                        return;
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }

                if stop_flag.load(Ordering::Relaxed) {
                    return;
                }
                if let Err(error) = registry.run_knowledge_cycles(&runtime, "mother-heartbeat") {
                    tracing::warn!(%error, "knowledge-child heartbeat failed");
                }

                ticks += 1;
                if ticks.is_multiple_of(checkpoint_every_ticks) {
                    if let Err(error) = checkpoint_project_databases(&runtime) {
                        tracing::warn!(%error, "wal checkpoint cycle failed");
                    }
                }
            }
        })
        .expect("failed to spawn heartbeat thread");

    HeartbeatHandle {
        stop_requested,
        join_handle: Some(join_handle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ProjectUid;

    #[test]
    fn checkpoint_interval_defaults_to_five_ticks_for_five_minutes() {
        assert_eq!(
            checkpoint_interval_ticks(DEFAULT_WAL_CHECKPOINT_INTERVAL_SECS),
            5
        );
        assert_eq!(checkpoint_interval_ticks(60), 1);
        assert_eq!(checkpoint_interval_ticks(61), 2);
    }

    #[test]
    fn checkpoint_cycle_scans_registered_project_databases() {
        let root = tempfile::tempdir().unwrap();
        let runtime = KnowledgeRuntimeStore::new(root.path().join("mother/state.db"));
        let uid = ProjectUid::new("2bdc808e").unwrap();
        runtime.register_project(&uid, root.path()).unwrap();

        let projects_dir = runtime
            .path()
            .parent()
            .unwrap()
            .join("projects")
            .join(uid.as_str());
        std::fs::create_dir_all(&projects_dir).unwrap();
        for db_name in ["events.db", "patina.db", "runtime.db"] {
            let db_path = projects_dir.join(db_name);
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
            conn.execute(
                "CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY, v TEXT)",
                [],
            )
            .unwrap();
            conn.execute("INSERT INTO t(v) VALUES ('x')", []).unwrap();
        }

        let checkpointed = checkpoint_project_databases(&runtime).unwrap();
        assert_eq!(checkpointed, 3);
    }
}
