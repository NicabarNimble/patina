use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    Buffer, BufferState, DisplayRequest, DisplayRequestOutcome, Frame, FrameKind,
    FramedJsonPayload, MajorMode, MinorMode, ObservabilityGap, PayloadContract, ShapeMatch,
    ShapeMatchKind, ViewRequirement, ViewShape, ViewShapeMaturity, ViewShapeScope, Window,
    WindowConnectionState,
};
use crate::view_buffer::catalog::{DataCatalog, MOTHER_STATUS_SHAPE_ID, MOTHER_STATUS_SOURCE_ID};

pub const SHAPE_MATCH_CONFIDENCE_THRESHOLD: f64 = 0.60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenBufferRequest {
    pub shape_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedShapeMatch {
    pub shape_id: Option<String>,
    pub match_kind: ShapeMatchKind,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeViewRequest {
    pub user_id: String,
    pub agent_id: String,
    pub raw_request: String,
    pub proposed_match: Option<ProposedShapeMatch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComposedViewRequest {
    pub request: DisplayRequest,
    pub shape_match: Option<ShapeMatch>,
    pub open_outcome: Option<OpenBufferOutcome>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectWindowRequest {
    pub frame_id: String,
    pub frame_kind: FrameKind,
    pub window_id: String,
    pub buffer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisconnectWindowRequest {
    pub window_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillBufferRequest {
    pub buffer_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenedBuffer {
    pub buffer: Buffer,
    pub payload: FramedJsonPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum OpenBufferOutcome {
    Opened(OpenedBuffer),
    ObservabilityGap(ObservabilityGap),
}

#[derive(Debug, Clone)]
pub struct ViewBufferService {
    catalog: DataCatalog,
    shapes: BTreeMap<String, ViewShape>,
    buffers: BTreeMap<String, Buffer>,
    frames: BTreeMap<String, Frame>,
    windows: BTreeMap<String, Window>,
    gaps: BTreeMap<String, ObservabilityGap>,
}

impl ViewBufferService {
    pub fn with_catalog(catalog: DataCatalog) -> Self {
        Self::with_catalog_and_shapes(catalog, vec![mother_status_shape()])
    }

    pub fn with_catalog_and_shapes(
        catalog: DataCatalog,
        shapes: impl IntoIterator<Item = ViewShape>,
    ) -> Self {
        Self {
            catalog,
            shapes: shapes
                .into_iter()
                .map(|shape| (shape.shape_id.clone(), shape))
                .collect(),
            buffers: BTreeMap::new(),
            frames: BTreeMap::new(),
            windows: BTreeMap::new(),
            gaps: BTreeMap::new(),
        }
    }

    pub fn list_buffers(&self) -> Vec<Buffer> {
        self.buffers.values().cloned().collect()
    }

    pub fn list_frames(&self) -> Vec<Frame> {
        self.frames.values().cloned().collect()
    }

    pub fn list_windows(&self) -> Vec<Window> {
        self.windows.values().cloned().collect()
    }

    pub fn list_gaps(&self) -> Vec<ObservabilityGap> {
        self.gaps.values().cloned().collect()
    }

    pub fn compose_request(&mut self, request: ComposeViewRequest) -> Result<ComposedViewRequest> {
        if request.raw_request.trim().is_empty() {
            return Err(anyhow!("raw display request must not be empty"));
        }

        let mut display_request = DisplayRequest::pending(
            format!("req_{}", uuid::Uuid::new_v4().simple()),
            request.user_id,
            request.agent_id,
            request.raw_request,
            Utc::now(),
        );

        let Some(proposed_match) = request.proposed_match else {
            display_request.outcome = DisplayRequestOutcome::Unable;
            return Ok(ComposedViewRequest {
                request: display_request,
                shape_match: None,
                open_outcome: None,
                reason: Some("no shape match proposed".to_string()),
            });
        };

        let shape_match = ShapeMatch {
            request_id: display_request.request_id.clone(),
            shape_id: proposed_match.shape_id.clone(),
            match_kind: proposed_match.match_kind,
            confidence: proposed_match.confidence,
        };

        let should_open = match self.validate_openable_match(&shape_match) {
            Ok(()) => true,
            Err(reason) => {
                display_request.outcome = DisplayRequestOutcome::Unable;
                return Ok(ComposedViewRequest {
                    request: display_request,
                    shape_match: Some(shape_match),
                    open_outcome: None,
                    reason: Some(reason),
                });
            }
        };

        if should_open {
            let shape_id = shape_match
                .shape_id
                .clone()
                .expect("validated openable match should have shape id");
            let open_outcome = self.open_buffer(OpenBufferRequest { shape_id })?;
            display_request.outcome = match open_outcome {
                OpenBufferOutcome::Opened(_) => DisplayRequestOutcome::BufferOpened,
                OpenBufferOutcome::ObservabilityGap(_) => {
                    DisplayRequestOutcome::ObservabilityGapReported
                }
            };
            return Ok(ComposedViewRequest {
                request: display_request,
                shape_match: Some(shape_match),
                open_outcome: Some(open_outcome),
                reason: None,
            });
        }

        unreachable!("validated openable match should return above")
    }

    fn validate_openable_match(&self, shape_match: &ShapeMatch) -> std::result::Result<(), String> {
        match shape_match.match_kind {
            ShapeMatchKind::ExplicitUserChoice => self.validate_active_shape(shape_match),
            ShapeMatchKind::Exact => {
                if shape_match.confidence < SHAPE_MATCH_CONFIDENCE_THRESHOLD {
                    return Err(format!(
                        "exact shape match confidence {:.2} below threshold {:.2}",
                        shape_match.confidence, SHAPE_MATCH_CONFIDENCE_THRESHOLD
                    ));
                }
                self.validate_active_shape(shape_match)
            }
            ShapeMatchKind::Similar => Err("similar shape adaptation is deferred".to_string()),
            ShapeMatchKind::None => Err("no usable shape matched request".to_string()),
        }
    }

    fn validate_active_shape(&self, shape_match: &ShapeMatch) -> std::result::Result<(), String> {
        let shape_id = shape_match
            .shape_id
            .as_deref()
            .ok_or_else(|| "shape match is missing shape_id".to_string())?;
        let shape = self
            .shapes
            .get(shape_id)
            .ok_or_else(|| format!("unknown view shape '{}'", shape_id))?;
        if !shape.active {
            return Err(format!("inactive view shape '{}'", shape_id));
        }
        Ok(())
    }

    pub fn open_buffer(&mut self, request: OpenBufferRequest) -> Result<OpenBufferOutcome> {
        let shape = self
            .shapes
            .get(&request.shape_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown view shape '{}'", request.shape_id))?;
        if !shape.active {
            return Err(anyhow!("inactive view shape '{}'", request.shape_id));
        }

        if let Some(missing) = shape
            .requirements
            .iter()
            .find(|requirement| !self.catalog.observed_required_fact(requirement))
        {
            let gap = self.record_gap(&shape, missing, Utc::now());
            return Ok(OpenBufferOutcome::ObservabilityGap(gap));
        }

        let buffer_id = self.next_buffer_id(&shape);
        let buffer = Buffer::live_from_shape(buffer_id, &shape, Utc::now());
        let payload = FramedJsonPayload::new(&buffer, &shape, self.payload_json_for_shape(&shape));
        self.buffers
            .insert(buffer.buffer_id.clone(), buffer.clone());
        Ok(OpenBufferOutcome::Opened(OpenedBuffer { buffer, payload }))
    }

    pub fn connect_window(&mut self, request: ConnectWindowRequest) -> Result<Window> {
        let buffer = self
            .buffers
            .get(&request.buffer_id)
            .ok_or_else(|| anyhow!("unknown view buffer '{}'", request.buffer_id))?;
        if !buffer.state.is_connectable() {
            return Err(anyhow!(
                "view buffer '{}' is not connectable in state {:?}",
                buffer.buffer_id,
                buffer.state
            ));
        }

        let now = Utc::now();
        let frame = Frame {
            frame_id: request.frame_id,
            frame_kind: request.frame_kind,
            connected_at: now,
        };
        let window = Window {
            window_id: request.window_id,
            frame_id: frame.frame_id.clone(),
            buffer_id: Some(buffer.buffer_id.clone()),
            connection_state: WindowConnectionState::Connected,
            connected_at: Some(now),
            disconnected_at: None,
        };
        self.frames.insert(frame.frame_id.clone(), frame);
        self.windows
            .insert(window.window_id.clone(), window.clone());
        Ok(window)
    }

    pub fn disconnect_window(&mut self, request: DisconnectWindowRequest) -> Result<Window> {
        let window = self
            .windows
            .get_mut(&request.window_id)
            .ok_or_else(|| anyhow!("unknown view window '{}'", request.window_id))?;
        if window.connection_state != WindowConnectionState::Connected {
            return Err(anyhow!(
                "view window '{}' is not connected",
                request.window_id
            ));
        }
        window.connection_state = WindowConnectionState::Disconnected;
        window.disconnected_at = Some(Utc::now());
        Ok(window.clone())
    }

    pub fn kill_buffer(&mut self, request: KillBufferRequest) -> Result<Buffer> {
        let buffer = self
            .buffers
            .get_mut(&request.buffer_id)
            .ok_or_else(|| anyhow!("unknown view buffer '{}'", request.buffer_id))?;
        if !buffer.state.is_connectable() {
            return Err(anyhow!(
                "view buffer '{}' cannot be killed from state {:?}",
                request.buffer_id,
                buffer.state
            ));
        }
        buffer.state = BufferState::Killed;
        buffer.killed_at = Some(Utc::now());
        Ok(buffer.clone())
    }

    fn record_gap(
        &mut self,
        shape: &ViewShape,
        missing: &ViewRequirement,
        created_at: DateTime<Utc>,
    ) -> ObservabilityGap {
        let gap_id = format!(
            "gap_{}_{}",
            sanitize_id(&missing.fact_path),
            uuid::Uuid::new_v4().simple()
        );
        let missing_source_id = self
            .catalog
            .fact(&missing.fact_path)
            .map(|fact| fact.source_id.clone())
            .or_else(|| Some(MOTHER_STATUS_SOURCE_ID.to_string()));
        let gap = ObservabilityGap::open(
            gap_id,
            Some(shape.shape_id.clone()),
            missing.fact_path.clone(),
            missing_source_id,
            format!("required fact '{}' is not observed", missing.fact_path),
            created_at,
        );
        self.gaps.insert(gap.gap_id.clone(), gap.clone());
        gap
    }

    fn next_buffer_id(&mut self, shape: &ViewShape) -> String {
        format!(
            "buf_{}_{}",
            sanitize_id(&shape.shape_id),
            uuid::Uuid::new_v4().simple()
        )
    }

    fn payload_json_for_shape(&self, shape: &ViewShape) -> serde_json::Value {
        let rows: Vec<_> = shape
            .requirements
            .iter()
            .filter(|requirement| requirement.required)
            .filter_map(|requirement| {
                self.catalog.value(&requirement.fact_path).map(|value| {
                    json!({
                        "fact_path": requirement.fact_path,
                        "purpose": requirement.purpose,
                        "value": value,
                    })
                })
            })
            .collect();

        json!({
            "major_mode": shape.major_mode,
            "minor_modes": shape.minor_modes,
            "columns": ["fact_path", "purpose", "value"],
            "rows": rows,
        })
    }
}

pub fn mother_status_shape() -> ViewShape {
    ViewShape {
        shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
        title: "Mother Status".to_string(),
        source_ref: "local-allium-view-library".to_string(),
        scope: ViewShapeScope::MotherUser,
        version: 1,
        active: true,
        major_mode: MajorMode::Table,
        minor_modes: vec![MinorMode::Pinned],
        maturity: ViewShapeMaturity::Stable,
        payload_contract: PayloadContract::FramedJson,
        payload_version: 1,
        vision_id: None,
        project_uid: None,
        replaced_by: None,
        requirements: vec![
            required("mother.status.version", "display Mother binary version"),
            required(
                "mother.status.control_plane_ready",
                "display control-plane readiness",
            ),
            required(
                "mother.status.registered_projects",
                "display registered project count",
            ),
            required(
                "mother.status.children_ready_count",
                "display ready child count",
            ),
            required("mother.status.children_total", "display total child count"),
        ],
    }
}

fn required(fact_path: &str, purpose: &str) -> ViewRequirement {
    ViewRequirement {
        fact_path: fact_path.to_string(),
        required: true,
        purpose: purpose.to_string(),
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::view_buffer::{
        BufferState, FrameKind, MotherStatusFacts, ObservabilityGapStatus, SourceAvailability,
        WindowConnectionState,
    };

    fn status_catalog() -> DataCatalog {
        DataCatalog::mother_status(MotherStatusFacts {
            version: "0.67.1".to_string(),
            uptime_secs: 42,
            control_plane_ready: true,
            registered_projects: 48,
            children_ready_count: 1,
            children_total: 2,
            startup_profile: "full".to_string(),
            memory_pressure: "ok".to_string(),
            observed_at: Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap(),
        })
    }

    #[test]
    fn composes_explicit_request_into_open_buffer() {
        // obligation: spec.mother-view-request-composer.mvrc3-compose-api
        // obligation: spec.mother-view-request-composer.mvrc4-explicit-exact-open
        // obligation: rule-success.CaptureUserDisplayRequest
        // obligation: rule-success.SelectExplicitUserRequestedShape
        let mut service = ViewBufferService::with_catalog(status_catalog());

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                    match_kind: ShapeMatchKind::ExplicitUserChoice,
                    confidence: 1.0,
                }),
            })
            .expect("request should compose");

        assert_eq!(
            composed.request.outcome,
            DisplayRequestOutcome::BufferOpened
        );
        assert_eq!(
            composed
                .shape_match
                .as_ref()
                .expect("shape match should persist")
                .match_kind,
            ShapeMatchKind::ExplicitUserChoice
        );
        assert!(matches!(
            composed.open_outcome,
            Some(OpenBufferOutcome::Opened(_))
        ));
        assert_eq!(service.list_buffers().len(), 1);
    }

    #[test]
    fn composes_exact_request_above_threshold_into_open_buffer() {
        // obligation: spec.mother-view-request-composer.mvrc4-explicit-exact-open
        // obligation: rule-success.SelectExactShapeMatch
        let mut service = ViewBufferService::with_catalog(status_catalog());

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                    match_kind: ShapeMatchKind::Exact,
                    confidence: SHAPE_MATCH_CONFIDENCE_THRESHOLD,
                }),
            })
            .expect("request should compose");

        assert_eq!(
            composed.request.outcome,
            DisplayRequestOutcome::BufferOpened
        );
        assert!(matches!(
            composed.open_outcome,
            Some(OpenBufferOutcome::Opened(_))
        ));
    }

    #[test]
    fn compose_request_reports_observability_gap_when_required_fact_missing() {
        // obligation: spec.mother-view-request-composer.mvrc4-explicit-exact-open
        // obligation: rule-success.RecordObservabilityGapWhenRequiredFactIsMissing
        let catalog = status_catalog().without_fact("mother.status.children_total");
        let mut service = ViewBufferService::with_catalog(catalog);

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                    match_kind: ShapeMatchKind::ExplicitUserChoice,
                    confidence: 1.0,
                }),
            })
            .expect("request should compose");

        assert_eq!(
            composed.request.outcome,
            DisplayRequestOutcome::ObservabilityGapReported
        );
        assert!(matches!(
            composed.open_outcome,
            Some(OpenBufferOutcome::ObservabilityGap(_))
        ));
        assert_eq!(service.list_buffers().len(), 0);
        assert_eq!(service.list_gaps().len(), 1);
    }

    #[test]
    fn missing_and_inactive_shape_matches_do_not_open_buffers() {
        // obligation: spec.mother-view-request-composer.mvrc5-fail-closed-outcomes
        let mut inactive_shape = mother_status_shape();
        inactive_shape.active = false;
        let mut service =
            ViewBufferService::with_catalog_and_shapes(status_catalog(), vec![inactive_shape]);

        for shape_id in [MOTHER_STATUS_SHAPE_ID, "missing.shape"] {
            let composed = service
                .compose_request(ComposeViewRequest {
                    user_id: "local-user".to_string(),
                    agent_id: "pi".to_string(),
                    raw_request: "show mother status".to_string(),
                    proposed_match: Some(ProposedShapeMatch {
                        shape_id: Some(shape_id.to_string()),
                        match_kind: ShapeMatchKind::ExplicitUserChoice,
                        confidence: 1.0,
                    }),
                })
                .expect("request should compose");

            assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
            assert!(composed.open_outcome.is_none());
        }
        assert_eq!(service.list_buffers().len(), 0);
    }

    #[test]
    fn low_confidence_exact_request_does_not_open_buffer() {
        // obligation: spec.mother-view-request-composer.mvrc5-fail-closed-outcomes
        // obligation: rule-failure.SelectExactShapeMatch.1
        let mut service = ViewBufferService::with_catalog(status_catalog());

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                    match_kind: ShapeMatchKind::Exact,
                    confidence: 0.2,
                }),
            })
            .expect("request should compose");

        assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
        assert!(composed.open_outcome.is_none());
        assert_eq!(service.list_buffers().len(), 0);
        assert!(composed
            .reason
            .expect("reason should be present")
            .contains("below threshold"));
    }

    #[test]
    fn similar_and_no_match_requests_do_not_open_buffers() {
        // obligation: spec.mother-view-request-composer.mvrc5-fail-closed-outcomes
        let mut service = ViewBufferService::with_catalog(status_catalog());

        for proposed_match in [
            ProposedShapeMatch {
                shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                match_kind: ShapeMatchKind::Similar,
                confidence: 0.9,
            },
            ProposedShapeMatch {
                shape_id: None,
                match_kind: ShapeMatchKind::None,
                confidence: 0.0,
            },
        ] {
            let composed = service
                .compose_request(ComposeViewRequest {
                    user_id: "local-user".to_string(),
                    agent_id: "pi".to_string(),
                    raw_request: "show mother status".to_string(),
                    proposed_match: Some(proposed_match),
                })
                .expect("request should compose");

            assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
            assert!(composed.open_outcome.is_none());
        }
        assert_eq!(service.list_buffers().len(), 0);
    }

    #[test]
    fn opens_live_buffer_when_required_facts_are_observed() {
        // obligation: rule-success.OpenLiveBufferWhenRequiredFactsAreObserved
        let mut service = ViewBufferService::with_catalog(status_catalog());

        let outcome = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .expect("open should evaluate");

        let OpenBufferOutcome::Opened(opened) = outcome else {
            panic!("expected opened buffer");
        };
        assert_eq!(opened.buffer.state, BufferState::Live);
        assert_eq!(opened.buffer.name, "*Mother Status*");
        assert_eq!(opened.payload.frame.protocol, "patina:view-buffer");
        assert_eq!(service.list_buffers().len(), 1);
        assert_eq!(service.list_gaps().len(), 0);
    }

    #[test]
    fn opens_live_buffer_from_active_library_shape() {
        // obligation: spec.mother-view-shape-library.mvsl4-open-from-library
        // obligation: rule-success.OpenLiveBufferWhenRequiredFactsAreObserved
        let mut shape = mother_status_shape();
        shape.shape_id = "project.status.summary".to_string();
        shape.title = "Project Status Summary".to_string();
        shape.requirements = vec![required("mother.status.version", "display Mother version")];
        let mut service = ViewBufferService::with_catalog_and_shapes(status_catalog(), vec![shape]);

        let outcome = service
            .open_buffer(OpenBufferRequest {
                shape_id: "project.status.summary".to_string(),
            })
            .expect("library shape should open");

        let OpenBufferOutcome::Opened(opened) = outcome else {
            panic!("expected opened buffer");
        };
        assert_eq!(opened.buffer.shape_id, "project.status.summary");
        assert_eq!(opened.buffer.name, "*Project Status Summary*");
        assert_eq!(service.list_buffers().len(), 1);
    }

    #[test]
    fn unknown_library_shape_does_not_open() {
        // obligation: spec.mother-view-shape-library.mvsl6-tests-and-trace
        let mut service = ViewBufferService::with_catalog_and_shapes(status_catalog(), Vec::new());

        let error = service
            .open_buffer(OpenBufferRequest {
                shape_id: "missing.shape".to_string(),
            })
            .unwrap_err();

        assert!(error.to_string().contains("unknown view shape"));
        assert_eq!(service.list_buffers().len(), 0);
    }

    #[test]
    fn inactive_library_shape_does_not_open() {
        // obligation: spec.mother-view-shape-library.mvsl4-open-from-library
        let mut shape = mother_status_shape();
        shape.active = false;
        let mut service = ViewBufferService::with_catalog_and_shapes(status_catalog(), vec![shape]);

        let error = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .unwrap_err();

        assert!(error.to_string().contains("inactive view shape"));
        assert_eq!(service.list_buffers().len(), 0);
    }

    #[test]
    fn records_gap_and_refuses_buffer_when_required_fact_missing() {
        // obligation: rule-success.RecordObservabilityGapWhenRequiredFactIsMissing
        // obligation: rule-failure.OpenLiveBufferWhenRequiredFactsAreObserved.2
        let catalog = status_catalog().without_fact("mother.status.children_total");
        let mut service = ViewBufferService::with_catalog(catalog);

        let outcome = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .expect("open should evaluate");

        let OpenBufferOutcome::ObservabilityGap(gap) = outcome else {
            panic!("expected observability gap");
        };
        assert_eq!(gap.status, ObservabilityGapStatus::Open);
        assert_eq!(gap.missing_fact_path, "mother.status.children_total");
        assert_eq!(service.list_buffers().len(), 0);
        assert_eq!(service.list_gaps().len(), 1);
    }

    #[test]
    fn records_gap_and_refuses_buffer_when_required_source_unavailable() {
        // obligation: rule-success.RecordObservabilityGapWhenRequiredFactIsMissing
        // obligation: rule-failure.OpenLiveBufferWhenRequiredFactsAreObserved.2
        let catalog = status_catalog()
            .with_source_availability(MOTHER_STATUS_SOURCE_ID, SourceAvailability::Unavailable);
        let mut service = ViewBufferService::with_catalog(catalog);

        let outcome = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .expect("open should evaluate");

        let OpenBufferOutcome::ObservabilityGap(gap) = outcome else {
            panic!("expected observability gap");
        };
        assert_eq!(gap.status, ObservabilityGapStatus::Open);
        assert_eq!(
            gap.missing_source_id,
            Some(MOTHER_STATUS_SOURCE_ID.to_string())
        );
        assert_eq!(service.list_buffers().len(), 0);
        assert_eq!(service.list_gaps().len(), 1);
    }

    #[test]
    fn connects_disconnects_and_kills_live_buffer() {
        // obligation: rule-success.ConnectWindowToExistingBuffer
        // obligation: rule-success.DisconnectWindowWithoutKillingBuffer
        // obligation: rule-success.KillBufferWhenUserClosesBuffer
        let mut service = ViewBufferService::with_catalog(status_catalog());
        let outcome = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .expect("open should evaluate");
        let OpenBufferOutcome::Opened(opened) = outcome else {
            panic!("expected opened buffer");
        };

        let connected = service
            .connect_window(ConnectWindowRequest {
                frame_id: "frame_tui".to_string(),
                frame_kind: FrameKind::Tui,
                window_id: "win_1".to_string(),
                buffer_id: opened.buffer.buffer_id.clone(),
            })
            .expect("window should connect");
        assert_eq!(connected.connection_state, WindowConnectionState::Connected);
        assert_eq!(service.list_frames().len(), 1);
        assert_eq!(service.list_windows().len(), 1);

        let disconnected = service
            .disconnect_window(DisconnectWindowRequest {
                window_id: connected.window_id,
            })
            .expect("window should disconnect");
        assert_eq!(
            disconnected.connection_state,
            WindowConnectionState::Disconnected
        );
        assert_eq!(service.list_buffers()[0].state, BufferState::Live);

        let killed = service
            .kill_buffer(KillBufferRequest {
                buffer_id: opened.buffer.buffer_id,
            })
            .expect("buffer should be killed");
        assert_eq!(killed.state, BufferState::Killed);
    }
}
