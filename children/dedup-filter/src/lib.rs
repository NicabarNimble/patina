wit_bindgen::generate!({
    path: "wit",
    world: "dedup-filter",
    generate_all,
});

struct DedupFilter;

use patina_sdk::toys;

impl exports::patina::records::transform::Guest for DedupFilter {
    fn transform(
        records: Vec<patina::records::types::RecordEnvelope>,
    ) -> Result<patina::records::types::TransformResult, String> {
        let bucket = toys::keyvalue::open("patina:dedup-filter")?;

        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        let mut duplicate_output_violations = 0_u64;
        let mut seen_output_hashes = std::collections::HashSet::new();

        for record in records {
            let dedup_key = format!("dedup:{}", record.content_hash);
            let exists = bucket.exists(&dedup_key)?;

            if exists {
                toys::log::info(
                    "dedup-filter",
                    &format!(
                        "dedup-filter duplicate {} (duplicate-content-hash)",
                        record.source_path
                    ),
                );
                rejected.push(patina::records::types::RejectedRecord {
                    reason: "duplicate-content-hash".to_string(),
                    envelope: record,
                });
                continue;
            }

            bucket.set(&dedup_key, b"seen")?;

            if !seen_output_hashes.insert(record.content_hash.clone()) {
                duplicate_output_violations += 1;
            }

            accepted.push(record);
        }

        toys::measure::counter("records_seen", (accepted.len() + rejected.len()) as f64)?;
        toys::measure::counter("ready_records", accepted.len() as f64)?;
        toys::measure::counter("duplicate_records", rejected.len() as f64)?;

        let duplicate_output_rate_pct = if accepted.is_empty() {
            0.0
        } else {
            (duplicate_output_violations as f64 / accepted.len() as f64) * 100.0
        };
        toys::measure::gauge("duplicate_output_rate_pct", duplicate_output_rate_pct)?;

        Ok(patina::records::types::TransformResult { accepted, rejected })
    }
}

export!(DedupFilter);
