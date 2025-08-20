use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use super::{PatternPointer, PatternSpread, ComplianceStatus};

pub struct PointerDatabase {
    conn: Connection,
}

impl PointerDatabase {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database: {}", db_path.display()))?;
        
        let mut db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    fn initialize_schema(&mut self) -> Result<()> {
        let schema = include_str!("../../resources/schema/pointers.sql");
        self.conn.execute_batch(schema)
            .context("Failed to initialize pointer database schema")?;
        Ok(())
    }

    pub fn insert_pointer(&mut self, pointer: &PatternPointer) -> Result<i64> {
        let commit_hash = pointer.commit_hash.as_deref().unwrap_or("");
        
        self.conn.execute(
            "INSERT OR REPLACE INTO pattern_pointers 
             (pattern_id, file_path, line_start, line_end, symbol, compliance, 
              explanation, suggested_fix, confidence, commit_hash, 
              created_at, last_verified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                pointer.pattern_id,
                pointer.file_path.to_string_lossy(),
                pointer.line_start,
                pointer.line_end,
                pointer.symbol,
                pointer.compliance.to_string(),
                pointer.explanation,
                pointer.suggested_fix,
                pointer.confidence,
                commit_hash,
                pointer.created_at.to_rfc3339(),
                pointer.last_verified.to_rfc3339(),
            ],
        )?;
        
        let id = self.conn.last_insert_rowid();
        
        // Update pattern spread statistics
        self.update_pattern_spread(&pointer.pattern_id)?;
        
        Ok(id)
    }

    pub fn get_pointers_by_pattern(&self, pattern_id: &str) -> Result<Vec<PatternPointer>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pattern_id, file_path, line_start, line_end, symbol,
                    compliance, explanation, suggested_fix, confidence,
                    created_at, last_verified, commit_hash
             FROM pattern_pointers
             WHERE pattern_id = ?1
             ORDER BY file_path"
        )?;
        
        let pointers = stmt.query_map([pattern_id], |row| {
            Ok(PatternPointer {
                id: Some(row.get(0)?),
                pattern_id: row.get(1)?,
                file_path: PathBuf::from(row.get::<_, String>(2)?),
                line_start: row.get(3)?,
                line_end: row.get(4)?,
                symbol: row.get(5)?,
                compliance: ComplianceStatus::from_str(&row.get::<_, String>(6)?),
                explanation: row.get(7)?,
                suggested_fix: row.get(8)?,
                confidence: row.get(9)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                last_verified: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                commit_hash: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(pointers)
    }

    pub fn get_pointers_by_file(&self, file_path: &Path) -> Result<Vec<PatternPointer>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pattern_id, file_path, line_start, line_end, symbol,
                    compliance, explanation, suggested_fix, confidence,
                    created_at, last_verified, commit_hash
             FROM pattern_pointers
             WHERE file_path = ?1
             ORDER BY pattern_id"
        )?;
        
        let pointers = stmt.query_map([file_path.to_string_lossy()], |row| {
            Ok(PatternPointer {
                id: Some(row.get(0)?),
                pattern_id: row.get(1)?,
                file_path: PathBuf::from(row.get::<_, String>(2)?),
                line_start: row.get(3)?,
                line_end: row.get(4)?,
                symbol: row.get(5)?,
                compliance: ComplianceStatus::from_str(&row.get::<_, String>(6)?),
                explanation: row.get(7)?,
                suggested_fix: row.get(8)?,
                confidence: row.get(9)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                last_verified: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                commit_hash: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(pointers)
    }

    pub fn update_pattern_spread(&mut self, pattern_id: &str) -> Result<()> {
        // Count implementations by compliance status
        let mut stmt = self.conn.prepare(
            "SELECT compliance, COUNT(*) FROM pattern_pointers 
             WHERE pattern_id = ?1 
             GROUP BY compliance"
        )?;
        
        let mut total = 0;
        let mut compliant = 0;
        let mut violation = 0;
        let mut partial = 0;
        
        let counts = stmt.query_map([pattern_id], |row| {
            let status: String = row.get(0)?;
            let count: i32 = row.get(1)?;
            Ok((status, count))
        })?;
        
        for count in counts {
            let (status, n) = count?;
            total += n;
            match status.as_str() {
                "compliant" => compliant = n,
                "violation" => violation = n,
                "partial" => partial = n,
                _ => {}
            }
        }
        
        // Update or insert spread record
        self.conn.execute(
            "INSERT OR REPLACE INTO pattern_spread 
             (pattern_id, total_implementations, compliant_count, 
              violation_count, partial_count, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                pattern_id,
                total,
                compliant,
                violation,
                partial,
                Utc::now().to_rfc3339(),
            ],
        )?;
        
        Ok(())
    }

    pub fn get_pattern_spread(&self, pattern_id: &str) -> Result<PatternSpread> {
        let spread = self.conn.query_row(
            "SELECT total_implementations, compliant_count, violation_count, 
                    partial_count, last_updated
             FROM pattern_spread
             WHERE pattern_id = ?1",
            [pattern_id],
            |row| {
                Ok(PatternSpread {
                    pattern_id: pattern_id.to_string(),
                    total_implementations: row.get::<_, i32>(0)? as usize,
                    compliant_count: row.get::<_, i32>(1)? as usize,
                    violation_count: row.get::<_, i32>(2)? as usize,
                    partial_count: row.get::<_, i32>(3)? as usize,
                    compliance_rate: 0.0, // Calculated separately
                    last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                })
            },
        ).optional()?;
        
        let mut spread = spread.unwrap_or(PatternSpread {
            pattern_id: pattern_id.to_string(),
            total_implementations: 0,
            compliant_count: 0,
            violation_count: 0,
            partial_count: 0,
            compliance_rate: 0.0,
            last_updated: Utc::now(),
        });
        
        spread.compliance_rate = spread.compliance_rate();
        Ok(spread)
    }

    pub fn verify_pointer(&mut self, pointer_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE pattern_pointers 
             SET last_verified = ?1 
             WHERE id = ?2",
            params![Utc::now().to_rfc3339(), pointer_id],
        )?;
        Ok(())
    }
}