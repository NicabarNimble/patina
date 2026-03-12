use patina_child_sdk::host::GuestHost;
use patina_child_sdk::toys::{
    CheckpointToy, FetchToy, LakeToy, LogToy, MeasureToy, StateToy, TaskIntent, TaskIntentKind,
};
use patina_child_sdk::{register_knowledge_child, ChildHealth, HealthStatus, KnowledgeChildPlugin};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
struct DuckLakeToys {
    log: LogToy<GuestHost>,
    measure: MeasureToy<GuestHost>,
    state: StateToy<GuestHost>,
    checkpoint: CheckpointToy<GuestHost>,
    fetch: FetchToy<GuestHost>,
    lake: LakeToy<GuestHost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceConfig {
    source_id: String,
    lake: String,
    table: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default)]
    data_url: Option<String>,
    #[serde(default)]
    rows: Vec<serde_json::Value>,
}

fn default_mode() -> String {
    "inline".into()
}

#[derive(Default)]
struct DuckLakeChild {
    toys: DuckLakeToys,
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
        let rows = match config.mode.as_str() {
            "inline" => config
                .rows
                .iter()
                .map(|row| serde_json::to_string(row).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?,
            "http" => {
                let url = config
                    .data_url
                    .as_deref()
                    .ok_or_else(|| "http mode requires data_url".to_string())?;
                let body = self.toys.fetch.get(url)?;
                let parsed: serde_json::Value = serde_json::from_str(&body)
                    .map_err(|e| format!("invalid fetch response: {}", e))?;
                let rows = parsed
                    .get("rows")
                    .and_then(|v| v.as_array())
                    .or_else(|| parsed.as_array())
                    .ok_or_else(|| {
                        "fetched payload must be an array or {rows:[...]}".to_string()
                    })?;
                rows.iter()
                    .map(|row| serde_json::to_string(row).map_err(|e| e.to_string()))
                    .collect::<Result<Vec<_>, _>>()?
            }
            other => return Err(format!("unsupported source mode '{}'", other)),
        };

        self.toys.lake.ensure_lake(&config.lake)?;
        self.toys.lake.ensure_table(&config.lake, &config.table)?;
        let written = self.toys.lake.append_json_batch(
            &config.lake,
            &config.table,
            &config.source_id,
            &rows,
        )?;
        let cursor = Some(format!("{}:{}", config.source_id, written));
        self.toys.lake.save_cursor(
            &config.lake,
            &config.source_id,
            &config.table,
            cursor.as_deref(),
            written,
            "ok",
            None,
        )?;
        self.toys.checkpoint.save(
            "ducklake.sync",
            &serde_json::json!({
                "source_id": config.source_id,
                "cursor": cursor,
                "written": written
            })
            .to_string(),
        )?;
        self.toys.measure.record(
            "capture",
            "ducklake",
            "sync",
            &serde_json::json!({"written": written}).to_string(),
        )?;
        Ok(
            serde_json::json!({"status": "synced", "source_id": source_id, "written": written})
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
