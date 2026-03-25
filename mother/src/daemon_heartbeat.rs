use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{registry::ChildRegistry, KnowledgeRuntimeStore, Toy};

/// Heartbeat interval in seconds.
const HEARTBEAT_INTERVAL_SECS: u64 = 60;

/// Spawn the heartbeat thread.
///
/// Default Mother runtime only advances knowledge children. Legacy shell-toy
/// children remain behind explicit migration mode.
pub fn spawn_heartbeat(registry: Arc<ChildRegistry>, legacy_migration: bool) {
    let in_flight: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let runtime = KnowledgeRuntimeStore::default();

    std::thread::Builder::new()
        .name("mother-heartbeat".to_string())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
            if let Err(error) = registry.run_knowledge_cycles(&runtime, "mother-heartbeat") {
                eprintln!("[mother] knowledge-child heartbeat failed: {}", error);
            }
            if legacy_migration {
                let toys = registry.tick_legacy_all();
                for toy in toys {
                    let mut flight = in_flight.lock().unwrap_or_else(|e| e.into_inner());
                    if flight.contains(&toy.name) {
                        eprintln!("[mother:toy] skipping '{}' (already in flight)", toy.name);
                        continue;
                    }
                    flight.insert(toy.name.clone());
                    drop(flight);

                    spawn_toy_tracked(toy, Arc::clone(&in_flight));
                }
            }
        })
        .expect("failed to spawn heartbeat thread");
}

fn spawn_toy_tracked(toy: Toy, in_flight: Arc<Mutex<HashSet<String>>>) {
    let toy_name = toy.name.clone();
    let in_flight_thread = Arc::clone(&in_flight);

    match std::thread::Builder::new()
        .name(format!("toy-{}", toy.name))
        .spawn(move || {
            eprintln!(
                "[mother:toy] spawning '{}': {} {:?}",
                toy.name, toy.command, toy.args
            );
            match std::process::Command::new(&toy.command)
                .args(&toy.args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()
            {
                Ok(status) if status.success() => {
                    eprintln!("[mother:toy] '{}' completed successfully", toy.name);
                }
                Ok(status) => {
                    eprintln!("[mother:toy] '{}' failed with {}", toy.name, status);
                }
                Err(e) => {
                    eprintln!("[mother:toy] '{}' failed to spawn: {}", toy.name, e);
                }
            }
            let mut flight = in_flight_thread.lock().unwrap_or_else(|e| e.into_inner());
            flight.remove(&toy.name);
        }) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[mother:toy] thread spawn failed for '{}': {}", toy_name, e);
            let mut flight = in_flight.lock().unwrap_or_else(|e| e.into_inner());
            flight.remove(&toy_name);
        }
    }
}
