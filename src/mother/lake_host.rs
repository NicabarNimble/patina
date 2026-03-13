use anyhow::{Context, Result};
use duckdb::Connection;
use std::path::{Path, PathBuf};

use super::KnowledgeRuntimeStore;

fn sanitize_table_name(table: &str) -> Result<String> {
    if table.is_empty()
        || !table
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        anyhow::bail!("invalid table name '{}'", table);
    }
    Ok(table.replace('-', "_"))
}

fn ensure_lake_dir(name: &str) -> Result<PathBuf> {
    let path = crate::paths::lakes::lakes_dir().join(name);
    if !path.exists() {
        std::fs::create_dir_all(&path)
            .with_context(|| format!("creating lake {}", path.display()))?;
    }
    let lake_toml = path.join("lake.toml");
    if !lake_toml.exists() {
        let created_at = chrono::Utc::now().to_rfc3339();
        std::fs::write(
            &lake_toml,
            format!("name = \"{}\"\ncreated_at = \"{}\"\n", name, created_at),
        )
        .with_context(|| format!("writing {}", lake_toml.display()))?;
    }
    Ok(path)
}

fn open_lake_db(path: &Path) -> Result<Connection> {
    let db_path = path.join("lake.duckdb");
    let conn = Connection::open(&db_path)
        .with_context(|| format!("opening lake db {}", db_path.display()))?;
    Ok(conn)
}

pub fn ensure_lake(name: &str) -> Result<String> {
    let path = match crate::paths::lakes::resolve_lake_path(name) {
        Ok(path) => path,
        Err(_) => ensure_lake_dir(name)?,
    };
    Ok(path.to_string_lossy().to_string())
}

pub fn load_cursor(
    store: &KnowledgeRuntimeStore,
    lake: &str,
    source: &str,
    data_type: &str,
) -> Result<Option<String>> {
    store.load_lake_cursor(lake, source, data_type)
}

pub fn save_cursor(
    store: &KnowledgeRuntimeStore,
    update: &crate::mother::state::LakeCursorUpdate<'_>,
) -> Result<()> {
    store.save_lake_cursor(update)
}

pub fn ensure_table(lake: &str, table: &str) -> Result<()> {
    let lake_path = ensure_lake(lake)?;
    let conn = open_lake_db(Path::new(&lake_path))?;
    let table = sanitize_table_name(table)?;
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {} (
            _ingested_at TIMESTAMP DEFAULT current_timestamp,
            _source_id VARCHAR,
            data JSON
        )",
        table
    ))?;
    Ok(())
}

pub fn append_json_batch(
    lake: &str,
    table: &str,
    source: &str,
    rows_json: &[String],
) -> Result<u64> {
    let lake_path = ensure_lake(lake)?;
    let conn = open_lake_db(Path::new(&lake_path))?;
    let table = sanitize_table_name(table)?;
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {} (
            _ingested_at TIMESTAMP DEFAULT current_timestamp,
            _source_id VARCHAR,
            data JSON
        )",
        table
    ))?;
    let mut inserted = 0_u64;
    let sql = format!(
        "INSERT INTO {} (_source_id, data) VALUES (?1, json(?2))",
        table
    );
    let mut stmt = conn.prepare(&sql)?;
    for row in rows_json {
        stmt.execute(duckdb::params![source, row])?;
        inserted += 1;
    }
    Ok(inserted)
}

pub fn query_json(lake: &str, sql: &str) -> Result<String> {
    let lake_path = ensure_lake(lake)?;
    let conn = open_lake_db(Path::new(&lake_path))?;
    let mut stmt = conn.prepare(sql)?;
    let column_count = stmt.column_count();
    let column_names = stmt
        .column_names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let mut object = serde_json::Map::new();
        for idx in 0..column_count {
            let key = column_names
                .get(idx)
                .cloned()
                .unwrap_or_else(|| format!("col_{}", idx));
            let value = row
                .get::<_, String>(idx)
                .map(serde_json::Value::String)
                .or_else(|_| row.get::<_, i64>(idx).map(serde_json::Value::from))
                .or_else(|_| row.get::<_, f64>(idx).map(serde_json::Value::from))
                .or_else(|_| row.get::<_, bool>(idx).map(serde_json::Value::from))
                .unwrap_or(serde_json::Value::Null);
            object.insert(key, value);
        }
        out.push(serde_json::Value::Object(object));
    }
    Ok(serde_json::to_string(&out)?)
}
