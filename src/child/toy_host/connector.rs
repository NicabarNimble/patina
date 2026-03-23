//! Connector toy host helpers.

use crate::mother::KnowledgeRuntimeStore;
use serde::{Deserialize, Serialize};

const CONNECTOR_BINDING_PREFIX: &str = "connector:binding:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorBindingRecord {
    pub binding_id: String,
    pub connection: String,
    pub owner: String,
    pub repo: String,
    pub types: Vec<String>,
}

pub fn ensure_granted(connector_granted: bool, plugin_name: &str) -> Result<(), String> {
    if connector_granted {
        Ok(())
    } else {
        Err(format!(
            "connector toy not granted for plugin '{}'",
            plugin_name
        ))
    }
}

pub fn list_bindings(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
) -> Result<Vec<ConnectorBindingRecord>, String> {
    let keys = runtime
        .list_state_prefix(plugin_name, CONNECTOR_BINDING_PREFIX)
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for key in keys {
        let Some(raw) = runtime
            .get_state(plugin_name, &key)
            .map_err(|e| e.to_string())?
        else {
            continue;
        };
        let parsed: ConnectorBindingRecord = serde_json::from_str(&raw)
            .map_err(|e| format!("invalid connector binding '{}': {}", key, e))?;
        out.push(parsed);
    }
    Ok(out)
}

pub fn upsert_binding(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    binding: ConnectorBindingRecord,
) -> Result<ConnectorBindingRecord, String> {
    validate_binding(&binding)?;
    let key = binding_key(&binding.binding_id);
    let payload = serde_json::to_string(&binding).map_err(|e| e.to_string())?;
    runtime
        .put_state(plugin_name, &key, &payload)
        .map_err(|e| e.to_string())?;
    Ok(binding)
}

pub fn remove_binding(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    binding_id: &str,
) -> Result<(), String> {
    runtime
        .delete_state(plugin_name, &binding_key(binding_id))
        .map_err(|e| e.to_string())
}

pub fn load_binding(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    binding_id: &str,
) -> Result<ConnectorBindingRecord, String> {
    let key = binding_key(binding_id);
    let binding_raw = runtime
        .get_state(plugin_name, &key)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("unknown connector binding '{}'", key))?;
    serde_json::from_str(&binding_raw)
        .map_err(|e| format!("invalid connector binding '{}': {}", key, e))
}

pub fn endpoint_for(binding: &ConnectorBindingRecord, data_type: &str) -> Result<String, String> {
    match data_type {
        "issues" => Ok(format!(
            "https://api.github.com/repos/{}/{}/issues",
            binding.owner, binding.repo
        )),
        "prs" => Ok(format!(
            "https://api.github.com/repos/{}/{}/pulls",
            binding.owner, binding.repo
        )),
        other => Err(format!(
            "unsupported connector type '{}' (expected 'issues' or 'prs')",
            other
        )),
    }
}

pub fn ensure_type_enabled(
    binding: &ConnectorBindingRecord,
    data_type: &str,
) -> Result<(), String> {
    if binding.types.iter().any(|t| t == data_type) {
        Ok(())
    } else {
        Err(format!(
            "type '{}' not enabled for binding '{}'",
            data_type, binding.binding_id
        ))
    }
}

fn binding_key(binding_id: &str) -> String {
    format!("{}{}", CONNECTOR_BINDING_PREFIX, binding_id)
}

fn validate_binding(binding: &ConnectorBindingRecord) -> Result<(), String> {
    if binding.binding_id.trim().is_empty()
        || binding.connection.trim().is_empty()
        || binding.owner.trim().is_empty()
        || binding.repo.trim().is_empty()
    {
        return Err("binding-id, connection, owner, and repo are required".into());
    }
    if binding.types.is_empty() {
        return Err("binding requires at least one type".into());
    }
    for data_type in &binding.types {
        if data_type != "issues" && data_type != "prs" {
            return Err(format!(
                "unsupported connector type '{}' (expected 'issues' or 'prs')",
                data_type
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> KnowledgeRuntimeStore {
        let temp = tempfile::TempDir::new().unwrap();
        let db = temp.keep().join("runtime.db");
        KnowledgeRuntimeStore::new(db)
    }

    #[test]
    fn validates_binding_shape() {
        let binding = ConnectorBindingRecord {
            binding_id: "gh-main".into(),
            connection: "github".into(),
            owner: "NicabarNimble".into(),
            repo: "patina".into(),
            types: vec!["issues".into(), "prs".into()],
        };
        assert!(validate_binding(&binding).is_ok());
    }

    #[test]
    fn persists_and_loads_binding() {
        let store = temp_store();
        let plugin = "patina-ducklake";
        let binding = ConnectorBindingRecord {
            binding_id: "gh-main".into(),
            connection: "github".into(),
            owner: "NicabarNimble".into(),
            repo: "patina".into(),
            types: vec!["issues".into()],
        };
        upsert_binding(&store, plugin, binding.clone()).unwrap();
        let loaded = load_binding(&store, plugin, &binding.binding_id).unwrap();
        assert_eq!(loaded.binding_id, binding.binding_id);
        assert_eq!(loaded.repo, "patina");
    }
}
