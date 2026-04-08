use chrono::Utc;
use patina_sdk::child::{Child, ChildHealth, HealthStatus};
use patina_sdk::granted;
use patina_sdk::register_child;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default)]
struct LakehouseCatalogChild;

#[derive(Debug, Clone, Deserialize)]
struct FileWrittenEvent {
    file_path: String,
    record_count: u64,
    written_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct CatalogFileEntry {
    file_path: String,
    record_count: u64,
    written_at: String,
    registered_at: String,
    schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CatalogColumn {
    name: String,
    nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CatalogSchema {
    version: u32,
    columns: Vec<CatalogColumn>,
    updated_at: String,
}

const BASE_COLUMNS: &[&str] = &[
    "record_id",
    "source_path",
    "source_hash",
    "source_modified_at",
    "source_size_bytes",
    "content",
    "content_hash",
    "content_type",
    "encoding",
    "line_count",
    "ingested_at",
    "batch_id",
    "schema_version",
];

fn parse_payload(payload: &str) -> Result<Value, String> {
    if payload.trim().is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(payload).map_err(|e| format!("invalid payload json: {}", e))
    }
}

fn default_schema() -> CatalogSchema {
    CatalogSchema {
        version: 1,
        columns: BASE_COLUMNS
            .iter()
            .map(|name| CatalogColumn {
                name: (*name).to_string(),
                nullable: false,
            })
            .collect(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

fn load_schema(state: &patina_sdk::granted::State) -> CatalogSchema {
    let Some(raw) = state.get("catalog:schema:current") else {
        return default_schema();
    };
    serde_json::from_str::<CatalogSchema>(&raw).unwrap_or_else(|_| default_schema())
}

fn persist_schema(
    state: &patina_sdk::granted::State,
    schema: &CatalogSchema,
) -> Result<(), String> {
    state.put(
        "catalog:schema:current",
        &serde_json::to_string(schema).map_err(|e| e.to_string())?,
    )?;
    state.put(
        &format!("catalog:schema:history:{}", schema.version),
        &serde_json::to_string(schema).map_err(|e| e.to_string())?,
    )?;
    Ok(())
}

fn evolve_schema(
    schema: &CatalogSchema,
    added_columns: &[String],
) -> Result<(CatalogSchema, Vec<String>), String> {
    let mut next = schema.clone();
    let mut applied = Vec::new();

    for column in added_columns {
        let name = column.trim();
        if name.is_empty() {
            return Err("schema evolution column cannot be empty".to_string());
        }
        if name.contains('.') || name.contains(':') || name.contains(' ') {
            return Err(format!(
                "schema evolution column '{}' contains unsupported characters",
                name
            ));
        }
        if next.columns.iter().any(|existing| existing.name == name) {
            continue;
        }

        next.columns.push(CatalogColumn {
            name: name.to_string(),
            nullable: true,
        });
        next.version += 1;
        next.updated_at = Utc::now().to_rfc3339();
        applied.push(name.to_string());
    }

    Ok((next, applied))
}

fn append_catalog_entries(entries: &[CatalogFileEntry]) -> Result<u32, String> {
    if entries.is_empty() {
        return Ok(0);
    }
    let rows_json = entries
        .iter()
        .map(|entry| serde_json::to_string(entry).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = serde_json::json!({
        "table": "catalog_files",
        "source": "lakehouse-catalog",
        "rows_json": rows_json,
    });
    let sql = granted::sql();
    let conn = sql.open("default")?;
    let stmt = sql.prepare(
        "__patina_mutate__",
        &["append-json-batch".to_string(), payload.to_string()],
    )?;
    sql.exec(&conn, &stmt)
}

fn read_catalog_row_count() -> Result<u64, String> {
    let sql = granted::sql();
    let conn = sql.open("default")?;
    let stmt = sql.prepare("SELECT COUNT(*) FROM catalog_files", &[])?;
    let rows = sql.query(&conn, &stmt)?;
    let Some(value) = rows.first().and_then(|row| row.values.first()) else {
        return Ok(0);
    };
    match value {
        patina_sdk::toys::SqlValue::Int32(v) => Ok((*v).max(0) as u64),
        patina_sdk::toys::SqlValue::Int64(v) => Ok((*v).max(0) as u64),
        patina_sdk::toys::SqlValue::Text(v) => v.parse::<u64>().map_err(|e| e.to_string()),
        _ => Err("catalog count query returned unsupported SQL type".to_string()),
    }
}

fn catalog_entry_key(file_path: &str) -> String {
    format!("catalog:file:{}", file_path)
}

impl LakehouseCatalogChild {
    fn register_written(&mut self, payload: &str) -> Result<String, String> {
        let payload_value = parse_payload(payload)?;
        let limit = payload_value
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(128)
            .min(u32::MAX as u64) as u32;
        let after_offset = payload_value
            .get("after_offset")
            .and_then(|value| value.as_u64());
        let schema_additions = payload_value
            .get("add_nullable_columns")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let events = patina_sdk::child::patina::events_stream::events_stream::subscribe(
            "file.written",
            after_offset,
            limit,
        )?;

        let state = granted::state();
        let current_schema = load_schema(&state);
        let (next_schema, applied_schema_changes) =
            evolve_schema(&current_schema, &schema_additions)?;
        persist_schema(&state, &next_schema)?;

        let mut registered = 0_u64;
        let mut entries = Vec::new();
        let mut last_offset = None;

        for event in events {
            last_offset = Some(event.offset);
            let file_written: FileWrittenEvent = serde_json::from_str(&event.payload)
                .map_err(|e| format!("invalid file.written payload: {}", e))?;

            let entry = CatalogFileEntry {
                file_path: file_written.file_path,
                record_count: file_written.record_count,
                written_at: file_written.written_at,
                registered_at: Utc::now().to_rfc3339(),
                schema_version: next_schema.version,
            };
            entries.push(entry);
            registered += 1;
        }

        for entry in &entries {
            state.put(
                &catalog_entry_key(&entry.file_path),
                &serde_json::to_string(entry).map_err(|e| e.to_string())?,
            )?;
        }

        let catalog_keys = state.list_prefix("catalog:file:");

        let sql_inserted = append_catalog_entries(&entries)?;
        let catalog_rows = read_catalog_row_count()?;

        if let Some(offset) = last_offset {
            patina_sdk::child::patina::events_stream::events_stream::ack("file.written", offset)?;
        }

        Ok(serde_json::json!({
            "status": "ok",
            "registered_files": registered,
            "sql_inserted_rows": sql_inserted,
            "entries": entries,
            "catalog_keys": catalog_keys,
            "catalog_rows": catalog_rows,
            "schema_version": next_schema.version,
            "schema": next_schema,
            "schema_migrations_applied": applied_schema_changes,
            "acked_through": last_offset,
        })
        .to_string())
    }
}

impl Child for LakehouseCatalogChild {
    fn name(&self) -> String {
        "lakehouse-catalog".to_string()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("lakehouse-catalog loaded");
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
            "register-written" => self.register_written(payload),
            other => Err(format!("lakehouse-catalog: unknown action '{}'", other)),
        }
    }
}

register_child!(LakehouseCatalogChild);
