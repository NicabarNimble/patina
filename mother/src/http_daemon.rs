use std::io::{Read, Write};
use std::net::Shutdown;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex};

use crate::microserver;

pub const DEFAULT_MAX_BODY_SIZE: usize = 1_048_576;

struct ConnectionPool {
    max: usize,
    active: Mutex<usize>,
    slot_available: Condvar,
}

struct ConnectionPermit {
    pool: Arc<ConnectionPool>,
}

impl ConnectionPool {
    fn new(max: usize) -> Self {
        Self {
            max,
            active: Mutex::new(0),
            slot_available: Condvar::new(),
        }
    }

    fn try_acquire(self: &Arc<Self>) -> Option<ConnectionPermit> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *active < self.max {
            *active += 1;
            Some(ConnectionPermit {
                pool: Arc::clone(self),
            })
        } else {
            None
        }
    }

    fn release(&self) {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *active > 0 {
            *active -= 1;
        }
        self.slot_available.notify_one();
    }
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        self.pool.release();
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }
}

impl HttpResponse {
    pub fn json(status: u16, value: &impl serde::Serialize) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: serde_json::to_vec(value).unwrap_or_default(),
        }
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

pub fn json_error(status: u16, message: &str) -> HttpResponse {
    HttpResponse::json(status, &serde_json::json!({ "error": message }))
}

pub fn with_security_headers(response: HttpResponse) -> HttpResponse {
    response
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("X-Frame-Options", "DENY")
}

fn from_micro(req: microserver::HttpRequest) -> HttpRequest {
    HttpRequest {
        method: req.method,
        path: req.path,
        headers: req.headers,
        body: req.body,
    }
}

fn to_micro(resp: HttpResponse) -> microserver::HttpResponse {
    microserver::HttpResponse {
        status: resp.status,
        headers: resp.headers,
        body: resp.body,
    }
}

pub fn handle_connection(
    stream: &mut (impl Read + Write),
    max_body_size: usize,
    handler: &dyn Fn(HttpRequest) -> HttpResponse,
) {
    let req = match microserver::read_request(stream) {
        Some(Ok(req)) => from_micro(req),
        Some(Err(msg)) => {
            let resp = to_micro(with_security_headers(json_error(400, &msg)));
            microserver::write_response(stream, &resp);
            return;
        }
        None => return,
    };

    let resp = if req.body.len() > max_body_size {
        with_security_headers(json_error(413, "Request too large"))
    } else {
        match panic::catch_unwind(AssertUnwindSafe(|| handler(req))) {
            Ok(resp) => resp,
            Err(payload) => {
                let panic_message = if let Some(msg) = payload.downcast_ref::<&str>() {
                    (*msg).to_string()
                } else if let Some(msg) = payload.downcast_ref::<String>() {
                    msg.clone()
                } else {
                    "unknown panic payload".to_string()
                };
                tracing::error!(panic_message, "http handler panicked");
                with_security_headers(json_error(500, "internal server error"))
            }
        }
    };

    microserver::write_response(stream, &to_micro(resp));
}

pub fn accept_loop_tcp(
    listener: std::net::TcpListener,
    max_body_size: usize,
    max_connections: usize,
    handler: Arc<dyn Fn(HttpRequest) -> HttpResponse + Send + Sync>,
) -> ! {
    let pool = Arc::new(ConnectionPool::new(max_connections));
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let Some(permit) = pool.try_acquire() else {
                    let busy = to_micro(with_security_headers(json_error(503, "Server busy")));
                    microserver::write_response(&mut stream, &busy);
                    let _ = stream.shutdown(Shutdown::Write);
                    continue;
                };
                let handler = Arc::clone(&handler);
                std::thread::spawn(move || {
                    let _permit = permit;
                    handle_connection(&mut stream, max_body_size, handler.as_ref());
                    let _ = stream.shutdown(Shutdown::Write);
                });
            }
            Err(error) => tracing::error!(%error, "tcp accept error"),
        }
    }
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::Arc;

    fn send_request(addr: SocketAddr, path: &str) -> String {
        let mut stream = TcpStream::connect(addr).unwrap();
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n");
        stream.write_all(request.as_bytes()).unwrap();
        stream.shutdown(Shutdown::Write).unwrap();

        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[test]
    fn panicking_handler_returns_500_and_server_keeps_serving() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handler: Arc<dyn Fn(HttpRequest) -> HttpResponse + Send + Sync> = Arc::new(|req| {
            if req.path == "/panic" {
                panic!("panic marker");
            }
            HttpResponse::json(200, &serde_json::json!({"ok": true}))
        });

        let server = std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let mut stream = stream.unwrap();
                handle_connection(&mut stream, DEFAULT_MAX_BODY_SIZE, handler.as_ref());
                let _ = stream.shutdown(Shutdown::Write);
            }
        });

        let first = send_request(addr, "/panic");
        assert!(first.starts_with("HTTP/1.1 500"));
        assert!(first.contains("internal server error"));

        let second = send_request(addr, "/ok");
        assert!(second.starts_with("HTTP/1.1 200"));
        assert!(second.contains("{\"ok\":true}"));

        server.join().unwrap();
    }

    #[test]
    fn pool_plus_one_connection_gets_503() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handler: Arc<dyn Fn(HttpRequest) -> HttpResponse + Send + Sync> = Arc::new(|_| {
            std::thread::sleep(std::time::Duration::from_millis(200));
            HttpResponse::json(200, &serde_json::json!({"ok": true}))
        });

        let server = std::thread::spawn(move || {
            let pool = Arc::new(ConnectionPool::new(1));
            for stream in listener.incoming().take(2) {
                let mut stream = stream.unwrap();
                let handler = Arc::clone(&handler);
                if let Some(permit) = pool.try_acquire() {
                    std::thread::spawn(move || {
                        let _permit = permit;
                        handle_connection(&mut stream, DEFAULT_MAX_BODY_SIZE, handler.as_ref());
                        let _ = stream.shutdown(Shutdown::Write);
                    });
                } else {
                    let busy = to_micro(with_security_headers(json_error(503, "Server busy")));
                    microserver::write_response(&mut stream, &busy);
                    let _ = stream.shutdown(Shutdown::Write);
                }
            }
        });

        let first = std::thread::spawn(move || send_request(addr, "/slow"));
        std::thread::sleep(std::time::Duration::from_millis(25));
        let second = send_request(addr, "/busy");

        assert!(second.starts_with("HTTP/1.1 503"));
        assert!(second.contains("Server busy"));

        let first_resp = first.join().unwrap();
        assert!(first_resp.starts_with("HTTP/1.1 200"));

        server.join().unwrap();
    }
}

pub fn accept_loop_uds(
    listener: std::os::unix::net::UnixListener,
    max_body_size: usize,
    max_connections: usize,
    handler: Arc<dyn Fn(HttpRequest) -> HttpResponse + Send + Sync>,
) -> ! {
    let pool = Arc::new(ConnectionPool::new(max_connections));
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let Some(permit) = pool.try_acquire() else {
                    let busy = to_micro(with_security_headers(json_error(503, "Server busy")));
                    microserver::write_response(&mut stream, &busy);
                    let _ = stream.shutdown(Shutdown::Write);
                    continue;
                };
                let handler = Arc::clone(&handler);
                std::thread::spawn(move || {
                    let _permit = permit;
                    handle_connection(&mut stream, max_body_size, handler.as_ref());
                    let _ = stream.shutdown(Shutdown::Write);
                });
            }
            Err(error) => tracing::error!(%error, "uds accept error"),
        }
    }
    std::process::exit(0);
}
