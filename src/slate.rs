use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SLATE_WORK_DIR: &str = "layer/slate/work";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SlateWorkItem {
    pub id: String,
    pub title: String,
    pub kind: String,
    #[serde(default = "default_status")]
    pub status: String,
    pub human_request: String,
    #[serde(default)]
    pub allium_anchors: Vec<String>,
    #[serde(default)]
    pub user_alignment: String,
    #[serde(default)]
    pub belief_refs: Vec<String>,
    #[serde(default)]
    pub proof_plan: Vec<String>,
    #[serde(default)]
    pub closure_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlateWorkProjection {
    pub item: SlateWorkItem,
    pub source_path: String,
}

pub struct SlateProjectStore {
    conn: Connection,
    db_path: PathBuf,
}

fn default_status() -> String {
    "draft".to_string()
}

impl SlateProjectStore {
    pub fn open_for_project(project_root: &Path) -> Result<Self> {
        let uid = crate::project::register_with_mother(project_root)?;
        let db_path = crate::paths::mother::projects::slate_db(&uid).map_err(anyhow::Error::msg)?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)
            .with_context(|| format!("open Slate projection DB {}", db_path.display()))?;
        let store = Self { conn, db_path };
        store.prepare_schema()?;
        Ok(store)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn prepare_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS slate_work_items (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                human_request TEXT NOT NULL,
                user_alignment TEXT NOT NULL,
                allium_anchors_json TEXT NOT NULL,
                belief_refs_json TEXT NOT NULL,
                proof_plan_json TEXT NOT NULL,
                closure_evidence_json TEXT NOT NULL,
                source_path TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_slate_work_items_status
            ON slate_work_items(status);

            CREATE INDEX IF NOT EXISTS idx_slate_work_items_kind
            ON slate_work_items(kind);

            CREATE TABLE IF NOT EXISTS slate_work_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                work_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                source_path TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_slate_work_events_work
            ON slate_work_events(work_id, created_at DESC);
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_work(&self, projection: &SlateWorkProjection) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO slate_work_items (
                id, title, kind, status, human_request, user_alignment,
                allium_anchors_json, belief_refs_json, proof_plan_json,
                closure_evidence_json, source_path, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                kind = excluded.kind,
                status = excluded.status,
                human_request = excluded.human_request,
                user_alignment = excluded.user_alignment,
                allium_anchors_json = excluded.allium_anchors_json,
                belief_refs_json = excluded.belief_refs_json,
                proof_plan_json = excluded.proof_plan_json,
                closure_evidence_json = excluded.closure_evidence_json,
                source_path = excluded.source_path,
                updated_at = excluded.updated_at
            "#,
            params![
                projection.item.id,
                projection.item.title,
                projection.item.kind,
                projection.item.status,
                projection.item.human_request,
                projection.item.user_alignment,
                serde_json::to_string(&projection.item.allium_anchors)?,
                serde_json::to_string(&projection.item.belief_refs)?,
                serde_json::to_string(&projection.item.proof_plan)?,
                serde_json::to_string(&projection.item.closure_evidence)?,
                projection.source_path,
            ],
        )?;
        Ok(())
    }

    pub fn refresh_from_project(&self, project_root: &Path) -> Result<Vec<SlateWorkProjection>> {
        let projections = scan_project_work(project_root)?;
        self.conn.execute("DELETE FROM slate_work_items", [])?;
        for projection in &projections {
            self.upsert_work(projection)?;
        }
        Ok(projections)
    }

    pub fn list_work(&self) -> Result<Vec<SlateWorkProjection>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, title, kind, status, human_request, user_alignment,
                   allium_anchors_json, belief_refs_json, proof_plan_json,
                   closure_evidence_json, source_path
            FROM slate_work_items
            ORDER BY status, kind, id
            "#,
        )?;
        let rows = stmt
            .query_map([], |row| {
                let allium_json: String = row.get(6)?;
                let belief_json: String = row.get(7)?;
                let proof_json: String = row.get(8)?;
                let closure_json: String = row.get(9)?;
                Ok(SlateWorkProjection {
                    item: SlateWorkItem {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        kind: row.get(2)?,
                        status: row.get(3)?,
                        human_request: row.get(4)?,
                        user_alignment: row.get(5)?,
                        allium_anchors: serde_json::from_str(&allium_json).unwrap_or_default(),
                        belief_refs: serde_json::from_str(&belief_json).unwrap_or_default(),
                        proof_plan: serde_json::from_str(&proof_json).unwrap_or_default(),
                        closure_evidence: serde_json::from_str(&closure_json).unwrap_or_default(),
                    },
                    source_path: row.get(10)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub fn scan_project_work(project_root: &Path) -> Result<Vec<SlateWorkProjection>> {
    let root = project_root.join(SLATE_WORK_DIR);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    fn walk(project_root: &Path, dir: &Path, out: &mut Vec<SlateWorkProjection>) -> Result<()> {
        for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(project_root, &path, out)?;
                continue;
            }
            if path.file_name().and_then(|name| name.to_str()) != Some("work.toml") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let item: SlateWorkItem =
                toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
            let source_path = path
                .strip_prefix(project_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            out.push(SlateWorkProjection { item, source_path });
        }
        Ok(())
    }

    walk(project_root, &root, &mut out)?;
    out.sort_by(|a, b| a.item.id.cmp(&b.item.id));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_project_living_slate_work_files() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path();
        std::fs::create_dir_all(project.join(".patina"))?;
        std::fs::create_dir_all(project.join("layer/slate/work/demo"))?;
        std::fs::write(
            project.join("layer/slate/work/demo/work.toml"),
            r#"
id = "demo"
title = "Demo"
kind = "build"
status = "ready"
human_request = "Build the thing"
allium_anchors = ["layer/allium/demo.allium"]
user_alignment = "User confirmed."
proof_plan = ["cargo test"]
"#,
        )?;

        let items = scan_project_work(project)?;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item.id, "demo");
        assert_eq!(items[0].source_path, "layer/slate/work/demo/work.toml");
        Ok(())
    }
}
