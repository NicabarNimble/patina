//! Belief grounding and search
//!
//! Provides belief-related factual queries:
//! - Belief grounding: find evidence for/against a belief
//! - Belief search: FTS5 keyword search across beliefs
//!
//! The vector-similarity component of belief search stays in scry (semantic).
//! The factual components (FTS5, evidence lookup, reach analysis) move here.
//!
//! Moved from scry during semantic-structural split (Phase 1).

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::search::SearchResult;

fn local_db_path() -> Result<String> {
    Ok(patina::eventlog::patina_db_path()?
        .to_string_lossy()
        .to_string())
}

/// Search beliefs using FTS5 keyword matching
pub fn search_beliefs_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    // Check if belief_fts exists
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='belief_fts'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !exists {
        return Ok(Vec::new());
    }

    // Tokenize query for FTS5: join terms with OR
    let terms: Vec<&str> = query.split_whitespace().filter(|t| t.len() > 1).collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let fts_query = terms.join(" OR ");

    let mut stmt = conn.prepare(
        "SELECT b.id, b.statement, b.entrenchment, b.file_path,
                b.evidence_count, b.evidence_verified, b.applied_in,
                bm25(belief_fts) as score
         FROM belief_fts
         JOIN beliefs b ON b.id = belief_fts.id
         WHERE belief_fts MATCH ?1
         ORDER BY score
         LIMIT ?2",
    )?;

    let rows = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
        let id: String = row.get(0)?;
        let statement: String = row.get(1)?;
        let entrenchment: String = row.get(2)?;
        let file_path: String = row.get(3)?;
        let evidence_count: i32 = row.get(4)?;
        let evidence_verified: i32 = row.get(5)?;
        let applied_in: i32 = row.get(6)?;
        let bm25_score: f64 = row.get(7)?;

        let content = format_belief_content(
            &statement,
            &entrenchment,
            evidence_count,
            evidence_verified,
            applied_in,
        );

        Ok(SearchResult {
            content,
            score: -bm25_score as f32,
            event_type: "belief".to_string(),
            source_id: format!("belief:{} ({})", id, file_path),
            timestamp: String::new(),
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Fetch top-N reached code files for a belief, ordered by reach_score
pub fn fetch_reached_files(conn: &Connection, belief_id: &str, limit: usize) -> Vec<(String, f32)> {
    let mut stmt = match conn.prepare(
        "SELECT file_path, reach_score FROM belief_code_reach
         WHERE belief_id = ?1
         ORDER BY reach_score DESC
         LIMIT ?2",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stmt.query_map(rusqlite::params![belief_id, limit], |row| {
        let path: String = row.get(0)?;
        let score: f64 = row.get(1)?;
        Ok((path, score as f32))
    })
    .ok()
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// Execute belief grounding — find evidence for/against a belief (CLI output)
pub fn execute_belief_grounding(belief_id: &str, limit: usize, json: bool) -> Result<()> {
    let db_path = local_db_path()?;
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Look up belief
    let (statement, entrenchment, file_path, evidence_count, evidence_verified, applied_in): (
        String,
        String,
        String,
        i32,
        i32,
        i32,
    ) = conn
        .query_row(
            "SELECT statement, entrenchment, file_path, evidence_count, evidence_verified, applied_in
             FROM beliefs WHERE id = ?",
            [belief_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .with_context(|| format!("Belief '{}' not found", belief_id))?;

    // Fetch reached code files
    let reached = fetch_reached_files(&conn, belief_id, limit);

    // Fetch related beliefs via co-citation in belief_evidence
    let related = fetch_related_beliefs(&conn, belief_id, limit);

    if json {
        let result = serde_json::json!({
            "belief": {
                "id": belief_id,
                "statement": statement,
                "entrenchment": entrenchment,
                "file_path": file_path,
                "evidence_count": evidence_count,
                "evidence_verified": evidence_verified,
                "applied_in": applied_in,
            },
            "reached_code": reached.iter().map(|(path, score)| {
                serde_json::json!({"file": path, "reach_score": score})
            }).collect::<Vec<_>>(),
            "related_beliefs": related,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Belief: {}", belief_id);
        println!("  Statement: {}", statement);
        println!("  Entrenchment: {}", entrenchment);
        println!("  File: {}", file_path);
        println!(
            "  Evidence: {}/{} verified, {} applied",
            evidence_verified, evidence_count, applied_in
        );

        if !reached.is_empty() {
            println!("\nReached code:");
            for (path, score) in &reached {
                println!("  {:.3}  {}", score, path);
            }
        }

        if !related.is_empty() {
            println!("\nRelated beliefs:");
            for r in &related {
                println!(
                    "  {} — {}",
                    r["id"].as_str().unwrap_or("?"),
                    r["statement"].as_str().unwrap_or("?")
                );
            }
        }
    }

    Ok(())
}

/// Execute belief grounding and return JSON string (for MCP)
pub fn execute_belief_grounding_json(belief_id: &str, limit: usize) -> Result<String> {
    let db_path = local_db_path()?;
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    let (statement, entrenchment, file_path, evidence_count, evidence_verified, applied_in): (
        String,
        String,
        String,
        i32,
        i32,
        i32,
    ) = conn
        .query_row(
            "SELECT statement, entrenchment, file_path, evidence_count, evidence_verified, applied_in
             FROM beliefs WHERE id = ?",
            [belief_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .with_context(|| format!("Belief '{}' not found", belief_id))?;

    let reached = fetch_reached_files(&conn, belief_id, limit);
    let related = fetch_related_beliefs(&conn, belief_id, limit);

    let result = serde_json::json!({
        "belief": {
            "id": belief_id,
            "statement": statement,
            "entrenchment": entrenchment,
            "file_path": file_path,
            "evidence_count": evidence_count,
            "evidence_verified": evidence_verified,
            "applied_in": applied_in,
        },
        "reached_code": reached.iter().map(|(path, score)| {
            serde_json::json!({"file": path, "reach_score": score})
        }).collect::<Vec<_>>(),
        "related_beliefs": related,
    });

    Ok(serde_json::to_string_pretty(&result)?)
}

/// Fetch beliefs related to the given belief via shared evidence
fn fetch_related_beliefs(
    conn: &Connection,
    belief_id: &str,
    limit: usize,
) -> Vec<serde_json::Value> {
    // Try to find beliefs that share evidence sources
    let sql = "SELECT DISTINCT b.id, b.statement, b.entrenchment
               FROM beliefs b
               WHERE b.id != ?1
               AND b.id IN (
                   SELECT DISTINCT be2.belief_id
                   FROM belief_evidence be1
                   JOIN belief_evidence be2 ON be1.source = be2.source
                   WHERE be1.belief_id = ?1
                   AND be2.belief_id != ?1
               )
               LIMIT ?2";

    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stmt.query_map(rusqlite::params![belief_id, limit], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "statement": row.get::<_, String>(1)?,
            "entrenchment": row.get::<_, String>(2)?,
        }))
    })
    .ok()
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn format_belief_content(
    statement: &str,
    entrenchment: &str,
    evidence_count: i32,
    evidence_verified: i32,
    applied_in: i32,
) -> String {
    let mut parts = vec![format!(
        "evidence: {}/{}",
        evidence_count, evidence_verified
    )];
    if applied_in > 0 {
        parts.push(format!("{} applied", applied_in));
    }
    format!("{} ({}, {})", statement, entrenchment, parts.join(", "))
}
