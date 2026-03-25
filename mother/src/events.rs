use anyhow::Result;
use rusqlite::{params, Connection};

use crate::{KnowledgeRuntimeStore, PendingEvent};

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

pub fn pull(
    conn: &Connection,
    stream: &str,
    after_offset: Option<u64>,
    limit: u32,
) -> Result<Vec<PendingEvent>> {
    let after = after_offset.unwrap_or(0) as i64;
    match stream {
        "belief.changed" => pull_mutation_log(
            conn,
            stream,
            "belief_mutation_log",
            "belief.changed",
            after,
            limit,
        ),
        "graph.changed" => pull_mutation_log(
            conn,
            stream,
            "graph_mutation_log",
            "graph.changed",
            after,
            limit,
        ),
        "session.completed" => pull_sessions(conn, stream, after, limit),
        "fact.ingested" | "repo.synced" => Ok(Vec::new()),
        _ => anyhow::bail!("unknown stream '{}'", stream),
    }
}

fn pull_mutation_log(
    conn: &Connection,
    stream: &str,
    table: &str,
    event_type: &str,
    after: i64,
    limit: u32,
) -> Result<Vec<PendingEvent>> {
    let sql = format!(
        "SELECT seq, payload_json, created_at FROM {} WHERE seq > ?1 ORDER BY seq LIMIT ?2",
        table
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![after, limit as i64], |row| {
        let payload_json: String = row.get(1)?;
        Ok(PendingEvent {
            stream: stream.to_string(),
            offset: row.get::<_, i64>(0)? as u64,
            event_type: event_type.to_string(),
            payload: serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null),
            occurred_at: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn pull_sessions(
    conn: &Connection,
    stream: &str,
    after: i64,
    limit: u32,
) -> Result<Vec<PendingEvent>> {
    let mut stmt = conn.prepare(
        "SELECT rowid, runtime_id, project_uid, interface_kind, updated_at FROM mother_sessions WHERE rowid > ?1 AND status = 'ended' ORDER BY rowid LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![after, limit as i64], |row| {
        let runtime_id: String = row.get(1)?;
        let project_uid: String = row.get(2)?;
        let interface_kind: String = row.get(3)?;
        let payload = serde_json::json!({
            "runtime_id": runtime_id,
            "project_uid": project_uid,
            "interface_kind": interface_kind,
        });
        Ok(PendingEvent {
            stream: stream.to_string(),
            offset: row.get::<_, i64>(0)? as u64,
            event_type: "session.completed".to_string(),
            payload,
            occurred_at: row.get(4)?,
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
