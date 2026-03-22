use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use serde_json::json;

use crate::protocol::{
    ConnectPayload, ContextPayload, Envelope, LakeSyncPayload, MeasurePayload, SpecPayload,
    PROTOCOL_VERSION,
};

#[derive(Debug, Default, Clone)]
pub struct DaemonState {
    disconnected_sessions: Arc<Mutex<Vec<String>>>,
}

impl DaemonState {
    pub fn record_disconnect(&self, session_id: &str) {
        if let Ok(mut guard) = self.disconnected_sessions.lock() {
            guard.push(session_id.to_string());
        }
    }

    pub fn disconnected_sessions(&self) -> Vec<String> {
        self.disconnected_sessions
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }
}

pub fn listen(socket_path: &Path) -> Result<()> {
    let state = DaemonState::default();
    listen_with_state(socket_path, &state)
}

pub fn listen_with_state(socket_path: &Path, state: &DaemonState) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    let listener = UnixListener::bind(socket_path)?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = handle_client(&mut stream, state);
            }
            Err(_) => continue,
        }
    }
    Ok(())
}

pub fn serve_one(socket_path: &Path) -> Result<()> {
    let state = DaemonState::default();
    serve_one_with_state(socket_path, &state)
}

pub fn serve_one_with_state(socket_path: &Path, state: &DaemonState) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    let listener = UnixListener::bind(socket_path)?;
    if let Ok((mut stream, _)) = listener.accept() {
        let _ = handle_client(&mut stream, state);
    }
    Ok(())
}

fn handle_client(stream: &mut UnixStream, state: &DaemonState) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut active_session_id: Option<String> = None;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            if let Some(session_id) = active_session_id.as_deref() {
                state.record_disconnect(session_id);
            }
            return Ok(());
        }

        let parsed: Result<Envelope, _> = serde_json::from_str(&line);
        let response = match parsed {
            Ok(request) if request.v == PROTOCOL_VERSION => {
                let response = route_request(request);
                if let Some(result) = response.result.as_ref() {
                    if let Some(session_id) = result.get("session_id").and_then(|v| v.as_str()) {
                        active_session_id = Some(session_id.to_string());
                    }
                }
                response
            }
            _ => Envelope::unsupported_version(),
        };

        let serialized = serde_json::to_string(&response)?;
        stream.write_all(serialized.as_bytes())?;
        stream.write_all(b"\n")?;
    }
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
                "tools": ["context", "lake.sync", "measure", "spec"],
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
        "measure" => {
            let payload = payload.ok_or_else(|| "missing payload".to_string())?;
            let measure: MeasurePayload = serde_json::from_value(payload)
                .map_err(|e| format!("invalid measure payload: {}", e))?;
            Ok(json!({
                "output": format!(
                    "measure daemon path not yet implemented (system={}, json={}, full={}, verb={})",
                    measure.system,
                    measure.json,
                    measure.full,
                    measure.verb.unwrap_or_else(|| "none".to_string())
                )
            }))
        }
        "spec" => {
            let payload = payload.ok_or_else(|| "missing payload".to_string())?;
            let spec: SpecPayload = serde_json::from_value(payload)
                .map_err(|e| format!("invalid spec payload: {}", e))?;
            Ok(json!({
                "output": format!(
                    "spec daemon path not yet implemented (subcommand={}, id={}, status={}, target={}, json={}, handoff={})",
                    spec.subcommand,
                    spec.id.unwrap_or_else(|| "none".to_string()),
                    spec.status.unwrap_or_else(|| "none".to_string()),
                    spec.target.unwrap_or_else(|| "none".to_string()),
                    spec.json,
                    spec.handoff,
                )
            }))
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
        drop(reader);
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
        drop(reader);
        handle.join().unwrap().unwrap();
    }

    #[test]
    fn daemon_records_disconnect_after_connect_eof() {
        let tmp = tempfile::tempdir().unwrap();
        let socket = tmp.path().join("mother.sock");
        let state = DaemonState::default();
        let state_for_thread = state.clone();

        let socket_for_thread = socket.clone();
        let handle =
            thread::spawn(move || serve_one_with_state(&socket_for_thread, &state_for_thread));

        for _ in 0..30 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let mut stream = UnixStream::connect(&socket).unwrap();
        stream
            .write_all(
                b"{\"v\":1,\"action\":\"connect\",\"payload\":{\"agent\":\"opencode\",\"project\":\"/tmp/repo\",\"persona\":null}}\n",
            )
            .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        drop(reader);

        handle.join().unwrap().unwrap();
        let disconnected = state.disconnected_sessions();
        assert_eq!(disconnected.len(), 1);
        assert!(disconnected[0].starts_with("opencode-"));
    }
}
