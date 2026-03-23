use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use crate::daemon_runner::{run_tcp_server, run_uds_server, TcpServerLaunch, UdsServerLaunch};
use crate::http_routes::Router;
use crate::registry::ChildRegistry;

#[derive(Debug, Clone)]
pub enum TransportMode {
    UdsHttp {
        run_dir: PathBuf,
        socket_path: PathBuf,
        pid_path: PathBuf,
    },
    TcpHttp {
        host: String,
        port: u16,
        token_path: PathBuf,
        token: String,
    },
    UdsJsonLines {
        socket_path: PathBuf,
        pid_path: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct DaemonBootstrapConfig {
    pub transport: TransportMode,
    pub legacy_migration: bool,
}

pub struct DaemonBootstrapRuntime {
    pub mode: RuntimeMode,
}

pub enum RuntimeMode {
    HttpApi {
        registry: Arc<ChildRegistry>,
        router: Arc<Router>,
    },
    JsonLines {
        state: crate::daemon::DaemonState,
    },
}

pub fn start(config: DaemonBootstrapConfig, runtime: DaemonBootstrapRuntime) -> Result<()> {
    match config.transport {
        TransportMode::TcpHttp {
            host,
            port,
            token_path,
            token,
        } => {
            let RuntimeMode::HttpApi { registry, router } = runtime.mode else {
                anyhow::bail!("TcpHttp transport requires HttpApi runtime mode");
            };
            let addr = format!("{}:{}", host, port);
            let listener = std::net::TcpListener::bind(&addr)?;
            run_tcp_server(TcpServerLaunch {
                listener,
                host,
                addr,
                token_path,
                token,
                legacy_migration: config.legacy_migration,
                registry,
                router,
            });
        }
        TransportMode::UdsHttp {
            run_dir,
            socket_path,
            pid_path,
        } => {
            let RuntimeMode::HttpApi { registry, router } = runtime.mode else {
                anyhow::bail!("UdsHttp transport requires HttpApi runtime mode");
            };
            crate::daemon_lifecycle::write_pid_file(&pid_path)?;
            crate::daemon_lifecycle::register_signal_handlers(pid_path, socket_path.clone());
            let listener = crate::socket::setup_unix_listener(&run_dir, &socket_path)?;
            run_uds_server(UdsServerLaunch {
                listener,
                socket_path,
                legacy_migration: config.legacy_migration,
                registry,
                router,
            });
        }
        TransportMode::UdsJsonLines {
            socket_path,
            pid_path,
        } => {
            let RuntimeMode::JsonLines { state } = runtime.mode else {
                anyhow::bail!("UdsJsonLines transport requires JsonLines runtime mode");
            };

            crate::daemon_lifecycle::write_pid_file(&pid_path)?;
            crate::daemon_lifecycle::register_signal_handlers(pid_path, socket_path.clone());

            println!("🚀 Mother daemon starting (extracted mode)...");
            println!("   PID: {}", std::process::id());
            println!("   Listening on {}", socket_path.display());
            println!("   Protocol: JSON-lines over Unix socket");
            println!("   Press Ctrl+C to stop\n");

            crate::daemon::listen_with_state(&socket_path, &state)?;
        }
    }

    Ok(())
}
