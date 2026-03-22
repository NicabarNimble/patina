use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use mother_crate::protocol::{Envelope, PROTOCOL_VERSION};

use crate::paths;

pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new() -> Self {
        Self {
            socket_path: paths::serve::socket_path(),
        }
    }

    pub fn ensure_running(&self) -> Result<()> {
        if self.socket_path.exists() && self.health_ping().is_ok() {
            return Ok(());
        }

        let patina_bin = std::env::current_exe().context("getting current executable path")?;
        Command::new(&patina_bin)
            .args(["mother", "start"])
            .env("PATINA_MOTHER_EXTRACTED", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("spawning extracted mother daemon")?;

        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(200));
            if self.socket_path.exists() && self.health_ping().is_ok() {
                return Ok(());
            }
        }

        anyhow::bail!("mother daemon did not become ready")
    }

    pub fn request(&self, action: &str, payload: serde_json::Value) -> Result<serde_json::Value> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("connecting to {}", self.socket_path.display()))?;
        let req = Envelope {
            v: PROTOCOL_VERSION,
            action: Some(action.to_string()),
            payload: Some(payload),
            result: None,
            error: None,
        };

        let line = serde_json::to_string(&req)?;
        stream.write_all(line.as_bytes())?;
        stream.write_all(b"\n")?;

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;
        let response: Envelope = serde_json::from_str(&response_line)
            .with_context(|| format!("invalid daemon response: {}", response_line.trim()))?;

        if let Some(error) = response.error {
            anyhow::bail!(error);
        }
        response
            .result
            .ok_or_else(|| anyhow::anyhow!("daemon response missing result"))
    }

    pub fn context(&self, question: &str) -> Result<String> {
        let result = self.request(
            "context",
            serde_json::json!({
                "question": question,
            }),
        )?;
        result
            .get("response")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("daemon context response missing 'response'"))
    }

    fn health_ping(&self) -> Result<()> {
        let _ = self.request(
            "connect",
            serde_json::json!({
                "agent": "patina-cli",
                "project": std::env::current_dir()?.display().to_string(),
                "persona": serde_json::Value::Null,
            }),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_client_roundtrip_connect() {
        let tmp = tempfile::tempdir().unwrap();
        let socket = tmp.path().join("mother.sock");
        let state = mother_crate::daemon::DaemonState::default();
        let state_for_thread = state.clone();

        let socket_for_thread = socket.clone();
        let handle = std::thread::spawn(move || {
            mother_crate::daemon::serve_one_with_state(&socket_for_thread, &state_for_thread)
        });

        for _ in 0..30 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let client = DaemonClient {
            socket_path: socket,
        };
        let result = client
            .request(
                "connect",
                serde_json::json!({"agent":"test","project":"/tmp/repo","persona":null}),
            )
            .unwrap();
        assert!(result.get("session_id").is_some());
        handle.join().unwrap().unwrap();
    }
}
