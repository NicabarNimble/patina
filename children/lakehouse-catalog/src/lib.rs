wit_bindgen::generate!({
    path: "wit",
    world: "lakehouse-catalog",
    generate_all,
});

use chrono::Utc;
use patina_sdk::toys;

struct LakehouseCatalog;

impl exports::patina::records::catalog::Guest for LakehouseCatalog {
    fn register(
        files: Vec<patina::records::types::FileWritten>,
    ) -> Result<Vec<patina::records::types::CatalogEntry>, String> {
        let bucket = toys::keyvalue::open("patina:lakehouse-catalog")?;

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
            bucket.set(&key, value.as_bytes())?;

            entries.push(entry);
        }

        toys::log::info(
            "lakehouse-catalog",
            &format!("registered {} files", entries.len()),
        );

        Ok(entries)
    }
}

export!(LakehouseCatalog);
