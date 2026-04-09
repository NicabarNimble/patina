wit_bindgen::generate!({
    path: "wit",
    world: "dedup-filter",
    generate_all,
});

struct DedupFilter;

fn keyvalue_error_to_string(err: wasi::keyvalue::store::Error) -> String {
    match err {
        wasi::keyvalue::store::Error::NoSuchStore => "no-such-store".to_string(),
        wasi::keyvalue::store::Error::AccessDenied => "access-denied".to_string(),
        wasi::keyvalue::store::Error::Other(msg) => format!("other({msg})"),
    }
}

impl exports::patina::records::transform::Guest for DedupFilter {
    fn transform(
        records: Vec<patina::records::types::RecordEnvelope>,
    ) -> Result<patina::records::types::TransformResult, String> {
        let upstream = patina::records::transform::transform(&records)?;
        let bucket =
            wasi::keyvalue::store::open("patina:dedup-filter").map_err(keyvalue_error_to_string)?;

        let mut accepted = Vec::new();
        let mut rejected = upstream.rejected;
        let mut duplicate_output_violations = 0_u64;
        let mut seen_output_hashes = std::collections::HashSet::new();

        for record in upstream.accepted {
            let dedup_key = format!("dedup:{}", record.content_hash);
            let exists = bucket
                .exists(&dedup_key)
                .map_err(keyvalue_error_to_string)?;

            if exists {
                wasi::logging::logging::log(
                    wasi::logging::logging::Level::Info,
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

            bucket
                .set(&dedup_key, b"seen")
                .map_err(keyvalue_error_to_string)?;

            if !seen_output_hashes.insert(record.content_hash.clone()) {
                duplicate_output_violations += 1;
            }

            accepted.push(record);
        }

        patina::measure::measure::counter(
            "records_seen",
            (accepted.len() + rejected.len()) as f64,
        )?;
        patina::measure::measure::counter("ready_records", accepted.len() as f64)?;
        patina::measure::measure::counter("duplicate_records", rejected.len() as f64)?;

        let duplicate_output_rate_pct = if accepted.is_empty() {
            0.0
        } else {
            (duplicate_output_violations as f64 / accepted.len() as f64) * 100.0
        };
        patina::measure::measure::gauge("duplicate_output_rate_pct", duplicate_output_rate_pct)?;

        Ok(patina::records::types::TransformResult { accepted, rejected })
    }
}

export!(DedupFilter);
