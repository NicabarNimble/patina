use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use anyhow::Result;
use serde_json::json;

use crate::protocol::{
    ConnectPayload, ContextPayload, Envelope, LakeSyncPayload, PROTOCOL_VERSION,
};

pub fn listen(socket_path: &Path) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    let listener = UnixListener::bind(socket_path)?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = handle_client(&mut stream);
            }
            Err(_) => continue,
        }
    }
    Ok(())
}

pub fn serve_one(socket_path: &Path) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    let listener = UnixListener::bind(socket_path)?;
    if let Ok((mut stream, _)) = listener.accept() {
        let _ = handle_client(&mut stream);
    }
    Ok(())
}

fn handle_client(stream: &mut UnixStream) -> Result<()> {
    let mut line = String::new();
    {
        let mut reader = BufReader::new(stream.try_clone()?);
        if reader.read_line(&mut line)? == 0 {
            return Ok(());
        }
    }

    let parsed: Result<Envelope, _> = serde_json::from_str(&line);
    let response = match parsed {
        Ok(request) if request.v == PROTOCOL_VERSION => route_request(request),
        _ => Envelope::unsupported_version(),
    };

    let serialized = serde_json::to_string(&response)?;
    stream.write_all(serialized.as_bytes())?;
    stream.write_all(b"\n")?;
    Ok(())
}

fn route_request(request: Envelope) -> Envelope {
    let Some(action) = request.action.as_deref() else {
        return Envelope {
            v: PROTOCOL_VERSION,
            action: None,
            payload: None,
            result: None,
            error: Some("missing action".to_string()),
        };
    };

    match handle_action(action, request.payload) {
        Ok(result) => Envelope {
            v: PROTOCOL_VERSION,
            action: None,
            payload: None,
            result: Some(result),
            error: None,
        },
        Err(error) => Envelope {
            v: PROTOCOL_VERSION,
            action: None,
            payload: None,
            result: None,
            error: Some(error),
        },
    }
}

fn handle_action(
    action: &str,
    payload: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    match action {
        "connect" => {
            let payload = payload.ok_or_else(|| "missing payload".to_string())?;
            let connect: ConnectPayload = serde_json::from_value(payload)
                .map_err(|e| format!("invalid connect payload: {}", e))?;
            if let Some(persona) = connect.persona.as_deref() {
                eprintln!(
                    "[mother] connect persona accepted (ignored pre-v1): {}",
                    persona
                );
            }
            Ok(json!({
                "session_id": format!("{}-{}", connect.agent, std::process::id()),
                "children": ["ducklake", "session-writer"],
                "tools": ["context", "lake.sync"],
            }))
        }
        "context" => {
            let payload = payload.ok_or_else(|| "missing payload".to_string())?;
            let context: ContextPayload = serde_json::from_value(payload)
                .map_err(|e| format!("invalid context payload: {}", e))?;
            Ok(json!({
                "response": format!("context not yet implemented for question: {}", context.question),
            }))
        }
        "lake.sync" => {
            let payload = payload.ok_or_else(|| "missing payload".to_string())?;
            let lake_sync: LakeSyncPayload = serde_json::from_value(payload)
                .map_err(|e| format!("invalid lake.sync payload: {}", e))?;
            Ok(json!({"lake": lake_sync.lake, "issues": 0, "prs": 0}))
        }
        other => Err(format!("unsupported action '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::thread;

    #[test]
    fn daemon_returns_unsupported_version_for_bad_envelope() {
        let tmp = tempfile::tempdir().unwrap();
        let socket = tmp.path().join("mother.sock");

        let socket_for_thread = socket.clone();
        let handle = thread::spawn(move || serve_one(&socket_for_thread));

        for _ in 0..30 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let mut stream = UnixStream::connect(&socket).unwrap();
        stream
            .write_all(b"{\"v\":2,\"action\":\"connect\"}\n")
            .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains("unsupported protocol version"));
        handle.join().unwrap().unwrap();
    }

    #[test]
    fn daemon_accepts_connect_and_returns_session_shape() {
        let tmp = tempfile::tempdir().unwrap();
        let socket = tmp.path().join("mother.sock");

        let socket_for_thread = socket.clone();
        let handle = thread::spawn(move || serve_one(&socket_for_thread));

        for _ in 0..30 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let mut stream = UnixStream::connect(&socket).unwrap();
        stream
            .write_all(
                b"{\"v\":1,\"action\":\"connect\",\"payload\":{\"agent\":\"opencode\",\"project\":\"/tmp/repo\",\"persona\":\"dev-bob\"}}\n",
            )
            .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains("session_id"));
        assert!(line.contains("ducklake"));
        assert!(line.contains("session-writer"));
        handle.join().unwrap().unwrap();
    }
}
