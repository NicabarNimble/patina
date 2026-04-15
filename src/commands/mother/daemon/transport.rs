use super::*;

pub(super) fn build_router(state: Arc<ServerState>, require_auth: bool) -> Router {
    let token = state.token.clone();
    let route_table = mother_crate::http_api::build_route_table(state);
    Router::new(require_auth, token, route_table)
}

pub(super) enum WarmupProbe {
    Uds {
        socket_path: PathBuf,
    },
    Tcp {
        host: String,
        port: u16,
        token: String,
    },
}

pub(super) fn parse_http_status(response: &[u8]) -> Option<u16> {
    let status_end = response.iter().position(|&b| b == b'\r')?;
    let first_line = std::str::from_utf8(&response[..status_end]).ok()?;
    first_line.split_whitespace().nth(1)?.parse().ok()
}

pub(super) fn probe_health_uds(socket_path: &Path) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket_path) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    if stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = Vec::new();
    if stream.read_to_end(&mut response).is_err() {
        return false;
    }
    parse_http_status(&response) == Some(200)
}

pub(super) fn probe_health_tcp(host: &str, port: u16, token: &str) -> bool {
    let Ok(mut stream) = TcpStream::connect(format!("{}:{}", host, port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let auth_header = if token.is_empty() {
        String::new()
    } else {
        format!("Authorization: Bearer {}\r\n", token)
    };
    let request =
        format!("GET /health HTTP/1.1\r\nHost: {host}\r\n{auth_header}Connection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut response = Vec::new();
    if stream.read_to_end(&mut response).is_err() {
        return false;
    }
    parse_http_status(&response) == Some(200)
}

pub(super) fn wait_for_health_200(probe: &WarmupProbe, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let is_ready = match probe {
            WarmupProbe::Uds { socket_path } => probe_health_uds(socket_path),
            WarmupProbe::Tcp { host, port, token } => probe_health_tcp(host, *port, token),
        };
        if is_ready {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

pub(super) fn spawn_child_warmup(state: Arc<ServerState>, probe: WarmupProbe) {
    let _ = std::thread::Builder::new()
        .name("mother-child-warmup".to_string())
        .spawn(move || {
            if !wait_for_health_200(&probe, Duration::from_secs(30)) {
                tracing::warn!("health endpoint was not reachable before child warmup");
                state.set_child_warmup_state(
                    "failed",
                    Some("health probe timeout before warmup".to_string()),
                );
                return;
            }

            if let Err(error) = state.warmup_children_now() {
                tracing::warn!(%error, "child warmup failed during background warmup");
            }
        });
}
