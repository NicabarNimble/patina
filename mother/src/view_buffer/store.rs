use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};

use super::{
    Buffer, BufferState, DisplayPattern, DisplayPatternKind, DisplayRequest, DisplayRequestOutcome,
    Frame, FrameKind, MajorMode, MinorMode, ObservabilityGap, ObservabilityGapStatus,
    ObservabilityImprovementArtifact, PayloadContract, ShapeMatch, ShapeMatchKind, ViewDerivation,
    ViewMaturationEvent, ViewMaturationOrigin, ViewMaturationTargetKind, ViewRequirement,
    ViewShape, ViewShapeAdaptation, ViewShapeCreation, ViewShapeMaturity, ViewShapeRevision,
    ViewShapeRevisionOrigin, ViewShapeRevisionState, ViewShapeScope, Window, WindowConnectionState,
};

pub(crate) fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS mother_view_display_requests (
            request_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            raw_request TEXT NOT NULL,
            requested_at TEXT NOT NULL,
            outcome TEXT NOT NULL,
            CHECK (LENGTH(TRIM(raw_request)) > 0),
            CHECK (outcome IN ('pending', 'buffer_opened', 'observability_gap_reported', 'unable'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_display_requests_outcome_requested
        ON mother_view_display_requests(outcome, requested_at DESC);

        CREATE TABLE IF NOT EXISTS mother_view_shape_matches (
            request_id TEXT PRIMARY KEY,
            shape_id TEXT,
            match_kind TEXT NOT NULL,
            confidence REAL NOT NULL,
            created_at TEXT NOT NULL,
            CHECK (match_kind IN ('exact', 'explicit_user_choice', 'similar', 'none')),
            CHECK (confidence >= 0.0 AND confidence <= 1.0),
            FOREIGN KEY (request_id) REFERENCES mother_view_display_requests(request_id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS mother_view_shape_adaptations (
            request_id TEXT PRIMARY KEY,
            precedent_shape_id TEXT NOT NULL,
            adapted_shape_id TEXT NOT NULL,
            opens_buffer INTEGER NOT NULL,
            request_outcome TEXT NOT NULL,
            created_at TEXT NOT NULL,
            CHECK (opens_buffer IN (0, 1)),
            CHECK (request_outcome IN ('pending', 'buffer_opened', 'observability_gap_reported', 'unable')),
            FOREIGN KEY (request_id) REFERENCES mother_view_display_requests(request_id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS mother_view_shape_creations (
            request_id TEXT PRIMARY KEY,
            created_shape_id TEXT NOT NULL,
            opens_buffer INTEGER NOT NULL,
            request_outcome TEXT NOT NULL,
            requirements_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            CHECK (opens_buffer IN (0, 1)),
            CHECK (request_outcome IN ('pending', 'buffer_opened', 'observability_gap_reported', 'unable')),
            FOREIGN KEY (request_id) REFERENCES mother_view_display_requests(request_id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS mother_view_shapes (
            shape_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            source_ref TEXT NOT NULL,
            scope TEXT NOT NULL,
            version INTEGER NOT NULL,
            active INTEGER NOT NULL,
            major_mode TEXT NOT NULL,
            minor_modes_json TEXT NOT NULL,
            maturity TEXT NOT NULL,
            payload_contract TEXT NOT NULL,
            payload_version INTEGER NOT NULL,
            vision_id TEXT,
            project_uid TEXT,
            replaced_by TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK (scope IN ('mother-user', 'vision', 'project', 'buffer-local')),
            CHECK (active IN (0, 1)),
            CHECK (major_mode IN ('table', 'list', 'graph', 'timeline', 'log', 'markdown', 'document', 'browser', 'image', 'artifact', 'custom')),
            CHECK (maturity IN ('exploratory', 'candidate', 'stable', 'promoted')),
            CHECK (payload_contract IN ('framed-json', 'typed-wit', 'hybrid'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_shapes_active_scope
        ON mother_view_shapes(active, scope, shape_id);

        CREATE TABLE IF NOT EXISTS mother_view_shape_requirements (
            shape_id TEXT NOT NULL,
            fact_path TEXT NOT NULL,
            required INTEGER NOT NULL,
            purpose TEXT NOT NULL,
            PRIMARY KEY (shape_id, fact_path),
            CHECK (required IN (0, 1)),
            FOREIGN KEY (shape_id) REFERENCES mother_view_shapes(shape_id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS mother_view_derivations (
            derivation_id TEXT PRIMARY KEY,
            shape_id TEXT NOT NULL,
            label TEXT NOT NULL,
            expression_ref TEXT NOT NULL,
            input_fact_paths_json TEXT NOT NULL,
            maturity TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK (LENGTH(TRIM(label)) > 0),
            CHECK (LENGTH(TRIM(expression_ref)) > 0),
            CHECK (maturity IN ('exploratory', 'candidate', 'stable', 'promoted'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_derivations_shape
        ON mother_view_derivations(shape_id, derivation_id);

        CREATE TABLE IF NOT EXISTS mother_view_display_patterns (
            pattern_id TEXT PRIMARY KEY,
            shape_id TEXT NOT NULL,
            pattern_kind TEXT NOT NULL,
            maturity TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK (pattern_kind IN ('grouping', 'sorting', 'filtering', 'highlighting', 'alerting', 'sectioning', 'mode_behavior')),
            CHECK (maturity IN ('exploratory', 'candidate', 'stable', 'promoted'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_display_patterns_shape
        ON mother_view_display_patterns(shape_id, pattern_id);

        CREATE TABLE IF NOT EXISTS mother_view_maturation_events (
            maturation_id TEXT PRIMARY KEY,
            target_kind TEXT NOT NULL,
            shape_id TEXT,
            derivation_id TEXT,
            pattern_id TEXT,
            origin TEXT NOT NULL,
            from_maturity TEXT NOT NULL,
            to_maturity TEXT NOT NULL,
            created_at TEXT NOT NULL,
            CHECK (target_kind IN ('shape', 'derivation', 'pattern')),
            CHECK (origin IN ('user_requested', 'mother_suggested', 'agent_inferred')),
            CHECK (from_maturity IN ('exploratory', 'candidate', 'stable')),
            CHECK (to_maturity IN ('candidate', 'stable', 'promoted'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_maturation_events_created
        ON mother_view_maturation_events(created_at DESC, maturation_id ASC);

        CREATE TABLE IF NOT EXISTS mother_view_observability_improvements (
            artifact_id TEXT PRIMARY KEY,
            source_gap_id TEXT,
            source_maturation_id TEXT,
            desired_fact_path TEXT NOT NULL,
            reason TEXT NOT NULL,
            created_at TEXT NOT NULL,
            work_item_created INTEGER NOT NULL,
            CHECK (LENGTH(TRIM(desired_fact_path)) > 0),
            CHECK (LENGTH(TRIM(reason)) > 0),
            CHECK (work_item_created IN (0, 1))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_observability_improvements_created
        ON mother_view_observability_improvements(created_at DESC, artifact_id ASC);

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
            replacement_buffer_id TEXT,
            major_mode TEXT NOT NULL,
            minor_modes_json TEXT NOT NULL,
            payload_contract TEXT NOT NULL,
            payload_version INTEGER NOT NULL,
            CHECK (state IN ('live', 'stale', 'blocked', 'replaced', 'killed')),
            CHECK (payload_contract IN ('framed-json', 'typed-wit', 'hybrid'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_buffers_state_created
        ON mother_view_buffers(state, created_at DESC);

        CREATE TABLE IF NOT EXISTS mother_view_shape_revisions (
            revision_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            previous_shape_id TEXT NOT NULL,
            revised_shape_id TEXT NOT NULL,
            previous_buffer_id TEXT,
            replacement_buffer_id TEXT,
            revision_scope TEXT NOT NULL,
            revision_origin TEXT NOT NULL,
            revision_state TEXT NOT NULL,
            reason TEXT NOT NULL,
            created_at TEXT NOT NULL,
            CHECK (revision_scope IN ('mother-user', 'vision', 'project', 'buffer-local')),
            CHECK (revision_origin IN ('user_correction', 'user_request', 'agent_adaptation')),
            CHECK (revision_state IN ('applied', 'proposed', 'rejected', 'reverted')),
            CHECK (LENGTH(TRIM(reason)) > 0)
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_shape_revisions_created
        ON mother_view_shape_revisions(created_at DESC);

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
            linked_work_item_id TEXT,
            created_at TEXT NOT NULL,
            resolved_at TEXT,
            CHECK (status IN ('open', 'linked-to-work-item', 'resolved'))
        );

        CREATE INDEX IF NOT EXISTS idx_mother_view_observability_gaps_status_created
        ON mother_view_observability_gaps(status, created_at DESC);
        "#,
    )?;
    ensure_column(conn, "mother_view_buffers", "replacement_buffer_id", "TEXT")?;
    ensure_column(
        conn,
        "mother_view_observability_gaps",
        "linked_work_item_id",
        "TEXT",
    )?;
    Ok(())
}

pub(crate) fn save_display_request(conn: &Connection, request: &DisplayRequest) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO mother_view_display_requests (
            request_id, user_id, agent_id, raw_request, requested_at, outcome
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(request_id) DO UPDATE SET
            user_id = excluded.user_id,
            agent_id = excluded.agent_id,
            raw_request = excluded.raw_request,
            requested_at = excluded.requested_at,
            outcome = excluded.outcome
        "#,
        params![
            &request.request_id,
            &request.user_id,
            &request.agent_id,
            &request.raw_request,
            request.requested_at.to_rfc3339(),
            enum_to_db(&request.outcome)?,
        ],
    )?;
    Ok(())
}

pub(crate) fn get_display_request(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<DisplayRequest>> {
    conn.query_row(
        r#"
        SELECT request_id, user_id, agent_id, raw_request, requested_at, outcome
        FROM mother_view_display_requests
        WHERE request_id = ?1
        "#,
        params![request_id],
        map_display_request_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_display_requests(conn: &Connection) -> Result<Vec<DisplayRequest>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT request_id, user_id, agent_id, raw_request, requested_at, outcome
        FROM mother_view_display_requests
        ORDER BY requested_at DESC, request_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], map_display_request_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn update_display_request_outcome(
    conn: &Connection,
    request_id: &str,
    outcome: &DisplayRequestOutcome,
) -> Result<bool> {
    let updated = conn.execute(
        r#"
        UPDATE mother_view_display_requests
        SET outcome = ?2
        WHERE request_id = ?1
        "#,
        params![request_id, enum_to_db(outcome)?],
    )?;
    Ok(updated > 0)
}

pub(crate) fn save_shape_match(conn: &Connection, shape_match: &ShapeMatch) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO mother_view_shape_matches (
            request_id, shape_id, match_kind, confidence, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(request_id) DO UPDATE SET
            shape_id = excluded.shape_id,
            match_kind = excluded.match_kind,
            confidence = excluded.confidence,
            created_at = excluded.created_at
        "#,
        params![
            &shape_match.request_id,
            shape_match.shape_id.as_deref(),
            enum_to_db(&shape_match.match_kind)?,
            shape_match.confidence,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn get_shape_match(conn: &Connection, request_id: &str) -> Result<Option<ShapeMatch>> {
    conn.query_row(
        r#"
        SELECT request_id, shape_id, match_kind, confidence
        FROM mother_view_shape_matches
        WHERE request_id = ?1
        "#,
        params![request_id],
        map_shape_match_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_shape_matches(conn: &Connection) -> Result<Vec<ShapeMatch>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT request_id, shape_id, match_kind, confidence
        FROM mother_view_shape_matches
        ORDER BY request_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], map_shape_match_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn save_shape_adaptation(
    conn: &Connection,
    adaptation: &ViewShapeAdaptation,
) -> Result<()> {
    // obligation: spec.mother-view-request-ux.mvru2-persist-request-artifacts
    conn.execute(
        r#"
        INSERT INTO mother_view_shape_adaptations (
            request_id, precedent_shape_id, adapted_shape_id, opens_buffer, request_outcome, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(request_id) DO UPDATE SET
            precedent_shape_id = excluded.precedent_shape_id,
            adapted_shape_id = excluded.adapted_shape_id,
            opens_buffer = excluded.opens_buffer,
            request_outcome = excluded.request_outcome
        "#,
        params![
            &adaptation.request_id,
            &adaptation.precedent_shape_id,
            &adaptation.adapted_shape_id,
            bool_to_db(adaptation.opens_buffer),
            enum_to_db(&adaptation.request_outcome)?,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn get_shape_adaptation(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<ViewShapeAdaptation>> {
    conn.query_row(
        r#"
        SELECT request_id, precedent_shape_id, adapted_shape_id, opens_buffer, request_outcome
        FROM mother_view_shape_adaptations
        WHERE request_id = ?1
        "#,
        params![request_id],
        map_shape_adaptation_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn save_shape_creation(conn: &Connection, creation: &ViewShapeCreation) -> Result<()> {
    // obligation: spec.mother-view-request-ux.mvru2-persist-request-artifacts
    conn.execute(
        r#"
        INSERT INTO mother_view_shape_creations (
            request_id, created_shape_id, opens_buffer, request_outcome, requirements_json, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(request_id) DO UPDATE SET
            created_shape_id = excluded.created_shape_id,
            opens_buffer = excluded.opens_buffer,
            request_outcome = excluded.request_outcome,
            requirements_json = excluded.requirements_json
        "#,
        params![
            &creation.request_id,
            &creation.created_shape_id,
            bool_to_db(creation.opens_buffer),
            enum_to_db(&creation.request_outcome)?,
            serde_json::to_string(&creation.requirements)?,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn get_shape_creation(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<ViewShapeCreation>> {
    conn.query_row(
        r#"
        SELECT request_id, created_shape_id, opens_buffer, request_outcome, requirements_json
        FROM mother_view_shape_creations
        WHERE request_id = ?1
        "#,
        params![request_id],
        map_shape_creation_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn save_shape_revision(conn: &Connection, revision: &ViewShapeRevision) -> Result<()> {
    // obligation: spec.mother-view-buffer-revision.mvbr5-persistence
    conn.execute(
        r#"
        INSERT INTO mother_view_shape_revisions (
            revision_id, user_id, agent_id, previous_shape_id, revised_shape_id,
            previous_buffer_id, replacement_buffer_id, revision_scope, revision_origin,
            revision_state, reason, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(revision_id) DO UPDATE SET
            replacement_buffer_id = excluded.replacement_buffer_id,
            revision_state = excluded.revision_state
        "#,
        params![
            &revision.revision_id,
            &revision.user_id,
            &revision.agent_id,
            &revision.previous_shape_id,
            &revision.revised_shape_id,
            revision.previous_buffer_id.as_deref(),
            revision.replacement_buffer_id.as_deref(),
            enum_to_db(&revision.revision_scope)?,
            enum_to_db(&revision.revision_origin)?,
            enum_to_db(&revision.revision_state)?,
            &revision.reason,
            revision.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn get_shape_revision(
    conn: &Connection,
    revision_id: &str,
) -> Result<Option<ViewShapeRevision>> {
    conn.query_row(
        r#"
        SELECT revision_id, user_id, agent_id, previous_shape_id, revised_shape_id,
               previous_buffer_id, replacement_buffer_id, revision_scope, revision_origin,
               revision_state, reason, created_at
        FROM mother_view_shape_revisions
        WHERE revision_id = ?1
        "#,
        params![revision_id],
        map_shape_revision_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_shape_revisions(conn: &Connection) -> Result<Vec<ViewShapeRevision>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT revision_id, user_id, agent_id, previous_shape_id, revised_shape_id,
               previous_buffer_id, replacement_buffer_id, revision_scope, revision_origin,
               revision_state, reason, created_at
        FROM mother_view_shape_revisions
        ORDER BY created_at DESC, revision_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], map_shape_revision_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn upsert_shape(conn: &Connection, shape: &ViewShape) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO mother_view_shapes (
            shape_id, title, source_ref, scope, version, active, major_mode,
            minor_modes_json, maturity, payload_contract, payload_version,
            vision_id, project_uid, replaced_by, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ON CONFLICT(shape_id) DO UPDATE SET
            title = excluded.title,
            source_ref = excluded.source_ref,
            scope = excluded.scope,
            version = excluded.version,
            active = excluded.active,
            major_mode = excluded.major_mode,
            minor_modes_json = excluded.minor_modes_json,
            maturity = excluded.maturity,
            payload_contract = excluded.payload_contract,
            payload_version = excluded.payload_version,
            vision_id = excluded.vision_id,
            project_uid = excluded.project_uid,
            replaced_by = excluded.replaced_by,
            updated_at = excluded.updated_at
        "#,
        params![
            &shape.shape_id,
            &shape.title,
            &shape.source_ref,
            enum_to_db(&shape.scope)?,
            i64::from(shape.version),
            bool_to_db(shape.active),
            enum_to_db(&shape.major_mode)?,
            serde_json::to_string(&shape.minor_modes)?,
            enum_to_db(&shape.maturity)?,
            enum_to_db(&shape.payload_contract)?,
            i64::from(shape.payload_version),
            shape.vision_id.as_deref(),
            shape.project_uid.as_deref(),
            shape.replaced_by.as_deref(),
            &now,
            &now,
        ],
    )?;
    conn.execute(
        "DELETE FROM mother_view_shape_requirements WHERE shape_id = ?1",
        params![&shape.shape_id],
    )?;
    for requirement in &shape.requirements {
        conn.execute(
            r#"
            INSERT INTO mother_view_shape_requirements (shape_id, fact_path, required, purpose)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                &shape.shape_id,
                &requirement.fact_path,
                bool_to_db(requirement.required),
                &requirement.purpose,
            ],
        )?;
    }
    Ok(())
}

pub(crate) fn get_shape(conn: &Connection, shape_id: &str) -> Result<Option<ViewShape>> {
    let shape = conn
        .query_row(
            r#"
            SELECT shape_id, title, source_ref, scope, version, active, major_mode,
                   minor_modes_json, maturity, payload_contract, payload_version,
                   vision_id, project_uid, replaced_by
            FROM mother_view_shapes
            WHERE shape_id = ?1
            "#,
            params![shape_id],
            map_shape_row,
        )
        .optional()?;

    shape
        .map(|mut shape| {
            shape.requirements = list_shape_requirements(conn, &shape.shape_id)?;
            Ok(shape)
        })
        .transpose()
}

pub(crate) fn list_shapes(conn: &Connection) -> Result<Vec<ViewShape>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT shape_id, title, source_ref, scope, version, active, major_mode,
               minor_modes_json, maturity, payload_contract, payload_version,
               vision_id, project_uid, replaced_by
        FROM mother_view_shapes
        ORDER BY shape_id ASC
        "#,
    )?;
    let mut shapes = stmt
        .query_map([], map_shape_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for shape in &mut shapes {
        shape.requirements = list_shape_requirements(conn, &shape.shape_id)?;
    }
    Ok(shapes)
}

pub(crate) fn deactivate_shape(conn: &Connection, shape_id: &str) -> Result<bool> {
    let updated = conn.execute(
        r#"
        UPDATE mother_view_shapes
        SET active = 0, updated_at = ?2
        WHERE shape_id = ?1
        "#,
        params![shape_id, Utc::now().to_rfc3339()],
    )?;
    Ok(updated > 0)
}

pub(crate) fn upsert_derivation(conn: &Connection, derivation: &ViewDerivation) -> Result<()> {
    // obligation: spec.mother-view-maturation.mvmat2-artifact-library
    let now = Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO mother_view_derivations (
            derivation_id, shape_id, label, expression_ref, input_fact_paths_json,
            maturity, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(derivation_id) DO UPDATE SET
            shape_id = excluded.shape_id,
            label = excluded.label,
            expression_ref = excluded.expression_ref,
            input_fact_paths_json = excluded.input_fact_paths_json,
            maturity = excluded.maturity,
            updated_at = excluded.updated_at
        "#,
        params![
            &derivation.derivation_id,
            &derivation.shape_id,
            &derivation.label,
            &derivation.expression_ref,
            serde_json::to_string(&derivation.input_fact_paths)?,
            enum_to_db(&derivation.maturity)?,
            &now,
            &now,
        ],
    )?;
    Ok(())
}

pub(crate) fn get_derivation(
    conn: &Connection,
    derivation_id: &str,
) -> Result<Option<ViewDerivation>> {
    conn.query_row(
        r#"
        SELECT derivation_id, shape_id, label, expression_ref, input_fact_paths_json, maturity
        FROM mother_view_derivations
        WHERE derivation_id = ?1
        "#,
        params![derivation_id],
        map_derivation_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_derivations(conn: &Connection) -> Result<Vec<ViewDerivation>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT derivation_id, shape_id, label, expression_ref, input_fact_paths_json, maturity
        FROM mother_view_derivations
        ORDER BY derivation_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], map_derivation_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn upsert_display_pattern(conn: &Connection, pattern: &DisplayPattern) -> Result<()> {
    // obligation: spec.mother-view-maturation.mvmat2-artifact-library
    let now = Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO mother_view_display_patterns (
            pattern_id, shape_id, pattern_kind, maturity, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(pattern_id) DO UPDATE SET
            shape_id = excluded.shape_id,
            pattern_kind = excluded.pattern_kind,
            maturity = excluded.maturity,
            updated_at = excluded.updated_at
        "#,
        params![
            &pattern.pattern_id,
            &pattern.shape_id,
            enum_to_db(&pattern.pattern_kind)?,
            enum_to_db(&pattern.maturity)?,
            &now,
            &now,
        ],
    )?;
    Ok(())
}

pub(crate) fn get_display_pattern(
    conn: &Connection,
    pattern_id: &str,
) -> Result<Option<DisplayPattern>> {
    conn.query_row(
        r#"
        SELECT pattern_id, shape_id, pattern_kind, maturity
        FROM mother_view_display_patterns
        WHERE pattern_id = ?1
        "#,
        params![pattern_id],
        map_display_pattern_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_display_patterns(conn: &Connection) -> Result<Vec<DisplayPattern>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT pattern_id, shape_id, pattern_kind, maturity
        FROM mother_view_display_patterns
        ORDER BY pattern_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], map_display_pattern_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn save_maturation_event(conn: &Connection, event: &ViewMaturationEvent) -> Result<()> {
    // obligation: spec.mother-view-maturation.mvmat3-shape-maturation
    // obligation: spec.mother-view-maturation.mvmat4-derivation-pattern-maturation
    conn.execute(
        r#"
        INSERT INTO mother_view_maturation_events (
            maturation_id, target_kind, shape_id, derivation_id, pattern_id, origin,
            from_maturity, to_maturity, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(maturation_id) DO NOTHING
        "#,
        params![
            &event.maturation_id,
            enum_to_db(&event.target_kind)?,
            event.shape_id.as_deref(),
            event.derivation_id.as_deref(),
            event.pattern_id.as_deref(),
            enum_to_db(&event.origin)?,
            enum_to_db(&event.from_maturity)?,
            enum_to_db(&event.to_maturity)?,
            event.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn get_maturation_event(
    conn: &Connection,
    maturation_id: &str,
) -> Result<Option<ViewMaturationEvent>> {
    conn.query_row(
        r#"
        SELECT maturation_id, target_kind, shape_id, derivation_id, pattern_id, origin,
               from_maturity, to_maturity, created_at
        FROM mother_view_maturation_events
        WHERE maturation_id = ?1
        "#,
        params![maturation_id],
        map_maturation_event_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_maturation_events(conn: &Connection) -> Result<Vec<ViewMaturationEvent>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT maturation_id, target_kind, shape_id, derivation_id, pattern_id, origin,
               from_maturity, to_maturity, created_at
        FROM mother_view_maturation_events
        ORDER BY created_at DESC, maturation_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], map_maturation_event_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn save_observability_improvement(
    conn: &Connection,
    artifact: &ObservabilityImprovementArtifact,
) -> Result<()> {
    // obligation: spec.mother-view-maturation.mvmat5-observability-improvement-artifact
    conn.execute(
        r#"
        INSERT INTO mother_view_observability_improvements (
            artifact_id, source_gap_id, source_maturation_id, desired_fact_path,
            reason, created_at, work_item_created
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(artifact_id) DO UPDATE SET
            source_gap_id = excluded.source_gap_id,
            source_maturation_id = excluded.source_maturation_id,
            desired_fact_path = excluded.desired_fact_path,
            reason = excluded.reason,
            work_item_created = excluded.work_item_created
        "#,
        params![
            &artifact.artifact_id,
            artifact.source_gap_id.as_deref(),
            artifact.source_maturation_id.as_deref(),
            &artifact.desired_fact_path,
            &artifact.reason,
            artifact.created_at.to_rfc3339(),
            bool_to_db(artifact.work_item_created),
        ],
    )?;
    Ok(())
}

pub(crate) fn get_observability_improvement(
    conn: &Connection,
    artifact_id: &str,
) -> Result<Option<ObservabilityImprovementArtifact>> {
    conn.query_row(
        r#"
        SELECT artifact_id, source_gap_id, source_maturation_id, desired_fact_path,
               reason, created_at, work_item_created
        FROM mother_view_observability_improvements
        WHERE artifact_id = ?1
        "#,
        params![artifact_id],
        map_observability_improvement_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_observability_improvements(
    conn: &Connection,
) -> Result<Vec<ObservabilityImprovementArtifact>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT artifact_id, source_gap_id, source_maturation_id, desired_fact_path,
               reason, created_at, work_item_created
        FROM mother_view_observability_improvements
        ORDER BY created_at DESC, artifact_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], map_observability_improvement_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn save_buffer(conn: &Connection, buffer: &Buffer) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO mother_view_buffers (
            buffer_id, name, shape_id, state, created_at, stale_at, blocked_at,
            replaced_at, killed_at, replacement_buffer_id, major_mode, minor_modes_json,
            payload_contract, payload_version
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(buffer_id) DO UPDATE SET
            name = excluded.name,
            shape_id = excluded.shape_id,
            state = excluded.state,
            created_at = excluded.created_at,
            stale_at = excluded.stale_at,
            blocked_at = excluded.blocked_at,
            replaced_at = excluded.replaced_at,
            killed_at = excluded.killed_at,
            replacement_buffer_id = excluded.replacement_buffer_id,
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
            buffer.replacement_buffer_id.as_deref(),
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
               replaced_at, killed_at, replacement_buffer_id, major_mode, minor_modes_json,
               payload_contract, payload_version
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
            linked_work_item_id, created_at, resolved_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(gap_id) DO UPDATE SET
            shape_id = excluded.shape_id,
            missing_fact_path = excluded.missing_fact_path,
            missing_source_id = excluded.missing_source_id,
            reason = excluded.reason,
            status = excluded.status,
            linked_work_item_id = excluded.linked_work_item_id,
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
            gap.linked_work_item_id.as_deref(),
            gap.created_at.to_rfc3339(),
            opt_time_to_db(&gap.resolved_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn get_gap(conn: &Connection, gap_id: &str) -> Result<Option<ObservabilityGap>> {
    conn.query_row(
        r#"
        SELECT gap_id, shape_id, missing_fact_path, missing_source_id, reason, status,
               linked_work_item_id, created_at, resolved_at
        FROM mother_view_observability_gaps
        WHERE gap_id = ?1
        "#,
        params![gap_id],
        |row| {
            Ok(ObservabilityGap {
                gap_id: row.get(0)?,
                shape_id: row.get(1)?,
                missing_fact_path: row.get(2)?,
                missing_source_id: row.get(3)?,
                reason: row.get(4)?,
                status: enum_from_db::<ObservabilityGapStatus>(row.get::<_, String>(5)?, 5)?,
                linked_work_item_id: row.get(6)?,
                created_at: time_from_db(row.get::<_, String>(7)?, 7)?,
                resolved_at: opt_time_from_db(row.get::<_, Option<String>>(8)?, 8)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_gaps(conn: &Connection) -> Result<Vec<ObservabilityGap>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT gap_id, shape_id, missing_fact_path, missing_source_id, reason, status,
               linked_work_item_id, created_at, resolved_at
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
                linked_work_item_id: row.get(6)?,
                created_at: time_from_db(row.get::<_, String>(7)?, 7)?,
                resolved_at: opt_time_from_db(row.get::<_, Option<String>>(8)?, 8)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn map_display_request_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DisplayRequest> {
    Ok(DisplayRequest {
        request_id: row.get(0)?,
        user_id: row.get(1)?,
        agent_id: row.get(2)?,
        raw_request: row.get(3)?,
        requested_at: time_from_db(row.get::<_, String>(4)?, 4)?,
        outcome: enum_from_db::<DisplayRequestOutcome>(row.get::<_, String>(5)?, 5)?,
    })
}

fn map_shape_match_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ShapeMatch> {
    Ok(ShapeMatch {
        request_id: row.get(0)?,
        shape_id: row.get(1)?,
        match_kind: enum_from_db::<ShapeMatchKind>(row.get::<_, String>(2)?, 2)?,
        confidence: row.get(3)?,
    })
}

fn map_shape_adaptation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ViewShapeAdaptation> {
    Ok(ViewShapeAdaptation {
        request_id: row.get(0)?,
        precedent_shape_id: row.get(1)?,
        adapted_shape_id: row.get(2)?,
        opens_buffer: bool_from_db(row.get::<_, i64>(3)?, 3)?,
        request_outcome: enum_from_db::<DisplayRequestOutcome>(row.get::<_, String>(4)?, 4)?,
    })
}

fn map_shape_creation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ViewShapeCreation> {
    Ok(ViewShapeCreation {
        request_id: row.get(0)?,
        created_shape_id: row.get(1)?,
        opens_buffer: bool_from_db(row.get::<_, i64>(2)?, 2)?,
        request_outcome: enum_from_db::<DisplayRequestOutcome>(row.get::<_, String>(3)?, 3)?,
        requirements: json_from_db::<Vec<ViewRequirement>>(row.get::<_, String>(4)?, 4)?,
    })
}

fn map_shape_revision_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ViewShapeRevision> {
    Ok(ViewShapeRevision {
        revision_id: row.get(0)?,
        user_id: row.get(1)?,
        agent_id: row.get(2)?,
        previous_shape_id: row.get(3)?,
        revised_shape_id: row.get(4)?,
        previous_buffer_id: row.get(5)?,
        replacement_buffer_id: row.get(6)?,
        revision_scope: enum_from_db::<ViewShapeScope>(row.get::<_, String>(7)?, 7)?,
        revision_origin: enum_from_db::<ViewShapeRevisionOrigin>(row.get::<_, String>(8)?, 8)?,
        revision_state: enum_from_db::<ViewShapeRevisionState>(row.get::<_, String>(9)?, 9)?,
        reason: row.get(10)?,
        created_at: time_from_db(row.get::<_, String>(11)?, 11)?,
    })
}

fn map_shape_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ViewShape> {
    Ok(ViewShape {
        shape_id: row.get(0)?,
        title: row.get(1)?,
        source_ref: row.get(2)?,
        scope: enum_from_db::<ViewShapeScope>(row.get::<_, String>(3)?, 3)?,
        version: u32_from_db(row.get::<_, i64>(4)?, 4)?,
        active: bool_from_db(row.get::<_, i64>(5)?, 5)?,
        major_mode: enum_from_db::<MajorMode>(row.get::<_, String>(6)?, 6)?,
        minor_modes: json_from_db::<Vec<MinorMode>>(row.get::<_, String>(7)?, 7)?,
        maturity: enum_from_db::<ViewShapeMaturity>(row.get::<_, String>(8)?, 8)?,
        payload_contract: enum_from_db::<PayloadContract>(row.get::<_, String>(9)?, 9)?,
        payload_version: u32_from_db(row.get::<_, i64>(10)?, 10)?,
        vision_id: row.get(11)?,
        project_uid: row.get(12)?,
        replaced_by: row.get(13)?,
        requirements: vec![],
    })
}

fn map_derivation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ViewDerivation> {
    Ok(ViewDerivation {
        derivation_id: row.get(0)?,
        shape_id: row.get(1)?,
        label: row.get(2)?,
        expression_ref: row.get(3)?,
        input_fact_paths: json_from_db::<Vec<String>>(row.get::<_, String>(4)?, 4)?,
        maturity: enum_from_db::<ViewShapeMaturity>(row.get::<_, String>(5)?, 5)?,
    })
}

fn map_display_pattern_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DisplayPattern> {
    Ok(DisplayPattern {
        pattern_id: row.get(0)?,
        shape_id: row.get(1)?,
        pattern_kind: enum_from_db::<DisplayPatternKind>(row.get::<_, String>(2)?, 2)?,
        maturity: enum_from_db::<ViewShapeMaturity>(row.get::<_, String>(3)?, 3)?,
    })
}

fn map_maturation_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ViewMaturationEvent> {
    Ok(ViewMaturationEvent {
        maturation_id: row.get(0)?,
        target_kind: enum_from_db::<ViewMaturationTargetKind>(row.get::<_, String>(1)?, 1)?,
        shape_id: row.get(2)?,
        derivation_id: row.get(3)?,
        pattern_id: row.get(4)?,
        origin: enum_from_db::<ViewMaturationOrigin>(row.get::<_, String>(5)?, 5)?,
        from_maturity: enum_from_db::<ViewShapeMaturity>(row.get::<_, String>(6)?, 6)?,
        to_maturity: enum_from_db::<ViewShapeMaturity>(row.get::<_, String>(7)?, 7)?,
        created_at: time_from_db(row.get::<_, String>(8)?, 8)?,
    })
}

fn map_observability_improvement_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ObservabilityImprovementArtifact> {
    Ok(ObservabilityImprovementArtifact {
        artifact_id: row.get(0)?,
        source_gap_id: row.get(1)?,
        source_maturation_id: row.get(2)?,
        desired_fact_path: row.get(3)?,
        reason: row.get(4)?,
        created_at: time_from_db(row.get::<_, String>(5)?, 5)?,
        work_item_created: bool_from_db(row.get::<_, i64>(6)?, 6)?,
    })
}

fn list_shape_requirements(
    conn: &Connection,
    shape_id: &str,
) -> rusqlite::Result<Vec<ViewRequirement>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT fact_path, required, purpose
        FROM mother_view_shape_requirements
        WHERE shape_id = ?1
        ORDER BY fact_path ASC
        "#,
    )?;
    let rows = stmt
        .query_map(params![shape_id], |row| {
            Ok(ViewRequirement {
                fact_path: row.get(0)?,
                required: bool_from_db(row.get::<_, i64>(1)?, 1)?,
                purpose: row.get(2)?,
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
        replacement_buffer_id: row.get(9)?,
        major_mode: enum_from_db::<MajorMode>(row.get::<_, String>(10)?, 10)?,
        minor_modes: json_from_db::<Vec<MinorMode>>(row.get::<_, String>(11)?, 11)?,
        payload_contract: enum_from_db::<PayloadContract>(row.get::<_, String>(12)?, 12)?,
        payload_version: u32_from_db(row.get::<_, i64>(13)?, 13)?,
    })
}

fn ensure_column(conn: &Connection, table: &str, column: &str, sql_type: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma)?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !names.iter().any(|name| name == column) {
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, sql_type),
            [],
        )?;
    }
    Ok(())
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

fn bool_to_db(value: bool) -> i64 {
    i64::from(value)
}

fn bool_from_db(value: i64, column: usize) -> rusqlite::Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(from_sql_error(
            column,
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected SQLite boolean 0 or 1, got {}", value),
            ),
        )),
    }
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
