use anyhow::{Context, Result};
use duckdb::Connection;
use std::path::{Path, PathBuf};

use crate::mother::KnowledgeRuntimeStore;

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

#[allow(dead_code)]
pub fn load_cursor(
    store: &KnowledgeRuntimeStore,
    lake: &str,
    source: &str,
    data_type: &str,
) -> Result<Option<String>> {
    store.load_lake_cursor(lake, source, data_type)
}

#[allow(dead_code)]
pub fn save_cursor(
    store: &KnowledgeRuntimeStore,
    update: &crate::mother::state::LakeCursorUpdate<'_>,
) -> Result<()> {
    store.save_lake_cursor(update)
}

#[allow(dead_code)]
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
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let mut object = serde_json::Map::new();
        let column_count = row.as_ref().column_count();
        for idx in 0..column_count {
            let key = format!("col_{}", idx);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mother::state::LakeCursorUpdate;

    fn with_temp_patina_home<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::TempDir::new().unwrap();
        let home = temp.path().join("patina-home");
        std::fs::create_dir_all(&home).unwrap();
        let old_home = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &home);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&home)));
        match old_home {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn lake_table_append_query_and_cursor_roundtrip() {
        with_temp_patina_home(|home| {
            let state_db = home.join("mother/state.db");
            let store = KnowledgeRuntimeStore::new_with_project(
                state_db,
                crate::mother::ProjectUid::new("2bdc808e").unwrap(),
            );

            ensure_table("default", "gh_issues").unwrap();

            let rows = vec![
                r#"{"id":1,"title":"issue-one"}"#.to_string(),
                r#"{"id":2,"title":"issue-two"}"#.to_string(),
            ];
            let inserted = append_json_batch("default", "gh_issues", "gh-main", &rows).unwrap();
            assert_eq!(inserted, 2);

            let queried = query_json(
                "default",
                "SELECT _source_id, json_extract_string(data, '$.title') AS title FROM gh_issues ORDER BY title",
            )
            .unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&queried).unwrap();
            let rows = parsed.as_array().unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0]["col_0"], "gh-main");
            assert_eq!(rows[0]["col_1"], "issue-one");
            assert_eq!(rows[1]["col_1"], "issue-two");

            save_cursor(
                &store,
                &LakeCursorUpdate {
                    lake_name: "default",
                    source_name: "gh-main",
                    data_type: "issues",
                    cursor_value: Some("cursor-v1"),
                    records_written: 2,
                    status: "ok",
                    last_error: None,
                },
            )
            .unwrap();
            let cursor = load_cursor(&store, "default", "gh-main", "issues").unwrap();
            assert_eq!(cursor.as_deref(), Some("cursor-v1"));
        });
    }

    #[test]
    fn lake_table_name_validation_rejects_invalid_names() {
        with_temp_patina_home(|_| {
            let err = ensure_table("default", "bad table name").unwrap_err();
            assert!(err.to_string().contains("invalid table name"));
        });
    }
}
