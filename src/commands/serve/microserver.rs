//! Minimal blocking HTTP/1.1 server over any Read + Write stream.
//!
//! Replaces rouille/tiny_http with ~150 lines of httparse-based parsing.
//! Designed for Unix domain sockets and TCP alike — both implement Read + Write.
//!
//! Intentionally limited surface:
//! - One request per connection (no keep-alive)
//! - No chunked transfer encoding (rejected)
//! - POST requires Content-Length
//! - Header cap: 32 KiB, Body cap: 1 MiB (Read::take, not Content-Length trust)

use std::io::{Read, Write};

/// Maximum header section size (32 KiB)
const MAX_HEADER_SIZE: usize = 32 * 1024;

/// Maximum request body size (1 MiB)
const MAX_BODY_SIZE: usize = 1_048_576;

/// Parsed HTTP request (transport-free)
#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// HTTP response to write back
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Reason phrase for common status codes
fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

/// Read and parse one HTTP request from a stream.
///
/// Returns None if the connection closed before a complete request was received.
/// Returns Some(Err) for malformed requests (caller should write an error response).
pub fn read_request(stream: &mut impl Read) -> Option<Result<HttpRequest, String>> {
    // Read header section with cap
    let mut header_buf = Vec::with_capacity(4096);
    let mut byte = [0u8; 1];

    loop {
        match stream.read(&mut byte) {
            Ok(0) => {
                if header_buf.is_empty() {
                    return None; // clean close
                }
                return Some(Err("Connection closed mid-request".to_string()));
            }
            Ok(_) => {
                header_buf.push(byte[0]);
                if header_buf.len() > MAX_HEADER_SIZE {
                    return Some(Err("Headers too large".to_string()));
                }
                // Check for end of headers (\r\n\r\n)
                if header_buf.len() >= 4 && header_buf[header_buf.len() - 4..] == *b"\r\n\r\n" {
                    break;
                }
            }
            Err(e) => {
                if header_buf.is_empty() {
                    return None; // read error on fresh connection = closed
                }
                return Some(Err(format!("Read error: {}", e)));
            }
        }
    }

    // Parse headers with httparse
    let mut parsed_headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut parsed_headers);

    match req.parse(&header_buf) {
        Ok(httparse::Status::Complete(_)) => {}
        Ok(httparse::Status::Partial) => {
            return Some(Err("Incomplete HTTP request".to_string()));
        }
        Err(e) => {
            return Some(Err(format!("HTTP parse error: {}", e)));
        }
    }

    let method = req.method.unwrap_or("").to_string();
    let path = req.path.unwrap_or("/").to_string();

    let mut headers = Vec::new();
    let mut content_length: Option<usize> = None;
    let mut chunked = false;

    for h in req.headers.iter() {
        let name = h.name.to_string();
        let value = String::from_utf8_lossy(h.value).to_string();

        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = value.trim().parse().ok();
        }
        if name.eq_ignore_ascii_case("Transfer-Encoding")
            && value.to_lowercase().contains("chunked")
        {
            chunked = true;
        }

        headers.push((name, value));
    }

    // Reject chunked encoding
    if chunked {
        return Some(Err("Chunked transfer encoding not supported".to_string()));
    }

    // Read body
    let body = if method == "POST" || method == "PUT" || method == "PATCH" {
        match content_length {
            Some(len) => {
                // Read with cap — do not trust Content-Length for size enforcement
                let read_limit = (MAX_BODY_SIZE + 1).min(len + 1);
                let mut body = Vec::with_capacity(len.min(MAX_BODY_SIZE));
                let bytes_read = stream
                    .take(read_limit as u64)
                    .read_to_end(&mut body)
                    .unwrap_or(0);
                if bytes_read > MAX_BODY_SIZE {
                    return Some(Err("Request body too large".to_string()));
                }
                body
            }
            None => {
                return Some(Err("POST requires Content-Length".to_string()));
            }
        }
    } else {
        Vec::new()
    };

    Some(Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    }))
}

/// Write an HTTP response to a stream.
pub fn write_response(stream: &mut impl Write, response: &HttpResponse) {
    let status_line = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason(response.status)
    );

    let mut header_block = status_line;
    header_block.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    header_block.push_str("Connection: close\r\n");

    for (name, value) in &response.headers {
        header_block.push_str(&format!("{}: {}\r\n", name, value));
    }
    header_block.push_str("\r\n");

    // Write header + body, ignore errors (client may have disconnected)
    let _ = stream.write_all(header_block.as_bytes());
    if !response.body.is_empty() {
        let _ = stream.write_all(&response.body);
    }
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_get_request() {
        let raw = b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut stream = Cursor::new(raw.to_vec());
        let req = read_request(&mut stream).unwrap().unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/health");
        assert!(req.body.is_empty());
    }

    #[test]
    fn test_parse_post_with_body() {
        let body = r#"{"query":"test"}"#;
        let raw = format!(
            "POST /api/scry HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let mut stream = Cursor::new(raw.into_bytes());
        let req = read_request(&mut stream).unwrap().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/api/scry");
        assert_eq!(String::from_utf8_lossy(&req.body), body);
    }

    #[test]
    fn test_reject_chunked() {
        let raw = b"POST /api/scry HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n";
        let mut stream = Cursor::new(raw.to_vec());
        let result = read_request(&mut stream).unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Chunked"));
    }

    #[test]
    fn test_post_requires_content_length() {
        let raw = b"POST /api/scry HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut stream = Cursor::new(raw.to_vec());
        let result = read_request(&mut stream).unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Content-Length"));
    }

    #[test]
    fn test_write_response() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: b"{}".to_vec(),
        };
        let mut buf = Vec::new();
        write_response(&mut buf, &resp);
        let output = String::from_utf8_lossy(&buf);
        assert!(output.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(output.contains("Content-Length: 2\r\n"));
        assert!(output.contains("Connection: close\r\n"));
        assert!(output.contains("Content-Type: application/json\r\n"));
        assert!(output.ends_with("{}"));
    }

    #[test]
    fn test_empty_stream_returns_none() {
        let mut stream = Cursor::new(Vec::<u8>::new());
        assert!(read_request(&mut stream).is_none());
    }

    #[test]
    fn test_headers_too_large() {
        // Generate a header section larger than 32 KiB
        let huge_header = format!(
            "GET / HTTP/1.1\r\nX-Big: {}\r\n\r\n",
            "A".repeat(MAX_HEADER_SIZE)
        );
        let mut stream = Cursor::new(huge_header.into_bytes());
        let result = read_request(&mut stream).unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }
}
