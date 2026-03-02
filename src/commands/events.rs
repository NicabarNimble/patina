//! Event store management — export/import JSONL replica for durability.
//!
//! events.db is irreplaceable runtime data. This module provides the
//! Schickling pattern's durability half: SQLite-as-runtime + JSONL-as-replica.
//!
//! Export: events.db → layer/events.jsonl (incremental, at-least-once)
//! Import: layer/events.jsonl → events.db (disaster recovery, INSERT OR IGNORE)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use patina::eventlog;

/// JSONL path within the project (committed to git)
const JSONL_PATH: &str = "layer/events.jsonl";

/// Metadata key for tracking last exported seq in events.db
const LAST_EXPORTED_SEQ_KEY: &str = "last_exported_seq";

/// A single event row, serialized as one JSON line in the JSONL file.
#[derive(Debug, Serialize, Deserialize)]
struct EventRow {
    seq: i64,
    event_type: String,
    timestamp: String,
    source_id: String,
    source_file: Option<String>,
    data: serde_json::Value,
}

/// Export new events from events.db to layer/events.jsonl.
///
/// At-least-once: JSONL write before marker update. If crash between
/// JSONL write and marker update, next export re-appends same events.
/// Import deduplicates on seq.
pub fn export() -> Result<()> {
    let conn = eventlog::open_events_db()?;

    // Get last exported seq from scrape_meta
    let last_seq: i64 = conn
        .query_row(
            "SELECT COALESCE(value, '0') FROM scrape_meta WHERE key = ?1",
            [LAST_EXPORTED_SEQ_KEY],
            |row| {
                let val: String = row.get(0)?;
                Ok(val.parse::<i64>().unwrap_or(0))
            },
        )
        .unwrap_or(0);

    // Select new events
    let mut stmt = conn.prepare(
        "SELECT seq, event_type, timestamp, source_id, source_file, data
         FROM eventlog
         WHERE seq > ?1
         ORDER BY seq ASC",
    )?;

    let rows: Vec<EventRow> = stmt
        .query_map([last_seq], |row| {
            let data_str: String = row.get(5)?;
            let data: serde_json::Value =
                serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null);
            Ok(EventRow {
                seq: row.get(0)?,
                event_type: row.get(1)?,
                timestamp: row.get(2)?,
                source_id: row.get(3)?,
                source_file: row.get(4)?,
                data,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    if rows.is_empty() {
        println!(
            "  No new events to export (last exported seq: {})",
            last_seq
        );
        return Ok(());
    }

    let max_seq = rows.last().map(|r| r.seq).unwrap_or(last_seq);
    let count = rows.len();

    // Ensure layer/ directory exists
    if let Some(parent) = Path::new(JSONL_PATH).parent() {
        fs::create_dir_all(parent)?;
    }

    // Append to JSONL file (at-least-once: write JSONL before updating marker)
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(JSONL_PATH)
        .with_context(|| format!("failed to open {JSONL_PATH}"))?;

    for row in &rows {
        let line = serde_json::to_string(row)?;
        writeln!(file, "{}", line)?;
    }

    // fsync the file to ensure durability
    file.sync_all()?;

    // NOW update the marker (after JSONL is safely on disk)
    conn.execute(
        "INSERT OR REPLACE INTO scrape_meta (key, value) VALUES (?1, ?2)",
        rusqlite::params![LAST_EXPORTED_SEQ_KEY, max_seq.to_string()],
    )?;

    println!(
        "  Exported {} events to {} (seq {}..{})",
        count,
        JSONL_PATH,
        last_seq + 1,
        max_seq
    );

    Ok(())
}

/// Import events from a JSONL file into events.db.
///
/// Disaster recovery: rebuilds events.db from its JSONL replica.
/// Uses INSERT OR IGNORE with seq as PRIMARY KEY — same seq = same event, skip.
/// Does NOT update last_exported_seq marker (this is recovery, not export).
pub fn import(path: &str) -> Result<()> {
    let import_path = Path::new(path);
    if !import_path.exists() {
        anyhow::bail!("File not found: {}", path);
    }

    // Ensure events.db is initialized
    eventlog::ensure_events_db()?;
    let conn = eventlog::open_events_db()?;

    let file = File::open(import_path).with_context(|| format!("failed to open {path}"))?;
    let reader = BufReader::new(file);

    let mut imported = 0i64;
    let mut skipped = 0i64;
    let mut errors = 0i64;

    let tx = conn.unchecked_transaction()?;

    for (line_num, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("  warning: line {}: read error: {e}", line_num + 1);
                errors += 1;
                continue;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let row: EventRow = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  warning: line {}: parse error: {e}", line_num + 1);
                errors += 1;
                continue;
            }
        };

        let data_str = row.data.to_string();
        let result = tx.execute(
            "INSERT OR IGNORE INTO eventlog (seq, event_type, timestamp, source_id, source_file, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                row.seq,
                row.event_type,
                row.timestamp,
                row.source_id,
                row.source_file,
                data_str,
            ],
        );

        match result {
            Ok(0) => skipped += 1,
            Ok(_) => imported += 1,
            Err(e) => {
                eprintln!("  warning: line {}: insert error: {e}", line_num + 1);
                errors += 1;
            }
        }
    }

    tx.commit()?;

    println!(
        "  Imported {} events, skipped {} duplicates",
        imported, skipped
    );
    if errors > 0 {
        println!("  {} lines had errors", errors);
    }

    Ok(())
}

/// Best-effort export for session end hook.
/// Warns on failure, never blocks session archival.
pub fn export_best_effort() {
    if let Err(e) = export() {
        eprintln!("patina: warning: event export failed: {e}");
    }
}
