//! Generic projection engine — schema-driven materialization.
//!
//! "Do X": Project events from events.db into read model tables declared
//! by `[[projections]]` in installed schemas.
//!
//! The engine reads schema declarations and generates SQL mechanically.
//! No connector-specific table names, column mappings, or domain logic.
//! All behavior is derived from `[[projections]]` entries in schema.toml.

use anyhow::Result;
use rusqlite::Connection;

use crate::commands::schema::{ColumnDef, ProjectionDef, SchemaMetadata};

// ============================================================================
// Public interface
// ============================================================================

/// Stats from a generic projection run.
pub struct ProjectionStats {
    pub tables_created: usize,
    pub rows_projected: usize,
}

/// Project events from events.db into read model tables for all installed schemas.
///
/// For each schema with `[[projections]]` entries:
/// 1. Run one-time forge→github migration (tables + event types)
/// 2. CREATE TABLE IF NOT EXISTS with declared columns + provenance columns
/// 3. INSERT OR REPLACE from events.db using json_extract with declared json_paths
/// 4. Dedup by declared primary_key (latest seq wins)
pub fn project_from_schemas(patina_conn: &Connection) -> Result<ProjectionStats> {
    let schemas = crate::commands::schema::load_all_installed()?;

    // One-time migration: forge_* tables → github_*, forge.* events → github.*
    migrate_forge_tables(patina_conn)?;
    migrate_forge_events()?;

    // Schema registry (still needed for write-side event type resolution)
    create_schema_registry(patina_conn)?;
    populate_schema_registry(patina_conn, &schemas)?;

    patina::eventlog::ensure_events_db()?;
    let events_path = patina::eventlog::EVENTS_DB;
    patina_conn.execute("ATTACH DATABASE ?1 AS events_db", [events_path])?;

    let mut stats = ProjectionStats {
        tables_created: 0,
        rows_projected: 0,
    };

    for schema in &schemas {
        for projection in &schema.projections {
            let fact = schema.facts.iter().find(|f| f.name == projection.fact);
            let event_type = match fact {
                Some(f) => &f.event_type,
                None => continue,
            };

            if !is_safe_identifier(&projection.table) {
                eprintln!(
                    "Warning: skipping projection with invalid table name: {}",
                    projection.table
                );
                continue;
            }

            // CREATE TABLE IF NOT EXISTS from declared columns
            let ddl = generate_create_table(projection);
            patina_conn.execute_batch(&ddl)?;
            stats.tables_created += 1;

            // INSERT OR REPLACE from events.db
            let insert_sql = generate_insert_sql(projection, event_type);
            let count = patina_conn.execute(&insert_sql, [])?;
            stats.rows_projected += count;
        }
    }

    patina_conn.execute("DETACH DATABASE events_db", [])?;

    Ok(stats)
}

// ============================================================================
// Schema registry — populated from installed schemas
// ============================================================================

/// Create schema_registry table in patina.db.
fn create_schema_registry(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_registry (
            schema_name  TEXT NOT NULL,
            event_type   TEXT NOT NULL,
            fact_name    TEXT NOT NULL,
            table_name   TEXT NOT NULL,
            fts_fields   TEXT,
            corpus_query TEXT,
            offset_slot  INTEGER,
            PRIMARY KEY (schema_name, event_type)
        );",
    )?;
    // Migration: drop priority column if it exists (no longer needed without
    // multi-schema table sharing). SQLite doesn't support DROP COLUMN in older
    // versions, so we just leave it if present — it's harmless.
    Ok(())
}

/// Populate schema_registry from installed schemas (idempotent — rebuilds each time).
fn populate_schema_registry(conn: &Connection, schemas: &[SchemaMetadata]) -> Result<()> {
    conn.execute("DELETE FROM schema_registry", [])?;

    let mut stmt = conn.prepare(
        "INSERT OR REPLACE INTO schema_registry
         (schema_name, event_type, fact_name, table_name, fts_fields, corpus_query, offset_slot)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;

    for schema in schemas {
        let corpus_query = schema.embedding.as_ref().map(|e| e.corpus_query.trim());
        let offset_slot = schema.embedding.as_ref().map(|e| e.offset_slot);

        for fact in &schema.facts {
            let index = schema.indexes.iter().find(|i| i.fact == fact.name);
            let table_name = index.map(|i| i.table.as_str()).unwrap_or("");
            let fts_fields =
                index.map(|i| serde_json::to_string(&i.fts_fields).unwrap_or_default());

            stmt.execute(rusqlite::params![
                &schema.schema.name,
                &fact.event_type,
                &fact.name,
                table_name,
                fts_fields,
                corpus_query,
                offset_slot,
            ])?;
        }
    }

    Ok(())
}

// ============================================================================
// SQL generation — pure functions, no hardcoded table/column names
// ============================================================================

/// Generate CREATE TABLE DDL from a projection declaration.
///
/// Declared columns come from the schema. The engine auto-adds two provenance
/// columns (event_seq, ingested_at) for cross-db tracing back to events.db.
fn generate_create_table(projection: &ProjectionDef) -> String {
    let mut sql = format!(
        "CREATE TABLE IF NOT EXISTS {} (\n",
        projection.table
    );

    for (i, col) in projection.columns.iter().enumerate() {
        if i > 0 {
            sql.push_str(",\n");
        }
        sql.push_str(&format!("    {} {}", col.name, col.col_type));
        if col.name == projection.primary_key {
            sql.push_str(" PRIMARY KEY");
        }
    }

    // Provenance columns — infrastructure, not domain
    sql.push_str(",\n    event_seq INTEGER");
    sql.push_str(",\n    ingested_at TEXT");
    sql.push_str("\n);");
    sql
}

/// Generate INSERT OR REPLACE SQL from a projection declaration.
///
/// Selects json_extract(e.data, json_path) for each declared column,
/// plus e.seq and e.timestamp for provenance. Deduplicates by primary_key
/// (latest seq wins).
fn generate_insert_sql(projection: &ProjectionDef, event_type: &str) -> String {
    let col_names: Vec<&str> = projection
        .columns
        .iter()
        .map(|c| c.name.as_str())
        .chain(["event_seq", "ingested_at"])
        .collect();

    let selects: Vec<String> = projection
        .columns
        .iter()
        .map(|c| format!("json_extract(e.data, '{}')", c.json_path))
        .chain([
            "e.seq".to_string(),
            "e.timestamp".to_string(),
        ])
        .collect();

    // Dedup: for each primary_key value, take the row with the highest seq
    // (latest version). This is the same strategy as the old hardcoded path.
    format!(
        "INSERT OR REPLACE INTO {table} ({cols}) \
         SELECT {selects} \
         FROM events_db.eventlog e \
         WHERE e.event_type = '{event_type}' \
         AND e.seq = (\
             SELECT MAX(e2.seq) FROM events_db.eventlog e2 \
             WHERE e2.event_type = '{event_type}' \
             AND json_extract(e2.data, '$.{pk}') = json_extract(e.data, '$.{pk}')\
         )",
        table = projection.table,
        cols = col_names.join(", "),
        selects = selects.join(", "),
        event_type = event_type,
        pk = projection.primary_key,
    )
}

// ============================================================================
// One-time forge → github migration
// ============================================================================

/// Rename forge_* tables to github_* in patina.db (one-time, idempotent).
fn migrate_forge_tables(conn: &Connection) -> Result<()> {
    rename_table_if_needed(conn, "forge_issues", "github_issues")?;
    rename_table_if_needed(conn, "forge_prs", "github_prs")?;

    // Drop forge_refs (sync backlog) — not schema-driven, no longer referenced
    drop_table_if_exists(conn, "forge_refs")?;

    // Drop stale indexes from the old tables
    let _ = conn.execute("DROP INDEX IF EXISTS idx_forge_issues_state", []);
    let _ = conn.execute("DROP INDEX IF EXISTS idx_forge_issues_updated", []);
    let _ = conn.execute("DROP INDEX IF EXISTS idx_forge_prs_state", []);
    let _ = conn.execute("DROP INDEX IF EXISTS idx_forge_prs_merged", []);
    let _ = conn.execute("DROP INDEX IF EXISTS idx_forge_refs_pending", []);

    Ok(())
}

/// Rename a table if the source exists and the target does not.
fn rename_table_if_needed(conn: &Connection, from: &str, to: &str) -> Result<()> {
    let from_exists = table_exists(conn, from)?;
    if !from_exists {
        return Ok(());
    }

    let to_exists = table_exists(conn, to)?;
    if to_exists {
        // Both exist — target was already created (e.g., by a previous generic
        // projection run). Drop the old source table; the target has fresh data.
        conn.execute(&format!("DROP TABLE {}", from), [])?;
    } else {
        conn.execute(&format!("ALTER TABLE {} RENAME TO {}", from, to), [])?;
    }
    Ok(())
}

fn drop_table_if_exists(conn: &Connection, name: &str) -> Result<()> {
    conn.execute(&format!("DROP TABLE IF EXISTS {}", name), [])?;
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Migrate forge.* event types to github.* in events.db (one-time, idempotent).
fn migrate_forge_events() -> Result<()> {
    let events_conn = patina::eventlog::open_events_db()?;

    let forge_count: i64 = events_conn.query_row(
        "SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'forge.%'",
        [],
        |row| row.get(0),
    )?;

    if forge_count > 0 {
        events_conn.execute(
            "UPDATE eventlog SET event_type = 'github.issue' WHERE event_type = 'forge.issue'",
            [],
        )?;
        events_conn.execute(
            "UPDATE eventlog SET event_type = 'github.pr' WHERE event_type = 'forge.pr'",
            [],
        )?;
        eprintln!(
            "  Migrated {} forge.* events to github.* in events.db",
            forge_count
        );
    }

    Ok(())
}

// ============================================================================
// Shared utilities
// ============================================================================

/// Check that a string is a safe SQL identifier (alphanumeric + underscore only).
fn is_safe_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_column(name: &str, col_type: &str, json_path: &str) -> ColumnDef {
        ColumnDef {
            name: name.to_string(),
            col_type: col_type.to_string(),
            json_path: json_path.to_string(),
        }
    }

    fn make_projection(table: &str, pk: &str, columns: Vec<ColumnDef>) -> ProjectionDef {
        ProjectionDef {
            fact: "test-fact".to_string(),
            table: table.to_string(),
            primary_key: pk.to_string(),
            columns,
        }
    }

    // --- DDL generation tests ---

    #[test]
    fn test_generate_create_table_basic() {
        let proj = make_projection(
            "test_items",
            "id",
            vec![
                make_column("id", "INTEGER", "$.id"),
                make_column("name", "TEXT", "$.name"),
            ],
        );

        let ddl = generate_create_table(&proj);
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS test_items"));
        assert!(ddl.contains("id INTEGER PRIMARY KEY"));
        assert!(ddl.contains("name TEXT"));
        assert!(ddl.contains("event_seq INTEGER"));
        assert!(ddl.contains("ingested_at TEXT"));
    }

    #[test]
    fn test_generate_create_table_primary_key_placement() {
        let proj = make_projection(
            "messages",
            "ts",
            vec![
                make_column("ts", "TEXT", "$.ts"),
                make_column("channel", "TEXT", "$.channel"),
                make_column("text", "TEXT", "$.text"),
            ],
        );

        let ddl = generate_create_table(&proj);
        assert!(ddl.contains("ts TEXT PRIMARY KEY"));
        assert!(!ddl.contains("channel TEXT PRIMARY KEY"));
    }

    // --- INSERT SQL generation tests ---

    #[test]
    fn test_generate_insert_sql_basic() {
        let proj = make_projection(
            "github_issues",
            "number",
            vec![
                make_column("number", "INTEGER", "$.number"),
                make_column("title", "TEXT", "$.title"),
            ],
        );

        let sql = generate_insert_sql(&proj, "github.issue");
        assert!(sql.contains("INSERT OR REPLACE INTO github_issues"));
        assert!(sql.contains("number, title, event_seq, ingested_at"));
        assert!(sql.contains("json_extract(e.data, '$.number')"));
        assert!(sql.contains("json_extract(e.data, '$.title')"));
        assert!(sql.contains("e.seq"));
        assert!(sql.contains("e.timestamp"));
        assert!(sql.contains("WHERE e.event_type = 'github.issue'"));
        assert!(sql.contains("json_extract(e2.data, '$.number') = json_extract(e.data, '$.number')"));
    }

    #[test]
    fn test_generate_insert_sql_different_primary_key() {
        let proj = make_projection(
            "slack_messages",
            "ts",
            vec![
                make_column("ts", "TEXT", "$.ts"),
                make_column("text", "TEXT", "$.text"),
            ],
        );

        let sql = generate_insert_sql(&proj, "slack.message");
        assert!(sql.contains("WHERE e.event_type = 'slack.message'"));
        assert!(sql.contains("json_extract(e2.data, '$.ts') = json_extract(e.data, '$.ts')"));
    }

    // --- Identifier safety tests ---

    #[test]
    fn test_safe_identifier() {
        assert!(is_safe_identifier("github_issues"));
        assert!(is_safe_identifier("test123"));
        assert!(!is_safe_identifier(""));
        assert!(!is_safe_identifier("bad; DROP TABLE"));
        assert!(!is_safe_identifier("has space"));
    }

    // --- Migration tests ---

    #[test]
    fn test_rename_table_source_exists_target_does_not() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE forge_issues (id INTEGER PRIMARY KEY, title TEXT);")
            .unwrap();
        conn.execute("INSERT INTO forge_issues VALUES (1, 'test')", [])
            .unwrap();

        rename_table_if_needed(&conn, "forge_issues", "github_issues").unwrap();

        assert!(!table_exists(&conn, "forge_issues").unwrap());
        assert!(table_exists(&conn, "github_issues").unwrap());

        let title: String = conn
            .query_row(
                "SELECT title FROM github_issues WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "test");
    }

    #[test]
    fn test_rename_table_both_exist_drops_source() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE forge_issues (id INTEGER PRIMARY KEY, title TEXT);
             CREATE TABLE github_issues (id INTEGER PRIMARY KEY, title TEXT);
             INSERT INTO forge_issues VALUES (1, 'old');
             INSERT INTO github_issues VALUES (2, 'new');",
        )
        .unwrap();

        rename_table_if_needed(&conn, "forge_issues", "github_issues").unwrap();

        assert!(!table_exists(&conn, "forge_issues").unwrap());
        assert!(table_exists(&conn, "github_issues").unwrap());
    }

    #[test]
    fn test_rename_table_source_does_not_exist_noop() {
        let conn = Connection::open_in_memory().unwrap();
        rename_table_if_needed(&conn, "nonexistent", "target").unwrap();
        assert!(!table_exists(&conn, "target").unwrap());
    }

    // --- End-to-end projection test ---

    #[test]
    fn test_projection_creates_table_and_inserts() {
        let conn = Connection::open_in_memory().unwrap();

        // Simulate events.db with eventlog
        conn.execute_batch(
            "CREATE TABLE eventlog (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_file TEXT,
                data TEXT NOT NULL,
                provenance TEXT NOT NULL DEFAULT 'local',
                CHECK(json_valid(data))
            );
            INSERT INTO eventlog (event_type, timestamp, source_id, data)
            VALUES ('test.item', '2026-01-01', '1',
                    '{\"id\": 1, \"name\": \"first\"}');
            INSERT INTO eventlog (event_type, timestamp, source_id, data)
            VALUES ('test.item', '2026-01-02', '2',
                    '{\"id\": 2, \"name\": \"second\"}');
            -- Duplicate with higher seq (should win)
            INSERT INTO eventlog (event_type, timestamp, source_id, data)
            VALUES ('test.item', '2026-01-03', '1',
                    '{\"id\": 1, \"name\": \"first-updated\"}');",
        )
        .unwrap();

        let proj = make_projection(
            "test_items",
            "id",
            vec![
                make_column("id", "INTEGER", "$.id"),
                make_column("name", "TEXT", "$.name"),
            ],
        );

        // CREATE TABLE
        let ddl = generate_create_table(&proj);
        conn.execute_batch(&ddl).unwrap();

        // INSERT — using main db as if it were attached as events_db
        // We need to simulate the ATTACH by using the same db
        let insert_sql = generate_insert_sql(&proj, "test.item")
            .replace("events_db.eventlog", "eventlog");
        let count = conn.execute(&insert_sql, []).unwrap();

        // Should have 2 rows (id=1 deduped to latest, id=2)
        assert_eq!(count, 2);

        let name: String = conn
            .query_row(
                "SELECT name FROM test_items WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "first-updated", "latest seq should win for dedup");

        // Provenance columns populated
        let seq: i64 = conn
            .query_row(
                "SELECT event_seq FROM test_items WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(seq, 3, "event_seq should be the seq of the winning row");
    }
}
