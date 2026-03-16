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

/// Result returned after a pipe/ingest operation (storage child → Mother).
///
/// Hard specification in [[raw-lake-ingestion]] DESIGN.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    /// Records successfully written.
    pub written: u64,
    /// Records skipped due to dedup (identity field match).
    pub dedup_skipped: u64,
    /// File paths created or appended to.
    #[serde(default)]
    pub files: Vec<String>,
    /// Provenance metadata for the write.
    pub provenance: IngestProvenance,
}

/// Provenance metadata attached to ingested records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestProvenance {
    /// Timestamp of ingestion.
    pub ingested_at: String,
    /// Name of the source connector.
    pub source_connector: String,
    /// Schema version used for validation.
    pub schema_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_result_round_trip_with_cursor() {
        let original = FetchResult {
            emitted: 42,
            cursor: Some("page_token_abc".to_string()),
        };

        let json = serde_json::to_value(&original).unwrap();
        let restored: FetchResult = serde_json::from_value(json).unwrap();

        assert_eq!(restored.emitted, 42);
        assert_eq!(restored.cursor.as_deref(), Some("page_token_abc"));
    }

    #[test]
    fn fetch_result_round_trip_without_cursor() {
        let original = FetchResult {
            emitted: 0,
            cursor: None,
        };

        let json = serde_json::to_value(&original).unwrap();
        // cursor omitted from JSON (skip_serializing_if)
        assert!(json.get("cursor").is_none());

        let restored: FetchResult = serde_json::from_value(json).unwrap();
        assert_eq!(restored.emitted, 0);
        assert!(restored.cursor.is_none());
    }
}
