use anyhow::Result;
use rusqlite::params;

use super::{KnowledgeRuntimeStore, PendingEvent};

pub const KNOWN_STREAMS: &[&str] = &[
    "belief.changed",
    "graph.changed",
    "fact.ingested",
    "session.completed",
    "repo.synced",
];

pub fn list_streams() -> Vec<String> {
    KNOWN_STREAMS.iter().map(|s| s.to_string()).collect()
}

pub fn pull(stream: &str, after_offset: Option<u64>, limit: u32) -> Result<Vec<PendingEvent>> {
    let conn = crate::eventlog::open_events_db()?;
    let after = after_offset.unwrap_or(0) as i64;
    let (predicate, maybe_arg) = match stream {
        "belief.changed" => ("event_type LIKE 'belief.%'".to_string(), None),
        "graph.changed" => ("event_type LIKE 'graph.%'".to_string(), None),
        "fact.ingested" => ("provenance = 'external'".to_string(), None),
        "session.completed" => ("event_type = ?3".to_string(), Some("session.ended")),
        "repo.synced" => ("event_type = ?3".to_string(), Some("repo.synced")),
        _ => anyhow::bail!("unknown stream '{}'", stream),
    };
    let sql = format!(
        "SELECT seq, event_type, data, timestamp FROM eventlog WHERE seq > ?1 AND {} ORDER BY seq LIMIT ?2",
        predicate
    );
    let mut stmt = conn.prepare(&sql)?;
    if let Some(arg) = maybe_arg {
        let rows = stmt.query_map(params![after, limit as i64, arg], |row| {
            let payload_json: String = row.get(2)?;
            Ok(PendingEvent {
                stream: stream.to_string(),
                offset: row.get::<_, i64>(0)? as u64,
                event_type: row.get(1)?,
                payload: serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null),
                occurred_at: row.get(3)?,
            })
        })?;
        return Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?);
    }
    let rows = stmt.query_map(params![after, limit as i64], |row| {
        let payload_json: String = row.get(2)?;
        Ok(PendingEvent {
            stream: stream.to_string(),
            offset: row.get::<_, i64>(0)? as u64,
            event_type: row.get(1)?,
            payload: serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null),
            occurred_at: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

pub fn ack_through(
    store: &KnowledgeRuntimeStore,
    plugin_name: &str,
    stream: &str,
    offset: u64,
) -> Result<()> {
    if !KNOWN_STREAMS.contains(&stream) {
        anyhow::bail!("unknown stream '{}'", stream);
    }
    store.ack_offset(plugin_name, stream, offset)
}
