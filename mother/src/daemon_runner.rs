use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

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
}

pub struct UdsServerLaunch {
    pub listener: std::os::unix::net::UnixListener,
    pub socket_path: PathBuf,
    pub registry: Arc<ChildRegistry>,
    pub router: Arc<Router>,
    pub max_connections: usize,
}

pub fn run_tcp_server(launch: TcpServerLaunch) -> ! {
    if launch.host != "127.0.0.1" && launch.host != "localhost" {
        eprintln!(
            "WARNING: Binding to {} exposes the server to the network.",
            launch.host
        );
        eprintln!(
            "  The server has no encryption (HTTP only). Use a reverse proxy for production."
        );
    }

    std::fs::write(&launch.token_path, launch.token.as_bytes())
        .expect("failed to write Mother auth token");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&launch.token_path, std::fs::Permissions::from_mode(0o600))
            .expect("failed to set Mother auth token file permissions");
    }
    eprintln!("Auth token written to {}", launch.token_path.display());

    println!("🚀 Mother daemon starting...");
    println!(
        "   Knowledge children: {} loaded",
        launch.registry.knowledge_len()
    );
    println!("   Listening on http://{}", launch.addr);
    println!("   Press Ctrl+C to stop\n");

    crate::daemon_heartbeat::spawn_heartbeat(Arc::clone(&launch.registry));
    let router = Arc::clone(&launch.router);
    let handler = Arc::new(move |request: HttpRequest| router.route(&request));
    http_daemon::accept_loop_tcp(
        launch.listener,
        DEFAULT_MAX_BODY_SIZE,
        launch.max_connections,
        handler,
    )
}

pub fn run_uds_server(launch: UdsServerLaunch) -> ! {
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

    crate::daemon_heartbeat::spawn_heartbeat(Arc::clone(&launch.registry));
    let router = Arc::clone(&launch.router);
    let handler = Arc::new(move |request: HttpRequest| router.route(&request));
    http_daemon::accept_loop_uds(
        launch.listener,
        DEFAULT_MAX_BODY_SIZE,
        launch.max_connections,
        handler,
    )
}
