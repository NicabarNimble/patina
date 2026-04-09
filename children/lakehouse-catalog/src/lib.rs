wit_bindgen::generate!({
    path: "wit",
    world: "lakehouse-catalog",
    generate_all,
});

use chrono::Utc;

struct LakehouseCatalog;

fn keyvalue_error_to_string(err: wasi::keyvalue::store::Error) -> String {
    match err {
        wasi::keyvalue::store::Error::NoSuchStore => "no-such-store".to_string(),
        wasi::keyvalue::store::Error::AccessDenied => "access-denied".to_string(),
        wasi::keyvalue::store::Error::Other(msg) => format!("other({msg})"),
    }
}

impl exports::patina::records::catalog::Guest for LakehouseCatalog {
    fn register(
        files: Vec<patina::records::types::FileWritten>,
    ) -> Result<Vec<patina::records::types::CatalogEntry>, String> {
        let bucket = wasi::keyvalue::store::open("patina:lakehouse-catalog")
            .map_err(keyvalue_error_to_string)?;

        let mut entries = Vec::new();
        for file in files {
            let entry = patina::records::types::CatalogEntry {
                file_path: file.file_path.clone(),
                record_count: file.record_count,
                written_at: file.written_at,
                registered_at: Utc::now().to_rfc3339(),
                schema_version: 1,
            };

            let key = format!("catalog:file:{}", entry.file_path);
            let value = format!(
                "{}|{}|{}|{}|{}",
                entry.file_path,
                entry.record_count,
                entry.written_at,
                entry.registered_at,
                entry.schema_version
            );
            bucket
                .set(&key, value.as_bytes())
                .map_err(keyvalue_error_to_string)?;

            entries.push(entry);
        }

        wasi::logging::logging::log(
            wasi::logging::logging::Level::Info,
            "lakehouse-catalog",
            &format!("registered {} files", entries.len()),
        );

        Ok(entries)
    }
}

export!(LakehouseCatalog);
