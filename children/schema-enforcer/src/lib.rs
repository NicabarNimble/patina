wit_bindgen::generate!({
    path: "wit",
    world: "schema-enforcer",
    generate_all,
});

struct SchemaEnforcer;

fn validate_record(record: &patina::records::types::RecordEnvelope) -> Result<(), String> {
    if record.record_id.trim().is_empty() {
        return Err("missing record_id".to_string());
    }
    if record.source_path.trim().is_empty() {
        return Err("missing source_path".to_string());
    }
    if record.source_hash.trim().is_empty() {
        return Err("missing source_hash".to_string());
    }
    if record.source_modified_at.trim().is_empty() {
        return Err("missing source_modified_at".to_string());
    }
    if record.content.trim().is_empty() {
        return Err("missing content".to_string());
    }
    if record.content_hash.trim().is_empty() {
        return Err("missing content_hash".to_string());
    }
    if record.content_type.trim().is_empty() {
        return Err("missing content_type".to_string());
    }
    if record.encoding.trim().is_empty() {
        return Err("missing encoding".to_string());
    }
    if record.ingested_at.trim().is_empty() {
        return Err("missing ingested_at".to_string());
    }
    if record.batch_id.trim().is_empty() {
        return Err("missing batch_id".to_string());
    }
    if record.schema_version == 0 {
        return Err("schema_version must be >= 1".to_string());
    }
    Ok(())
}

impl exports::patina::records::transform::Guest for SchemaEnforcer {
    fn transform(
        records: Vec<patina::records::types::RecordEnvelope>,
    ) -> Result<patina::records::types::TransformResult, String> {
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for record in records {
            match validate_record(&record) {
                Ok(()) => accepted.push(record),
                Err(reason) => {
                    wasi::logging::logging::log(
                        wasi::logging::logging::Level::Warn,
                        "schema-enforcer",
                        &format!(
                            "schema-enforcer rejected {} ({})",
                            record.source_path, reason
                        ),
                    );
                    rejected.push(patina::records::types::RejectedRecord {
                        reason,
                        envelope: record,
                    });
                }
            }
        }

        patina::measure::measure::counter("validated_records", accepted.len() as f64)?;
        patina::measure::measure::counter("rejected_records", rejected.len() as f64)?;

        let total = accepted.len() + rejected.len();
        let provenance_complete = accepted.iter().filter(|r| r.source_size_bytes > 0).count();
        let provenance_pct = if total == 0 {
            100.0
        } else {
            (provenance_complete as f64 / total as f64) * 100.0
        };
        patina::measure::measure::gauge("provenance_completeness_pct", provenance_pct)?;

        Ok(patina::records::types::TransformResult { accepted, rejected })
    }
}

export!(SchemaEnforcer);
