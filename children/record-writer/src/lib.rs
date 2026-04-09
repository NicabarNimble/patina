wit_bindgen::generate!({
    path: "wit",
    world: "record-writer",
    generate_all,
});

use chrono::Utc;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use patina_sdk::toys;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

struct RecordWriter;

fn write_records_parquet(
    records: &[patina::records::types::RecordEnvelope],
    output_path: &Path,
) -> Result<(), String> {
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
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create output directory '{}': {}",
                parent.display(),
                e
            )
        })?;
    }

    let file = std::fs::File::create(output_path).map_err(|e| {
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

fn resolve_output_path() -> Result<PathBuf, String> {
    let output_root = ["/output", "/lake", "/input"]
        .iter()
        .map(Path::new)
        .find(|path| path.exists() && path.is_dir())
        .ok_or_else(|| "no filesystem preopen directory available".to_string())?;

    Ok(output_root.join(format!(
        "records-{}.parquet",
        Utc::now().format("%Y%m%d-%H%M%S")
    )))
}

impl exports::patina::records::write::Guest for RecordWriter {
    fn write(
        records: Vec<patina::records::types::RecordEnvelope>,
    ) -> Result<Vec<patina::records::types::FileWritten>, String> {
        let accepted = records;

        let output_path = resolve_output_path()?;

        let bucket = toys::keyvalue::open("patina:record-writer")?;
        let write_start = Instant::now();
        for record in &accepted {
            let key = format!("record:{}", record.source_hash);
            bucket.set(&key, record.content.as_bytes())?;
        }
        let write_latency_ms = write_start.elapsed().as_secs_f64() * 1000.0;

        write_records_parquet(&accepted, &output_path)?;
        toys::measure::counter("records_written", accepted.len() as f64)?;
        toys::measure::gauge("batch_size", accepted.len() as f64)?;
        toys::measure::gauge("write_latency_ms", write_latency_ms)?;

        toys::log::info(
            "record-writer",
            &format!(
                "wrote {} records to {}",
                accepted.len(),
                output_path.display()
            ),
        );

        Ok(vec![patina::records::types::FileWritten {
            file_path: output_path.to_string_lossy().to_string(),
            record_count: accepted.len() as u64,
            written_at: Utc::now().to_rfc3339(),
        }])
    }
}

export!(RecordWriter);
