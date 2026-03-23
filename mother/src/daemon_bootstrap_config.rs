use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use crate::daemon_runner::{run_tcp_server, run_uds_server, TcpServerLaunch, UdsServerLaunch};
use crate::http_routes::Router;
use crate::registry::ChildRegistry;

#[derive(Debug, Clone)]
pub enum TransportMode {
    Uds {
        run_dir: PathBuf,
        socket_path: PathBuf,
        pid_path: PathBuf,
    },
    Tcp {
        host: String,
        port: u16,
        token_path: PathBuf,
        token: String,
    },
}

#[derive(Debug, Clone)]
pub struct DaemonBootstrapConfig {
    pub transport: TransportMode,
    pub legacy_migration: bool,
}

pub struct DaemonBootstrapRuntime {
    pub registry: Arc<ChildRegistry>,
    pub router: Arc<Router>,
}

pub fn start(config: DaemonBootstrapConfig, runtime: DaemonBootstrapRuntime) -> Result<()> {
    match config.transport {
        TransportMode::Tcp {
            host,
            port,
            token_path,
            token,
        } => {
            let addr = format!("{}:{}", host, port);
            let listener = std::net::TcpListener::bind(&addr)?;
            run_tcp_server(TcpServerLaunch {
                listener,
                host,
                addr,
                token_path,
                token,
                legacy_migration: config.legacy_migration,
                registry: runtime.registry,
                router: runtime.router,
            });
        }
        TransportMode::Uds {
            run_dir,
            socket_path,
            pid_path,
        } => {
            crate::daemon_lifecycle::write_pid_file(&pid_path)?;
            crate::daemon_lifecycle::register_signal_handlers(pid_path, socket_path.clone());
            let listener = crate::socket::setup_unix_listener(&run_dir, &socket_path)?;
            run_uds_server(UdsServerLaunch {
                listener,
                socket_path,
                legacy_migration: config.legacy_migration,
                registry: runtime.registry,
                router: runtime.router,
            });
        }
    }
}
