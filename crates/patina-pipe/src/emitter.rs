//! FactEmitter — streaming fact delivery over stdout.
//!
//! Each `emit()` call writes a pipe/fact JSON-RPC notification immediately.
//! O(1) memory — no accumulation. Content hashing via patina-pipe-types.

use std::io::Write;

use patina_pipe_types::{Fact, PipeError};

use crate::protocol::Notification;

/// Streams facts to stdout as JSON-RPC notifications during pipe/fetch.
///
/// Borrows `&mut W` for the duration of the fetch call. After fetch returns,
/// the caller reclaims the writer for the response.
pub struct FactEmitter<'a, W: Write> {
    writer: &'a mut W,
    count: u64,
}

impl<'a, W: Write> FactEmitter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer, count: 0 }
    }

    /// Emit a single fact as a pipe/fact notification.
    ///
    /// Computes canonical_json + content_hash automatically. Writes to
    /// stdout immediately — O(1) memory, no buffering.
    pub fn emit(
        &mut self,
        schema: &str,
        fact_type: &str,
        data: &serde_json::Value,
    ) -> Result<(), PipeError> {
        let hash = patina_pipe_types::content_hash(data);
        let fact = Fact {
            schema: schema.to_string(),
            fact_type: fact_type.to_string(),
            data: data.clone(),
            content_hash: hash,
            signature: String::new(),
        };

        let notification =
            Notification::fact(serde_json::to_value(&fact).map_err(|e| PipeError::Fatal {
                message: format!("serialize fact: {}", e),
            })?);

        let line = serde_json::to_string(&notification).map_err(|e| PipeError::Fatal {
            message: format!("serialize notification: {}", e),
        })?;

        writeln!(self.writer, "{}", line).map_err(|e| PipeError::Fatal {
            message: format!("write to stdout: {}", e),
        })?;

        self.writer.flush().map_err(|e| PipeError::Fatal {
            message: format!("flush stdout: {}", e),
        })?;

        self.count += 1;
        Ok(())
    }

    /// Number of facts emitted so far.
    pub fn count(&self) -> u64 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn emit_writes_notification() {
        let mut buf = Vec::new();
        {
            let mut emitter = FactEmitter::new(&mut buf);
            emitter
                .emit("github", "issue", &json!({"title": "test"}))
                .unwrap();
            assert_eq!(emitter.count(), 1);
        }
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("\"method\":\"pipe/fact\""));
        assert!(output.contains("\"schema\":\"github\""));
        assert!(output.contains("\"content_hash\":\"blake3:"));
    }

    #[test]
    fn emit_multiple_increments_count() {
        let mut buf = Vec::new();
        {
            let mut emitter = FactEmitter::new(&mut buf);
            emitter.emit("gh", "issue", &json!({"n": 1})).unwrap();
            emitter.emit("gh", "pr", &json!({"n": 2})).unwrap();
            assert_eq!(emitter.count(), 2);
        }
        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.trim().lines().collect();
        assert_eq!(lines.len(), 2);
    }
}
