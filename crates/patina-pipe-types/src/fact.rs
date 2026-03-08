use serde::{Deserialize, Serialize};

/// A single unit of external evidence entering Patina.
///
/// Facts are transport-agnostic — the same struct travels over WASM host
/// calls (patina-sdk) and stdio JSON-RPC (patina-pipe). Content addressing
/// via `content_hash` enables dedup across sources and runtimes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub schema: String,
    pub fact_type: String,
    pub data: serde_json::Value,
    /// "blake3:<hex>" over canonical JSON of `data`.
    pub content_hash: String,
    /// Stub until persona-federation ships keypair infrastructure.
    #[serde(default)]
    pub signature: String,
}

/// Summary returned after a fetch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub emitted: u64,
    /// Opaque cursor owned by the child. Mother stores and passes back
    /// on next fetch. Could be a timestamp, page token, etag, or
    /// sequence number — Mother never parses it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}
