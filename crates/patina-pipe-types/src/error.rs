use serde::{Deserialize, Serialize};

/// Typed pipe protocol errors.
///
/// Callers match on the variant, not numeric codes. JSON-RPC error codes
/// (-32001 through -32004) are transport detail — they live in the
/// transport serialization layer (patina-pipe, patina-sdk), not here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PipeError {
    /// Recoverable failure (network timeout, service unavailable).
    Transient {
        message: String,
        retry_after_ms: Option<u64>,
    },
    /// Unrecoverable failure (bad auth, schema not found, invalid config).
    Fatal { message: String },
    /// Source API rate limit hit (HTTP 429).
    RateLimited {
        message: String,
        retry_after_ms: u64,
    },
    /// Some facts emitted before failure. Cursor may be partially advanced.
    Partial { message: String, emitted: u64 },
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipeError::Transient { message, .. } => write!(f, "transient: {}", message),
            PipeError::Fatal { message } => write!(f, "fatal: {}", message),
            PipeError::RateLimited { message, .. } => write!(f, "rate limited: {}", message),
            PipeError::Partial { message, emitted } => {
                write!(f, "partial ({} emitted): {}", emitted, message)
            }
        }
    }
}

impl std::error::Error for PipeError {}
