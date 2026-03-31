use std::io::{Read, Write};
use std::net::Shutdown;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

use crate::microserver;

pub const DEFAULT_MAX_BODY_SIZE: usize = 1_048_576;

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
                eprintln!("HTTP handler panicked: {panic_message}");
                with_security_headers(json_error(500, "internal server error"))
            }
        }
    };

    microserver::write_response(stream, &to_micro(resp));
}

pub fn accept_loop_tcp(
    listener: std::net::TcpListener,
    max_body_size: usize,
    handler: Arc<dyn Fn(HttpRequest) -> HttpResponse + Send + Sync>,
) -> ! {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let handler = Arc::clone(&handler);
                std::thread::spawn(move || {
                    handle_connection(&mut stream, max_body_size, handler.as_ref());
                    let _ = stream.shutdown(Shutdown::Write);
                });
            }
            Err(e) => eprintln!("TCP accept error: {}", e),
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
}

pub fn accept_loop_uds(
    listener: std::os::unix::net::UnixListener,
    max_body_size: usize,
    handler: Arc<dyn Fn(HttpRequest) -> HttpResponse + Send + Sync>,
) -> ! {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let handler = Arc::clone(&handler);
                std::thread::spawn(move || {
                    handle_connection(&mut stream, max_body_size, handler.as_ref());
                    let _ = stream.shutdown(Shutdown::Write);
                });
            }
            Err(e) => eprintln!("UDS accept error: {}", e),
        }
    }
    std::process::exit(0);
}
