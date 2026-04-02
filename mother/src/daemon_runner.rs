use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::http_daemon::{self, HttpRequest, DEFAULT_MAX_BODY_SIZE};
use crate::http_routes::Router;
use crate::registry::ChildRegistry;

pub struct TcpServerLaunch {
    pub listener: TcpListener,
    pub host: String,
    pub addr: String,
    pub token_path: PathBuf,
    pub token: String,
    pub registry: Arc<ChildRegistry>,
    pub router: Arc<Router>,
    pub max_connections: usize,
    pub wal_checkpoint_interval_secs: u64,
}

pub struct UdsServerLaunch {
    pub listener: std::os::unix::net::UnixListener,
    pub pid_path: PathBuf,
    pub socket_path: PathBuf,
    pub registry: Arc<ChildRegistry>,
    pub router: Arc<Router>,
    pub max_connections: usize,
    pub wal_checkpoint_interval_secs: u64,
}

fn run_truncate_checkpoint(db_path: &std::path::Path) -> anyhow::Result<()> {
    if !db_path.exists() {
        return Ok(());
    }
    let conn = rusqlite::Connection::open(db_path)?;
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_row| Ok(()))?;
    Ok(())
}

fn checkpoint_all_project_databases_truncate() -> anyhow::Result<()> {
    let runtime = crate::KnowledgeRuntimeStore::default();
    let state_parent = runtime
        .path()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid mother state path"))?;
    let projects = runtime.list_registered_projects()?;

    for project in &projects {
        let project_dir = state_parent.join("projects").join(&project.project_uid);
        for db_name in ["events.db", "patina.db", "runtime.db"] {
            let db_path = project_dir.join(db_name);
            if let Err(error) = run_truncate_checkpoint(&db_path) {
                tracing::warn!(
                    project_uid = %project.project_uid,
                    db = %db_name,
                    %error,
                    "truncate checkpoint during shutdown failed"
                );
            }
        }
    }
    Ok(())
}

fn remove_runtime_file(path: &std::path::Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "failed to remove runtime file")
        }
    }
}

pub fn run_tcp_server(launch: TcpServerLaunch) -> anyhow::Result<()> {
    if launch.host != "127.0.0.1" && launch.host != "localhost" {
        tracing::warn!(host = %launch.host, "binding host exposes mother to the network");
        tracing::warn!("server has no transport encryption; use a reverse proxy in production");
    }

    std::fs::write(&launch.token_path, launch.token.as_bytes())
        .expect("failed to write Mother auth token");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&launch.token_path, std::fs::Permissions::from_mode(0o600))
            .expect("failed to set Mother auth token file permissions");
    }
    tracing::info!(token_path = %launch.token_path.display(), "auth token written");

    println!("🚀 Mother daemon starting...");
    println!(
        "   Knowledge children: {} loaded",
        launch.registry.knowledge_len()
    );
    println!("   Listening on http://{}", launch.addr);
    println!("   Press Ctrl+C to stop\n");

    let heartbeat = crate::daemon_heartbeat::spawn_heartbeat(
        Arc::clone(&launch.registry),
        launch.wal_checkpoint_interval_secs,
    );
    let router = Arc::clone(&launch.router);
    let handler = Arc::new(move |request: HttpRequest| router.route(&request));
    let drain = http_daemon::accept_loop_tcp(
        launch.listener,
        DEFAULT_MAX_BODY_SIZE,
        launch.max_connections,
        crate::daemon_lifecycle::shutdown_flag(),
        handler,
    )?;

    tracing::info!("draining in-flight requests");
    let drained = drain.wait_for_drain(Duration::from_secs(5));
    if !drained {
        tracing::warn!(active = drain.active_connections(), "drain timeout reached");
    }

    checkpoint_all_project_databases_truncate()?;
    heartbeat.stop_and_join();
    tracing::info!("shutdown complete");
    Ok(())
}

pub fn run_uds_server(launch: UdsServerLaunch) -> anyhow::Result<()> {
    println!("🚀 Mother daemon starting...");
    println!("   PID: {}", std::process::id());
    println!(
        "   Knowledge children: {} loaded",
        launch.registry.knowledge_len()
    );
    println!("   Listening on {}", launch.socket_path.display());
    println!(
        "   Test: curl -s --unix-socket {} http://localhost/health",
        launch.socket_path.display()
    );
    println!("   No TCP listener (use --host/--port for network access)");
    println!("   Press Ctrl+C to stop\n");

    let heartbeat = crate::daemon_heartbeat::spawn_heartbeat(
        Arc::clone(&launch.registry),
        launch.wal_checkpoint_interval_secs,
    );
    let router = Arc::clone(&launch.router);
    let handler = Arc::new(move |request: HttpRequest| router.route(&request));
    let drain = http_daemon::accept_loop_uds(
        launch.listener,
        DEFAULT_MAX_BODY_SIZE,
        launch.max_connections,
        crate::daemon_lifecycle::shutdown_flag(),
        handler,
    )?;

    tracing::info!("draining in-flight requests");
    let drained = drain.wait_for_drain(Duration::from_secs(5));
    if !drained {
        tracing::warn!(active = drain.active_connections(), "drain timeout reached");
    }

    checkpoint_all_project_databases_truncate()?;
    heartbeat.stop_and_join();
    remove_runtime_file(&launch.pid_path);
    remove_runtime_file(&launch.socket_path);
    tracing::info!("shutdown complete");
    Ok(())
}
