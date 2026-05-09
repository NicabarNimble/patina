use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, Connection};
use serde::{de::DeserializeOwned, Serialize};

use super::{
    Buffer, BufferState, Frame, FrameKind, MajorMode, MinorMode, ObservabilityGap,
    ObservabilityGapStatus, PayloadContract, Window, WindowConnectionState,
};

pub(crate) fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS mother_view_buffers (
            buffer_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            shape_id TEXT NOT NULL,
            state TEXT NOT NULL,
            created_at TEXT NOT NULL,
            stale_at TEXT,
            blocked_at TEXT,
            replaced_at TEXT,
            killed_at TEXT,
            major_mode TEXT NOT NULL,
            minor_modes_json TEXT NOT NULL,
            payload_contract TEXT NOT NULL,
            payload_version INTEGER NOT NULL,
            CHECK (state IN ('live', 'stale', 'blocked', 'replaced', 'killed')),
            CHECK (payload_contract IN ('framed-json', 'typed-wit', 'hybrid'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_buffers_state_created
        ON mother_view_buffers(state, created_at DESC);

        CREATE TABLE IF NOT EXISTS mother_view_frames (
            frame_id TEXT PRIMARY KEY,
            frame_kind TEXT NOT NULL,
            connected_at TEXT NOT NULL,
            CHECK (frame_kind IN ('sveltekit', 'tui', 'emacs', 'other'))
        );

        CREATE TABLE IF NOT EXISTS mother_view_windows (
            window_id TEXT PRIMARY KEY,
            frame_id TEXT NOT NULL,
            buffer_id TEXT,
            connection_state TEXT NOT NULL,
            connected_at TEXT,
            disconnected_at TEXT,
            CHECK (connection_state IN ('connected', 'disconnected'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_windows_frame
        ON mother_view_windows(frame_id, connection_state);

        CREATE TABLE IF NOT EXISTS mother_view_observability_gaps (
            gap_id TEXT PRIMARY KEY,
            shape_id TEXT,
            missing_fact_path TEXT NOT NULL,
            missing_source_id TEXT,
            reason TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            resolved_at TEXT,
            CHECK (status IN ('open', 'linked-to-work-item', 'resolved'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_observability_gaps_status_created
        ON mother_view_observability_gaps(status, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn save_buffer(conn: &Connection, buffer: &Buffer) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO mother_view_buffers (
            buffer_id, name, shape_id, state, created_at, stale_at, blocked_at,
            replaced_at, killed_at, major_mode, minor_modes_json, payload_contract,
            payload_version
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(buffer_id) DO UPDATE SET
            name = excluded.name,
            shape_id = excluded.shape_id,
            state = excluded.state,
            created_at = excluded.created_at,
            stale_at = excluded.stale_at,
            blocked_at = excluded.blocked_at,
            replaced_at = excluded.replaced_at,
            killed_at = excluded.killed_at,
            major_mode = excluded.major_mode,
            minor_modes_json = excluded.minor_modes_json,
            payload_contract = excluded.payload_contract,
            payload_version = excluded.payload_version
        "#,
        params![
            &buffer.buffer_id,
            &buffer.name,
            &buffer.shape_id,
            enum_to_db(&buffer.state)?,
            buffer.created_at.to_rfc3339(),
            opt_time_to_db(&buffer.stale_at),
            opt_time_to_db(&buffer.blocked_at),
            opt_time_to_db(&buffer.replaced_at),
            opt_time_to_db(&buffer.killed_at),
            enum_to_db(&buffer.major_mode)?,
            serde_json::to_string(&buffer.minor_modes)?,
            enum_to_db(&buffer.payload_contract)?,
            i64::from(buffer.payload_version),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_buffers(conn: &Connection) -> Result<Vec<Buffer>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT buffer_id, name, shape_id, state, created_at, stale_at, blocked_at,
               replaced_at, killed_at, major_mode, minor_modes_json, payload_contract,
               payload_version
        FROM mother_view_buffers
        ORDER BY created_at DESC, buffer_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], map_buffer_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn save_frame(conn: &Connection, frame: &Frame) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO mother_view_frames (frame_id, frame_kind, connected_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(frame_id) DO UPDATE SET
            frame_kind = excluded.frame_kind,
            connected_at = excluded.connected_at
        "#,
        params![
            &frame.frame_id,
            enum_to_db(&frame.frame_kind)?,
            frame.connected_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_frames(conn: &Connection) -> Result<Vec<Frame>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT frame_id, frame_kind, connected_at
        FROM mother_view_frames
        ORDER BY connected_at DESC, frame_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Frame {
                frame_id: row.get(0)?,
                frame_kind: enum_from_db::<FrameKind>(row.get::<_, String>(1)?, 1)?,
                connected_at: time_from_db(row.get::<_, String>(2)?, 2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn save_window(conn: &Connection, window: &Window) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO mother_view_windows (
            window_id, frame_id, buffer_id, connection_state, connected_at, disconnected_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(window_id) DO UPDATE SET
            frame_id = excluded.frame_id,
            buffer_id = excluded.buffer_id,
            connection_state = excluded.connection_state,
            connected_at = excluded.connected_at,
            disconnected_at = excluded.disconnected_at
        "#,
        params![
            &window.window_id,
            &window.frame_id,
            window.buffer_id.as_deref(),
            enum_to_db(&window.connection_state)?,
            opt_time_to_db(&window.connected_at),
            opt_time_to_db(&window.disconnected_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_windows(conn: &Connection) -> Result<Vec<Window>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT window_id, frame_id, buffer_id, connection_state, connected_at, disconnected_at
        FROM mother_view_windows
        ORDER BY window_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Window {
                window_id: row.get(0)?,
                frame_id: row.get(1)?,
                buffer_id: row.get(2)?,
                connection_state: enum_from_db::<WindowConnectionState>(
                    row.get::<_, String>(3)?,
                    3,
                )?,
                connected_at: opt_time_from_db(row.get::<_, Option<String>>(4)?, 4)?,
                disconnected_at: opt_time_from_db(row.get::<_, Option<String>>(5)?, 5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn save_gap(conn: &Connection, gap: &ObservabilityGap) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO mother_view_observability_gaps (
            gap_id, shape_id, missing_fact_path, missing_source_id, reason, status,
            created_at, resolved_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(gap_id) DO UPDATE SET
            shape_id = excluded.shape_id,
            missing_fact_path = excluded.missing_fact_path,
            missing_source_id = excluded.missing_source_id,
            reason = excluded.reason,
            status = excluded.status,
            created_at = excluded.created_at,
            resolved_at = excluded.resolved_at
        "#,
        params![
            &gap.gap_id,
            gap.shape_id.as_deref(),
            &gap.missing_fact_path,
            gap.missing_source_id.as_deref(),
            &gap.reason,
            enum_to_db(&gap.status)?,
            gap.created_at.to_rfc3339(),
            opt_time_to_db(&gap.resolved_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_gaps(conn: &Connection) -> Result<Vec<ObservabilityGap>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT gap_id, shape_id, missing_fact_path, missing_source_id, reason, status,
               created_at, resolved_at
        FROM mother_view_observability_gaps
        ORDER BY created_at DESC, gap_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ObservabilityGap {
                gap_id: row.get(0)?,
                shape_id: row.get(1)?,
                missing_fact_path: row.get(2)?,
                missing_source_id: row.get(3)?,
                reason: row.get(4)?,
                status: enum_from_db::<ObservabilityGapStatus>(row.get::<_, String>(5)?, 5)?,
                created_at: time_from_db(row.get::<_, String>(6)?, 6)?,
                resolved_at: opt_time_from_db(row.get::<_, Option<String>>(7)?, 7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn map_buffer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Buffer> {
    Ok(Buffer {
        buffer_id: row.get(0)?,
        name: row.get(1)?,
        shape_id: row.get(2)?,
        state: enum_from_db::<BufferState>(row.get::<_, String>(3)?, 3)?,
        created_at: time_from_db(row.get::<_, String>(4)?, 4)?,
        stale_at: opt_time_from_db(row.get::<_, Option<String>>(5)?, 5)?,
        blocked_at: opt_time_from_db(row.get::<_, Option<String>>(6)?, 6)?,
        replaced_at: opt_time_from_db(row.get::<_, Option<String>>(7)?, 7)?,
        killed_at: opt_time_from_db(row.get::<_, Option<String>>(8)?, 8)?,
        major_mode: enum_from_db::<MajorMode>(row.get::<_, String>(9)?, 9)?,
        minor_modes: json_from_db::<Vec<MinorMode>>(row.get::<_, String>(10)?, 10)?,
        payload_contract: enum_from_db::<PayloadContract>(row.get::<_, String>(11)?, 11)?,
        payload_version: u32_from_db(row.get::<_, i64>(12)?, 12)?,
    })
}

fn enum_to_db<T: Serialize>(value: &T) -> Result<String> {
    let value = serde_json::to_value(value)?;
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("enum serialized to non-string value"))
}

fn enum_from_db<T: DeserializeOwned>(value: String, column: usize) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|err| from_sql_error(column, err))
}

fn json_from_db<T: DeserializeOwned>(value: String, column: usize) -> rusqlite::Result<T> {
    serde_json::from_str(&value).map_err(|err| from_sql_error(column, err))
}

fn opt_time_to_db(value: &Option<DateTime<Utc>>) -> Option<String> {
    value.as_ref().map(DateTime::to_rfc3339)
}

fn time_from_db(value: String, column: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| from_sql_error(column, err))
}

fn opt_time_from_db(
    value: Option<String>,
    column: usize,
) -> rusqlite::Result<Option<DateTime<Utc>>> {
    value.map(|value| time_from_db(value, column)).transpose()
}

fn u32_from_db(value: i64, column: usize) -> rusqlite::Result<u32> {
    u32::try_from(value).map_err(|err| from_sql_error(column, err))
}

fn from_sql_error<E>(column: usize, err: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err))
}
