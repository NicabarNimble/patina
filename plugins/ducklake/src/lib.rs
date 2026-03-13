use patina_child_sdk::granted::{self, Bundle as GrantedBundle};
use patina_child_sdk::substrate::{TaskIntent, TaskIntentKind};
use patina_child_sdk::{register_knowledge_child, ChildHealth, HealthStatus, KnowledgeChildPlugin};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct DuckLakeToys {
    log: granted::Log,
    measure: granted::Measure,
    state: granted::State,
    checkpoint: granted::Checkpoint,
    lake: granted::Lake,
    connectors: granted::Connectors,
}

impl GrantedBundle for DuckLakeToys {
    fn granted() -> Self {
        Self {
            log: granted::log(),
            measure: granted::measure(),
            state: granted::state(),
            checkpoint: granted::checkpoint(),
            lake: granted::lake("default"),
            connectors: granted::connectors(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceConfig {
    source_id: String,
    #[serde(alias = "table")]
    table_prefix: String,
    binding_id: String,
    #[serde(default)]
    data_types: Vec<String>,
}

impl SourceConfig {
    fn normalized_types(&self) -> Vec<String> {
        if self.data_types.is_empty() {
            vec!["issues".into(), "prs".into()]
        } else {
            self.data_types.clone()
        }
    }
}

struct DuckLakeChild {
    toys: DuckLakeToys,
}

impl Default for DuckLakeChild {
    fn default() -> Self {
        Self {
            toys: DuckLakeToys::granted(),
        }
    }
}

impl DuckLakeChild {
    fn source_key(source_id: &str) -> String {
        format!("source:{}", source_id)
    }

    fn load_source(&self, source_id: &str) -> Result<SourceConfig, String> {
        let key = Self::source_key(source_id);
        let json = self
            .toys
            .state
            .get(&key)
            .ok_or_else(|| format!("unknown source '{}'", source_id))?;
        serde_json::from_str(&json).map_err(|e| format!("invalid source config: {}", e))
    }

    fn configure_source(&mut self, payload: &str) -> Result<String, String> {
        let config: SourceConfig =
            serde_json::from_str(payload).map_err(|e| format!("invalid source config: {}", e))?;
        let key = Self::source_key(&config.source_id);
        self.toys.state.put(
            &key,
            &serde_json::to_string(&config).map_err(|e| e.to_string())?,
        )?;
        Ok(serde_json::json!({"status": "configured", "source_id": config.source_id}).to_string())
    }

    fn sync_source(&mut self, payload: &str) -> Result<String, String> {
        let value: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| format!("invalid sync payload: {}", e))?;
        let source_id = value
            .get("source_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'source_id'".to_string())?;
        let config = self.load_source(source_id)?;
        let binding = self.toys.connectors.require(&config.binding_id)?;
        let mut per_type = serde_json::Map::new();
        let mut written_total = 0_u64;

        for data_type in config.normalized_types() {
            let cursor_before = self.toys.lake.load_cursor(&config.source_id, &data_type);
            let sync = binding.sync(&data_type, cursor_before.as_deref())?;
            let table = format!("{}_{}", config.table_prefix, data_type).replace('-', "_");
            self.toys.lake.ensure_table(&table)?;
            let written =
                self.toys
                    .lake
                    .append_json_batch(&table, &config.source_id, &sync.rows_json)?;
            self.toys
                .lake
                .save_cursor(&patina_child_sdk::toys::LakeCursorRecord {
                    source: config.source_id.clone(),
                    data_type: data_type.clone(),
                    cursor: sync.cursor.clone(),
                    written,
                    status: "ok".into(),
                    last_error: None,
                })?;
            written_total += written;
            per_type.insert(
                data_type,
                serde_json::json!({
                    "written": written,
                    "cursor": sync.cursor,
                }),
            );
        }

        self.toys.checkpoint.save(
            "ducklake.sync",
            &serde_json::json!({
                "source_id": config.source_id,
                "binding_id": config.binding_id,
                "types": per_type,
                "written": written_total,
            })
            .to_string(),
        )?;
        self.toys.measure.record(
            "capture",
            "lake",
            "sync",
            &serde_json::json!({"written": written_total}).to_string(),
        )?;
        Ok(
            serde_json::json!({"status": "synced", "source_id": source_id, "written": written_total})
                .to_string(),
        )
    }

    fn status(&self) -> Result<String, String> {
        let sources = self.toys.state.list_prefix("source:");
        let checkpoint = self.toys.checkpoint.load("ducklake.sync");
        Ok(serde_json::json!({
            "sources": sources,
            "checkpoint": checkpoint,
        })
        .to_string())
    }
}

impl KnowledgeChildPlugin for DuckLakeChild {
    fn name(&self) -> String {
        "ducklake".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        self.toys.log.info("ducklake knowledge child loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: None,
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "configure-source" => self.configure_source(payload),
            "fetch-source" => self.sync_source(payload),
            "status" => self.status(),
            other => Err(format!("ducklake: unknown action '{}'", other)),
        }
    }

    fn tick(&mut self) -> Vec<TaskIntent> {
        self.toys
            .state
            .list_prefix("source:")
            .into_iter()
            .filter_map(|key| {
                let source_id = key.strip_prefix("source:")?.to_string();
                Some(TaskIntent {
                    kind: TaskIntentKind::FetchSource,
                    payload_json: serde_json::json!({"source_id": source_id}).to_string(),
                    dedupe_key: Some(format!("ducklake:{}", key)),
                })
            })
            .collect()
    }
}

register_knowledge_child!(DuckLakeChild);
