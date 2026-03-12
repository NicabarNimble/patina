use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use super::KnowledgeRuntimeStore;
use crate::mother::Graph;

fn graph_conn() -> Result<Connection> {
    Ok(Connection::open(crate::paths::mother::graph_db())?)
}

pub fn query(kind: &str, params_json: &str) -> Result<String> {
    let graph = Graph::open()?;
    let params: serde_json::Value = serde_json::from_str(params_json)?;
    let response = match kind {
        "search" => {
            let query = params
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            serde_json::to_value(graph.search_beliefs(query, limit)?)?
        }
        "by-id" => {
            let belief_id = params
                .get("belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("by-id requires 'belief_id'"))?;
            let source = params
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if source.is_empty() {
                let conn = graph_conn()?;
                let mut stmt = conn
                    .prepare("SELECT source FROM beliefs WHERE id = ?1 ORDER BY source LIMIT 1")?;
                if let Some(source) = stmt
                    .query_row(params![belief_id], |row| row.get::<_, String>(0))
                    .optional()?
                {
                    let (entry, path) = graph
                        .get_belief(belief_id, &source)?
                        .ok_or_else(|| anyhow::anyhow!("belief '{}' not found", belief_id))?;
                    serde_json::json!({"belief": entry, "source_path": path})
                } else {
                    serde_json::Value::Null
                }
            } else {
                let (entry, path) = graph
                    .get_belief(belief_id, source)?
                    .ok_or_else(|| anyhow::anyhow!("belief '{}' not found", belief_id))?;
                serde_json::json!({"belief": entry, "source_path": path})
            }
        }
        "related" => {
            let belief_id = params
                .get("belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("related requires 'belief_id'"))?;
            serde_json::json!({
                "supports": graph.query_supports(belief_id)?,
                "attacks": graph.query_attacks(belief_id)?,
                "projects": graph.query_projects(belief_id)?,
                "applied_in": graph.query_belief_applied_in(belief_id)?,
            })
        }
        "pending-verification" => {
            let conn = graph_conn()?;
            let runtime = Connection::open(crate::paths::mother::runtime_db())?;
            let mut stmt = conn.prepare(
                "SELECT id, source, statement FROM beliefs ORDER BY last_activity DESC NULLS LAST, id LIMIT 50",
            )?;
            let beliefs = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut pending = Vec::new();
            for (belief_id, source, statement) in beliefs {
                let verification_count: i64 = runtime.query_row(
                    "SELECT COUNT(*) FROM belief_verifications WHERE belief_id = ?1",
                    params![belief_id],
                    |row| row.get(0),
                )?;
                if verification_count == 0 {
                    pending.push(serde_json::json!({
                        "belief_id": belief_id,
                        "source": source,
                        "statement": statement,
                    }));
                }
            }
            serde_json::Value::Array(pending)
        }
        other => anyhow::bail!("unknown belief query '{}'", other),
    };
    Ok(serde_json::to_string(&response)?)
}

pub fn mutate(
    store: &KnowledgeRuntimeStore,
    plugin_name: &str,
    action: &str,
    payload_json: &str,
) -> Result<()> {
    let payload: serde_json::Value = serde_json::from_str(payload_json)?;
    match action {
        "attach-evidence" => {
            let belief_id = payload
                .get("belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("attach-evidence requires 'belief_id'"))?;
            let evidence_id = payload
                .get("evidence_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("attach-evidence requires 'evidence_id'"))?;
            store.attach_belief_evidence(
                plugin_name,
                belief_id,
                evidence_id,
                Some(payload_json),
            )?;
        }
        "record-verification" => {
            let belief_id = payload
                .get("belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("record-verification requires 'belief_id'"))?;
            let status = payload
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("passed");
            let evidence_json = payload
                .get("evidence_ids")
                .map(serde_json::to_string)
                .transpose()?;
            let notes = payload.get("notes").and_then(|v| v.as_str());
            store.record_belief_verification(
                plugin_name,
                belief_id,
                status,
                evidence_json.as_deref(),
                notes,
            )?;
            let conn = graph_conn()?;
            let (field, delta) = match status {
                "passed" => ("verification_passed", "1"),
                "failed" => ("verification_failed", "1"),
                _ => ("verification_errored", "1"),
            };
            conn.execute(
                &format!(
                    "UPDATE beliefs SET verification_total = verification_total + 1, {} = {} + {} WHERE id = ?1",
                    field, field, delta
                ),
                params![belief_id],
            )?;
        }
        "link-related" => {
            let belief_id = payload
                .get("belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("link-related requires 'belief_id'"))?;
            let related_belief_id = payload
                .get("related_belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("link-related requires 'related_belief_id'"))?;
            store.link_beliefs(plugin_name, belief_id, related_belief_id, "related")?;
        }
        "supersede" => {
            let belief_id = payload
                .get("belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("supersede requires 'belief_id'"))?;
            let superseded_belief_id = payload
                .get("superseded_belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("supersede requires 'superseded_belief_id'"))?;
            store.link_beliefs(plugin_name, belief_id, superseded_belief_id, "supersedes")?;
        }
        other => anyhow::bail!("unknown belief action '{}'", other),
    }
    store.record_belief_mutation(plugin_name, action, payload_json)?;
    Ok(())
}
