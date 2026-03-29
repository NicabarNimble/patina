use chrono::Utc;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

#[derive(Default)]
struct RecordWriterChild;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    record_id: String,
    source_path: String,
    source_hash: String,
    source_modified_at: String,
    source_size_bytes: u64,
    content: String,
    content_hash: String,
    content_type: String,
    encoding: String,
    line_count: u64,
    ingested_at: String,
    batch_id: String,
    schema_version: u32,
}

#[derive(Debug, Clone, Serialize)]
struct FileWrittenEvent {
    file_path: String,
    record_count: u64,
    written_at: String,
}

fn parse_payload(payload: &str) -> Result<Value, String> {
    if payload.trim().is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(payload).map_err(|e| format!("invalid payload json: {}", e))
    }
}

fn resolve_output_path(payload: &Value) -> Result<PathBuf, String> {
    let Some(output_path) = payload.get("output_path").and_then(|v| v.as_str()) else {
        return Err("missing output_path in action payload".to_string());
    };
    if output_path.trim().is_empty() {
        return Err("output_path cannot be empty".to_string());
    }
    let output = PathBuf::from(output_path);
    if !output.starts_with("/output") {
        return Err("output_path must stay under /output preopen".to_string());
    }
    if output
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("output_path cannot contain '..' segments".to_string());
    }
    Ok(output)
}

fn emit_metric_counter(
    name: &str,
    delta: f64,
    labels: Vec<(String, String)>,
) -> Result<(), String> {
    patina_sdk::knowledge_child::patina::measure::measure::emit(
        &patina_sdk::knowledge_child::patina::measure::measure::Metric {
            name: name.to_string(),
            value: delta,
            labels,
        },
    )
}

fn emit_metric_gauge(name: &str, value: f64) -> Result<(), String> {
    patina_sdk::knowledge_child::patina::measure::measure::gauge(name, value)
}

fn emit_file_written(event: &FileWrittenEvent) -> Result<u64, String> {
    let payload = serde_json::to_string(event).map_err(|e| e.to_string())?;
    let client = patina_sdk::knowledge_child::wasi::messaging::producer::connect("file.written")?;
    let message = patina_sdk::knowledge_child::wasi::messaging::types::Message {
        topic: "file.written".to_string(),
        content_type: Some("application/json".to_string()),
        data: payload.into_bytes(),
        metadata: vec![],
    };
    patina_sdk::knowledge_child::wasi::messaging::producer::send(&client, &message)
}

fn write_records_parquet(records: &[Record], output_path: &Path) -> Result<(), String> {
    use arrow_array::{Int32Array, Int64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};

    let schema = Arc::new(Schema::new(vec![
        Field::new("record_id", DataType::Utf8, false),
        Field::new("source_path", DataType::Utf8, false),
        Field::new("source_hash", DataType::Utf8, false),
        Field::new("source_modified_at", DataType::Utf8, false),
        Field::new("source_size_bytes", DataType::Int64, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("content_hash", DataType::Utf8, false),
        Field::new("content_type", DataType::Utf8, false),
        Field::new("encoding", DataType::Utf8, false),
        Field::new("line_count", DataType::Int64, false),
        Field::new("ingested_at", DataType::Utf8, false),
        Field::new("batch_id", DataType::Utf8, false),
        Field::new("schema_version", DataType::Int32, false),
    ]));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.record_id.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.source_path.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.source_hash.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.source_modified_at.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                records
                    .iter()
                    .map(|record| record.source_size_bytes as i64)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.content.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.content_hash.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.content_type.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.encoding.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                records
                    .iter()
                    .map(|record| record.line_count as i64)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.ingested_at.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                records
                    .iter()
                    .map(|record| record.batch_id.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                records
                    .iter()
                    .map(|record| record.schema_version as i32)
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|e| format!("failed to build parquet record batch: {}", e))?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create output directory '{}': {}",
                parent.display(),
                e
            )
        })?;
    }

    let file = fs::File::create(output_path).map_err(|e| {
        format!(
            "failed to create parquet file '{}': {}",
            output_path.display(),
            e
        )
    })?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))
        .map_err(|e| format!("failed to initialize parquet writer: {}", e))?;
    writer
        .write(&batch)
        .map_err(|e| format!("failed to write parquet batch: {}", e))?;
    writer
        .close()
        .map_err(|e| format!("failed to close parquet writer: {}", e))?;
    Ok(())
}

impl RecordWriterChild {
    fn write_records(&mut self, payload: &str) -> Result<String, String> {
        let payload_value = parse_payload(payload)?;
        let output_path = resolve_output_path(&payload_value)?;
        let limit = payload_value
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(128)
            .min(u32::MAX as u64) as u32;
        let after_offset = payload_value
            .get("after_offset")
            .and_then(|value| value.as_u64());

        let events = patina_sdk::knowledge_child::patina::events_stream::events_stream::subscribe(
            "record.ready",
            after_offset,
            limit,
        )?;

        let state = granted::state();
        let mut records = Vec::new();
        let mut last_offset = None;

        for event in events {
            last_offset = Some(event.offset);
            let record: Record = serde_json::from_str(&event.payload)
                .map_err(|e| format!("invalid record.ready payload: {}", e))?;

            let key = format!("record:{}", record.source_hash);
            let state_json = serde_json::to_string(&record).map_err(|e| e.to_string())?;
            let write_start = Instant::now();
            state.put(&key, &state_json)?;
            let write_latency_ms = write_start.elapsed().as_secs_f64() * 1000.0;

            emit_metric_counter("records_written", 1.0, vec![])?;
            emit_metric_gauge("write_latency_ms", write_latency_ms)?;
            records.push(record);
        }

        if let Some(offset) = last_offset {
            patina_sdk::knowledge_child::patina::events_stream::events_stream::ack(
                "record.ready",
                offset,
            )?;
        }

        write_records_parquet(&records, &output_path)?;
        emit_metric_gauge("batch_size", records.len() as f64)?;
        emit_file_written(&FileWrittenEvent {
            file_path: output_path.to_string_lossy().to_string(),
            record_count: records.len() as u64,
            written_at: Utc::now().to_rfc3339(),
        })?;

        Ok(serde_json::json!({
            "status": "ok",
            "parquet_path": output_path.to_string_lossy(),
            "processed_records": records.len(),
            "state_keys": state.list_prefix("record:"),
            "acked_through": last_offset,
        })
        .to_string())
    }
}

impl KnowledgeChild for RecordWriterChild {
    fn name(&self) -> String {
        "record-writer".to_string()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("record-writer loaded");
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
            "write-records" => self.write_records(payload),
            other => Err(format!("record-writer: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(RecordWriterChild);
