use std::sync::Arc;
use std::time::Duration;

use crate::{registry::ChildRegistry, KnowledgeRuntimeStore};

/// Heartbeat interval in seconds.
const HEARTBEAT_INTERVAL_SECS: u64 = 60;

/// Spawn the heartbeat thread.
///
/// Mother heartbeat advances knowledge children only.
pub fn spawn_heartbeat(registry: Arc<ChildRegistry>) {
    let runtime = KnowledgeRuntimeStore::default();

    std::thread::Builder::new()
        .name("mother-heartbeat".to_string())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
            if let Err(error) = registry.run_knowledge_cycles(&runtime, "mother-heartbeat") {
                eprintln!("[mother] knowledge-child heartbeat failed: {}", error);
            }
        })
        .expect("failed to spawn heartbeat thread");
}
