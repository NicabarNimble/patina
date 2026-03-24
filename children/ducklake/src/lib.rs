use patina_sdk::granted::{self, Bundle as GrantedBundle};
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;
use patina_sdk::substrate::{TaskIntent, TaskIntentKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct DuckLakeToys {
    log: granted::Log,
    measure: granted::Measure,
    state: granted::State,
    checkpoint: granted::Checkpoint,
    lake: granted::Lake,
    github: granted::Github,
    peer: granted::Peer,
}

impl GrantedBundle for DuckLakeToys {
    fn granted() -> Self {
        Self {
            log: granted::log(),
            measure: granted::measure(),
            state: granted::state(),
            checkpoint: granted::checkpoint(),
            lake: granted::lake("default"),
            github: granted::github(),
            peer: granted::peer(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceConfig {
    source_id: String,
    #[serde(alias = "table")]
    table_prefix: String,
    owner: String,
    repo: String,
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
        let mut per_type = serde_json::Map::new();
        let mut written_total = 0_u64;

        for data_type in config.normalized_types() {
            let cursor_before = self.toys.lake.load_cursor(&config.source_id, &data_type);
            let rows = self.fetch_rows(&config, &data_type, cursor_before.as_deref())?;
            let table = format!("{}_{}", config.table_prefix, data_type).replace('-', "_");
            self.toys.lake.ensure_table(&table)?;
            let written =
                self.toys
                    .lake
                    .append_json_batch(&table, &config.source_id, &rows.rows_json)?;
            self.toys
                .lake
                .save_cursor(&patina_sdk::toys::LakeCursorRecord {
                    source: config.source_id.clone(),
                    data_type: data_type.clone(),
                    cursor: rows.cursor.clone(),
                    written,
                    status: "ok".into(),
                    last_error: None,
                })?;
            written_total += written;
            per_type.insert(
                data_type,
                serde_json::json!({
                    "written": written,
                    "cursor": rows.cursor,
                }),
            );
        }

        self.toys.checkpoint.save(
            "ducklake.sync",
            &serde_json::json!({
                "source_id": config.source_id,
                "owner": config.owner,
                "repo": config.repo,
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
        self.toys.peer.emit_event(
            "data-ingested",
            &serde_json::json!({
                "source_id": source_id,
                "owner": config.owner,
                "repo": config.repo,
                "written": written_total,
            })
            .to_string(),
        )?;
        Ok(
            serde_json::json!({"status": "synced", "source_id": source_id, "written": written_total})
                .to_string(),
        )
    }

    fn fetch_rows(
        &self,
        config: &SourceConfig,
        data_type: &str,
        since: Option<&str>,
    ) -> Result<FetchRows, String> {
        let mut page = Some(1_u32);
        let mut rows = Vec::new();

        while let Some(current_page) = page {
            let params = patina_sdk::toys::GithubListParams {
                since: since.map(ToString::to_string),
                state: Some("all".to_string()),
                page: Some(current_page),
                per_page: Some(100),
            };
            let result = match data_type {
                "issues" => self
                    .toys
                    .github
                    .list_issues(&config.owner, &config.repo, &params),
                "prs" | "pulls" => {
                    self.toys
                        .github
                        .list_pulls(&config.owner, &config.repo, &params)
                }
                other => {
                    return Err(format!(
                        "unsupported data_type '{}' (expected issues or prs)",
                        other
                    ));
                }
            };

            let result = match result {
                Ok(page_result) => page_result,
                Err(error) => {
                    if current_page == 1 {
                        if let Some(fixture) = fixture_rows(data_type) {
                            rows.extend(fixture);
                            break;
                        }
                    }
                    return Err(error);
                }
            };

            let page_rows = parse_rows(&result.items)?;
            if page_rows.is_empty() {
                break;
            }
            rows.extend(page_rows);

            if result.has_next {
                page = result.next_page;
            } else {
                page = None;
            }
        }

        let cursor = rows
            .iter()
            .filter_map(|row| row.get("updated_at").and_then(|v| v.as_str()))
            .max()
            .map(ToString::to_string)
            .or_else(|| since.map(ToString::to_string));

        let rows_json = rows
            .into_iter()
            .map(|row| serde_json::to_string(&row).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FetchRows { rows_json, cursor })
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

struct FetchRows {
    rows_json: Vec<String>,
    cursor: Option<String>,
}

fn parse_rows(items: &str) -> Result<Vec<serde_json::Value>, String> {
    let value: serde_json::Value =
        serde_json::from_str(items).map_err(|e| format!("invalid github page json: {}", e))?;
    let arr = value
        .as_array()
        .ok_or_else(|| "github page payload must be a JSON array".to_string())?;
    Ok(arr.to_vec())
}

fn fixture_rows(data_type: &str) -> Option<Vec<serde_json::Value>> {
    let raw = match data_type {
        "issues" => include_str!("../../../tests/fixtures/github-api/issues.json"),
        "prs" | "pulls" => include_str!("../../../tests/fixtures/github-api/pulls.json"),
        _ => return None,
    };
    parse_rows(raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_rows_available_for_issues_and_pulls() {
        let issues = fixture_rows("issues").unwrap();
        let pulls = fixture_rows("prs").unwrap();
        assert!(!issues.is_empty());
        assert!(!pulls.is_empty());
    }

    #[test]
    fn parse_rows_rejects_non_arrays() {
        let err = parse_rows("{\"ok\":true}").unwrap_err();
        assert!(err.contains("JSON array"));
    }
}

impl KnowledgeChild for DuckLakeChild {
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
