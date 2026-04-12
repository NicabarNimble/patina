use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use crate::daemon_runner::{run_tcp_server, run_uds_server, TcpServerLaunch, UdsServerLaunch};
use crate::http_routes::Router;
use crate::registry::ChildRegistry;

pub const DEFAULT_MAX_CONNECTIONS: usize = 16;
pub const DEFAULT_WAL_CHECKPOINT_INTERVAL_SECS: u64 = 300;

#[cfg(not(test))]
fn mother_logs_dir() -> PathBuf {
    let home = std::env::var_os("PATINA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".patina")
        });
    home.join("mother").join("logs")
}

#[cfg(not(test))]
pub fn ensure_logging_initialized() -> Result<()> {
    static LOGGING_INITIALIZED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if LOGGING_INITIALIZED.get().is_some() {
        return Ok(());
    }

    let log_dir = mother_logs_dir();
    std::fs::create_dir_all(&log_dir)?;
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("mother.jsonl"))?;

    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::sync::Mutex::new(log_file))
        .finish();

    if let Err(error) = tracing::subscriber::set_global_default(subscriber) {
        let message = error.to_string();
        if !message.contains("already been set") {
            return Err(anyhow::anyhow!(
                "failed to set tracing subscriber: {}",
                error
            ));
        }
    }

    let _ = LOGGING_INITIALIZED.set(());
    Ok(())
}

#[cfg(test)]
pub fn ensure_logging_initialized() -> Result<()> {
    Ok(())
}

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
}

#[derive(Debug, Clone)]
pub struct DaemonBootstrapConfig {
    pub transport: TransportMode,
    pub max_connections: usize,
    pub wal_checkpoint_interval_secs: u64,
}

pub struct DaemonBootstrapRuntime {
    pub registry: Arc<ChildRegistry>,
    pub router: Arc<Router>,
}

#[allow(unreachable_code)]
pub fn start(config: DaemonBootstrapConfig, runtime: DaemonBootstrapRuntime) -> Result<()> {
    ensure_logging_initialized()?;

    crate::daemon_lifecycle::register_signal_handlers();

    let DaemonBootstrapConfig {
        transport,
        max_connections,
        wal_checkpoint_interval_secs,
    } = config;
    match transport {
        TransportMode::TcpHttp {
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
                registry: runtime.registry,
                router: runtime.router,
                max_connections,
                wal_checkpoint_interval_secs,
            })?;
        }
        TransportMode::UdsHttp {
            run_dir,
            socket_path,
            pid_path,
        } => {
            crate::daemon_lifecycle::reconcile_pid_state(&pid_path, &socket_path)?;
            crate::daemon_lifecycle::write_pid_file(&pid_path)?;
            let listener = crate::socket::setup_unix_listener(&run_dir, &socket_path)?;
            run_uds_server(UdsServerLaunch {
                listener,
                pid_path,
                socket_path,
                registry: runtime.registry,
                router: runtime.router,
                max_connections,
                wal_checkpoint_interval_secs,
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn tcp_http_returns_error_when_port_in_use() {
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_port = occupied.local_addr().unwrap().port();
        let config = DaemonBootstrapConfig {
            transport: TransportMode::TcpHttp {
                host: "127.0.0.1".to_string(),
                port: occupied_port,
                token_path: std::path::PathBuf::from("/tmp/patina-token"),
                token: "test-token".to_string(),
            },
            max_connections: DEFAULT_MAX_CONNECTIONS,
            wal_checkpoint_interval_secs: DEFAULT_WAL_CHECKPOINT_INTERVAL_SECS,
        };
        let runtime = DaemonBootstrapRuntime {
            registry: Arc::new(crate::registry::ChildRegistry::new()),
            router: Arc::new(crate::http_routes::Router::new(
                false,
                String::new(),
                crate::http_routes::RouteTable {
                    get_health: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"ok": true}),
                        )
                    }),
                    get_version: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"version": "test"}),
                        )
                    }),
                    get_atlas_snapshot: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"summary": {"spec_count": 0}}),
                        )
                    }),
                    post_scry: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"results": []}),
                        )
                    }),
                    post_federation_status: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_federation_refresh: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_federation_query: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    get_secrets_cache: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_secrets_cache: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_secrets_lock: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_pando_registry_init: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"protocol_version": 2, "pandos": []}),
                        )
                    }),
                    get_pando_list: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"protocol_version": 2, "pandos": []}),
                        )
                    }),
                    post_lifecycle_load_pando: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_lifecycle_refresh: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_lifecycle_reload_child: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    child_request: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            404,
                            &serde_json::json!({"error": "no child"}),
                        )
                    }),
                },
            )),
        };

        let error = start(config, runtime).unwrap_err();
        assert!(error.to_string().to_lowercase().contains("in use"));
    }

    #[test]
    fn uds_http_returns_error_for_insecure_runtime_dir_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::TempDir::new().unwrap();
        let run_dir = temp.path().join("run");
        std::fs::create_dir_all(&run_dir).unwrap();
        std::fs::set_permissions(&run_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        let config = DaemonBootstrapConfig {
            transport: TransportMode::UdsHttp {
                run_dir: run_dir.clone(),
                socket_path: run_dir.join("serve.sock"),
                pid_path: run_dir.join("serve.pid"),
            },
            max_connections: DEFAULT_MAX_CONNECTIONS,
            wal_checkpoint_interval_secs: DEFAULT_WAL_CHECKPOINT_INTERVAL_SECS,
        };
        let runtime = DaemonBootstrapRuntime {
            registry: Arc::new(crate::registry::ChildRegistry::new()),
            router: Arc::new(crate::http_routes::Router::new(
                false,
                String::new(),
                crate::http_routes::RouteTable {
                    get_health: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"ok": true}),
                        )
                    }),
                    get_version: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"version": "test"}),
                        )
                    }),
                    get_atlas_snapshot: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"summary": {"spec_count": 0}}),
                        )
                    }),
                    post_scry: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"results": []}),
                        )
                    }),
                    post_federation_status: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_federation_refresh: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_federation_query: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    get_secrets_cache: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_secrets_cache: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_secrets_lock: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_pando_registry_init: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"protocol_version": 2, "pandos": []}),
                        )
                    }),
                    get_pando_list: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            200,
                            &serde_json::json!({"protocol_version": 2, "pandos": []}),
                        )
                    }),
                    post_lifecycle_load_pando: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_lifecycle_refresh: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    post_lifecycle_reload_child: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(200, &serde_json::json!({}))
                    }),
                    child_request: Arc::new(|_| {
                        crate::http_daemon::HttpResponse::json(
                            404,
                            &serde_json::json!({"error": "no child"}),
                        )
                    }),
                },
            )),
        };

        let error = start(config, runtime).unwrap_err();
        assert!(error.to_string().contains("group/world accessible"));
    }
}
