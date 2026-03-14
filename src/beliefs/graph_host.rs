use anyhow::Result;
use rusqlite::{params, Connection};

use crate::beliefs::{EdgeType, Graph};
use crate::mother::KnowledgeRuntimeStore;

pub fn query(kind: &str, params_json: &str) -> Result<String> {
    let graph = Graph::open()?;
    let params: serde_json::Value = serde_json::from_str(params_json)?;
    let response = match kind {
        "neighbors" => {
            let node = params
                .get("node")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("neighbors requires 'node'"))?;
            let edge_types = params
                .get("edge_types")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().and_then(EdgeType::parse))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| EdgeType::all().to_vec());
            serde_json::Value::Array(
                graph
                    .get_related(node, &edge_types)?
                    .into_iter()
                    .map(|node| {
                        serde_json::json!({
                            "id": node.id,
                            "node_type": node.node_type.as_str(),
                            "path": node.path.to_string_lossy(),
                            "domains": node.domains,
                            "summary": node.summary,
                            "importance": node.importance,
                        })
                    })
                    .collect(),
            )
        }
        "search" => {
            let query = params
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let conn = Connection::open(crate::paths::mother::graph_db())?;
            let like = format!("%{}%", query);
            let mut stmt = conn.prepare(
                "SELECT id, node_type, path FROM nodes WHERE id LIKE ?1 OR path LIKE ?1 ORDER BY id LIMIT 50",
            )?;
            let rows = stmt
                .query_map(params![like], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "node_type": row.get::<_, String>(1)?,
                        "path": row.get::<_, String>(2)?,
                    }))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            serde_json::Value::Array(rows)
        }
        "project-subgraph" => {
            let node = params
                .get("node")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("project-subgraph requires 'node'"))?;
            let edges = graph.get_edges_from(node)?;
            serde_json::Value::Array(
                edges
                    .into_iter()
                    .map(|edge| {
                        serde_json::json!({
                            "id": edge.id,
                            "from_node": edge.from_node,
                            "to_node": edge.to_node,
                            "edge_type": edge.edge_type.as_str(),
                            "weight": edge.weight,
                            "evidence": edge.evidence,
                        })
                    })
                    .collect(),
            )
        }
        "shortest-path" => serde_json::json!({"status": "not_implemented"}),
        other => anyhow::bail!("unknown graph query '{}'", other),
    };
    Ok(serde_json::to_string(&response)?)
}

pub fn mutate(
    store: &KnowledgeRuntimeStore,
    plugin_name: &str,
    action: &str,
    payload_json: &str,
) -> Result<()> {
    let graph = Graph::open()?;
    let payload: serde_json::Value = serde_json::from_str(payload_json)?;
    match action {
        "link" => {
            let from = payload
                .get("from")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("link requires 'from'"))?;
            let to = payload
                .get("to")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("link requires 'to'"))?;
            let edge_type = payload
                .get("edge_type")
                .and_then(|v| v.as_str())
                .and_then(EdgeType::parse)
                .unwrap_or(EdgeType::Uses);
            let evidence = payload.get("evidence").and_then(|v| v.as_str());
            graph.add_edge(from, to, edge_type, evidence)?;
        }
        "unlink" => {
            let from = payload
                .get("from")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("unlink requires 'from'"))?;
            let to = payload
                .get("to")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("unlink requires 'to'"))?;
            let edge_type = payload
                .get("edge_type")
                .and_then(|v| v.as_str())
                .and_then(EdgeType::parse)
                .unwrap_or(EdgeType::Uses);
            graph.remove_edge(from, to, edge_type)?;
        }
        "weight" => {
            let edge_id = payload
                .get("edge_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("weight requires 'edge_id'"))?;
            let weight = payload
                .get("weight")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| anyhow::anyhow!("weight requires 'weight'"))?;
            graph.set_edge_weight(edge_id, weight as f32)?;
        }
        "tag" => {}
        other => anyhow::bail!("unknown graph action '{}'", other),
    }
    store.record_graph_mutation(plugin_name, action, payload_json)?;
    Ok(())
}
