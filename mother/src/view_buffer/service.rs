use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    Buffer, BufferState, DisplayRequest, DisplayRequestOutcome, Frame, FrameKind,
    FramedJsonPayload, MajorMode, MinorMode, ObservabilityGap, PayloadContract, ShapeMatch,
    ShapeMatchKind, ViewRequestDetail, ViewRequirement, ViewShape, ViewShapeAdaptation,
    ViewShapeCreation, ViewShapeMaturity, ViewShapeRevision, ViewShapeRevisionOrigin,
    ViewShapeRevisionState, ViewShapeScope, Window, WindowConnectionState,
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
pub struct ProposedInitialShape {
    pub title: String,
    pub major_mode: MajorMode,
    #[serde(default)]
    pub minor_modes: Vec<MinorMode>,
    pub requirements: Vec<ViewRequirement>,
    #[serde(default)]
    pub vision_id: Option<String>,
    #[serde(default)]
    pub project_uid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeViewRequest {
    pub user_id: String,
    pub agent_id: String,
    pub raw_request: String,
    pub proposed_match: Option<ProposedShapeMatch>,
    #[serde(default)]
    pub proposed_initial_shape: Option<ProposedInitialShape>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComposedViewRequest {
    pub request: DisplayRequest,
    pub shape_match: Option<ShapeMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_adaptation: Option<ViewShapeAdaptation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapted_shape: Option<ViewShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_creation: Option<ViewShapeCreation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_shape: Option<ViewShape>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRequestShapeRequest {
    pub request_id: String,
    #[serde(default)]
    pub shape_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenRequestShapeOutcome {
    pub request_id: String,
    pub shape_id: String,
    pub open_outcome: OpenBufferOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviseViewShapeRequest {
    pub user_id: String,
    pub agent_id: String,
    pub shape_id: String,
    #[serde(default)]
    pub previous_buffer_id: Option<String>,
    pub revision_scope: ViewShapeScope,
    pub reason: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub major_mode: Option<MajorMode>,
    #[serde(default)]
    pub minor_modes: Option<Vec<MinorMode>>,
    #[serde(default)]
    pub requirements: Option<Vec<ViewRequirement>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisedViewShapeOutcome {
    pub revision: ViewShapeRevision,
    pub previous_shape: ViewShape,
    pub revised_shape: ViewShape,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_buffer: Option<Buffer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_open_outcome: Option<OpenBufferOutcome>,
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
        Self::with_catalog_shapes_and_buffers(catalog, shapes, Vec::new())
    }

    pub fn with_catalog_shapes_and_buffers(
        catalog: DataCatalog,
        shapes: impl IntoIterator<Item = ViewShape>,
        buffers: impl IntoIterator<Item = Buffer>,
    ) -> Self {
        Self {
            catalog,
            shapes: shapes
                .into_iter()
                .map(|shape| (shape.shape_id.clone(), shape))
                .collect(),
            buffers: buffers
                .into_iter()
                .map(|buffer| (buffer.buffer_id.clone(), buffer))
                .collect(),
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

    pub fn list_shapes(&self) -> Vec<ViewShape> {
        self.shapes.values().cloned().collect()
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
                shape_adaptation: None,
                adapted_shape: None,
                shape_creation: None,
                created_shape: None,
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

        if shape_match.match_kind == ShapeMatchKind::None {
            return match self.create_initial_shape(
                &display_request.request_id,
                request.proposed_initial_shape.as_ref(),
            ) {
                Ok((shape_creation, created_shape)) => {
                    display_request.outcome = DisplayRequestOutcome::Unable;
                    Ok(ComposedViewRequest {
                        request: display_request,
                        shape_match: Some(shape_match),
                        shape_adaptation: None,
                        adapted_shape: None,
                        shape_creation: Some(shape_creation),
                        created_shape: Some(created_shape),
                        open_outcome: None,
                        reason: None,
                    })
                }
                Err(reason) => {
                    display_request.outcome = DisplayRequestOutcome::Unable;
                    Ok(ComposedViewRequest {
                        request: display_request,
                        shape_match: Some(shape_match),
                        shape_adaptation: None,
                        adapted_shape: None,
                        shape_creation: None,
                        created_shape: None,
                        open_outcome: None,
                        reason: Some(reason),
                    })
                }
            };
        }

        if shape_match.match_kind == ShapeMatchKind::Similar {
            return match self.adapt_similar_shape(&display_request.request_id, &shape_match) {
                Ok((shape_adaptation, adapted_shape)) => {
                    display_request.outcome = DisplayRequestOutcome::Unable;
                    Ok(ComposedViewRequest {
                        request: display_request,
                        shape_match: Some(shape_match),
                        shape_adaptation: Some(shape_adaptation),
                        adapted_shape: Some(adapted_shape),
                        shape_creation: None,
                        created_shape: None,
                        open_outcome: None,
                        reason: None,
                    })
                }
                Err(reason) => {
                    display_request.outcome = DisplayRequestOutcome::Unable;
                    Ok(ComposedViewRequest {
                        request: display_request,
                        shape_match: Some(shape_match),
                        shape_adaptation: None,
                        adapted_shape: None,
                        shape_creation: None,
                        created_shape: None,
                        open_outcome: None,
                        reason: Some(reason),
                    })
                }
            };
        }

        let should_open = match self.validate_openable_match(&shape_match) {
            Ok(()) => true,
            Err(reason) => {
                display_request.outcome = DisplayRequestOutcome::Unable;
                return Ok(ComposedViewRequest {
                    request: display_request,
                    shape_match: Some(shape_match),
                    shape_adaptation: None,
                    adapted_shape: None,
                    shape_creation: None,
                    created_shape: None,
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
                shape_adaptation: None,
                adapted_shape: None,
                shape_creation: None,
                created_shape: None,
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
            ShapeMatchKind::Similar => Err("similar shape adaptation is unavailable".to_string()),
            ShapeMatchKind::None => Err("initial shape creation is unavailable".to_string()),
        }
    }

    fn create_initial_shape(
        &mut self,
        request_id: &str,
        proposal: Option<&ProposedInitialShape>,
    ) -> std::result::Result<(ViewShapeCreation, ViewShape), String> {
        // obligation: spec.mother-view-initial-shape-creation.mvisc2-catalog-guardrails
        // obligation: spec.mother-view-initial-shape-creation.mvisc3-initial-shape-creation
        // obligation: rule-success.CreateInitialShapeWhenNoShapeMatches
        let proposal = proposal
            .ok_or_else(|| "no initial shape proposal provided for no-match request".to_string())?;
        if proposal.title.trim().is_empty() {
            return Err("initial shape proposal title must not be empty".to_string());
        }
        let required_requirements: Vec<ViewRequirement> = proposal
            .requirements
            .iter()
            .filter(|requirement| requirement.required)
            .cloned()
            .collect();
        if required_requirements.is_empty() {
            return Err("initial shape proposal requires at least one required fact".to_string());
        }
        for requirement in &required_requirements {
            if requirement.fact_path.trim().is_empty() {
                return Err("initial shape proposal has blank fact_path".to_string());
            }
            if requirement.purpose.trim().is_empty() {
                return Err(format!(
                    "initial shape proposal requirement '{}' has blank purpose",
                    requirement.fact_path
                ));
            }
            if self.catalog.fact(&requirement.fact_path).is_none() {
                return Err(format!(
                    "initial shape proposal references uncatalogued fact '{}'",
                    requirement.fact_path
                ));
            }
            if !self.catalog.observed_required_fact(requirement) {
                return Err(format!(
                    "initial shape proposal required fact '{}' is not observed from an available source",
                    requirement.fact_path
                ));
            }
        }

        let created_shape_id = self.next_initial_shape_id(request_id);
        let created_shape = ViewShape {
            shape_id: created_shape_id.clone(),
            title: proposal.title.trim().to_string(),
            source_ref: "local-allium-view-library".to_string(),
            scope: ViewShapeScope::MotherUser,
            version: 1,
            active: true,
            major_mode: proposal.major_mode.clone(),
            minor_modes: proposal.minor_modes.clone(),
            maturity: ViewShapeMaturity::Exploratory,
            payload_contract: PayloadContract::FramedJson,
            payload_version: 1,
            vision_id: proposal.vision_id.clone(),
            project_uid: proposal.project_uid.clone(),
            replaced_by: None,
            requirements: proposal.requirements.clone(),
        };
        let shape_creation = ViewShapeCreation::created_without_opening(
            request_id.to_string(),
            created_shape_id,
            proposal.requirements.clone(),
        );
        self.shapes
            .insert(created_shape.shape_id.clone(), created_shape.clone());
        Ok((shape_creation, created_shape))
    }

    fn adapt_similar_shape(
        &mut self,
        request_id: &str,
        shape_match: &ShapeMatch,
    ) -> std::result::Result<(ViewShapeAdaptation, ViewShape), String> {
        // obligation: spec.mother-view-shape-adaptation.mvsa2-adapted-shape-creation
        // obligation: spec.mother-view-shape-adaptation.mvsa4-compose-integration
        // obligation: rule-success.AdaptSimilarShapeWhenNoExactShapeExists
        if shape_match.confidence < SHAPE_MATCH_CONFIDENCE_THRESHOLD {
            return Err(format!(
                "similar shape match confidence {:.2} below threshold {:.2}",
                shape_match.confidence, SHAPE_MATCH_CONFIDENCE_THRESHOLD
            ));
        }
        let precedent_shape_id = shape_match
            .shape_id
            .as_deref()
            .ok_or_else(|| "similar shape match is missing shape_id".to_string())?;
        let precedent = self
            .shapes
            .get(precedent_shape_id)
            .cloned()
            .ok_or_else(|| format!("unknown view shape '{}'", precedent_shape_id))?;
        if !precedent.active {
            return Err(format!("inactive view shape '{}'", precedent_shape_id));
        }

        let adapted_shape_id = self.next_adapted_shape_id(&precedent);
        let adapted_shape = ViewShape {
            shape_id: adapted_shape_id.clone(),
            title: format!("Adapted {}", precedent.title),
            source_ref: precedent.source_ref.clone(),
            scope: precedent.scope.clone(),
            version: 1,
            active: true,
            major_mode: precedent.major_mode.clone(),
            minor_modes: precedent.minor_modes.clone(),
            maturity: ViewShapeMaturity::Exploratory,
            payload_contract: precedent.payload_contract.clone(),
            payload_version: precedent.payload_version,
            vision_id: precedent.vision_id.clone(),
            project_uid: precedent.project_uid.clone(),
            replaced_by: None,
            requirements: precedent.requirements.clone(),
        };
        let shape_adaptation = ViewShapeAdaptation::created_without_opening(
            request_id.to_string(),
            precedent.shape_id,
            adapted_shape_id,
        );
        self.shapes
            .insert(adapted_shape.shape_id.clone(), adapted_shape.clone());
        Ok((shape_adaptation, adapted_shape))
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

    pub fn revise_view_shape(
        &mut self,
        request: ReviseViewShapeRequest,
    ) -> Result<RevisedViewShapeOutcome> {
        // obligation: spec.mother-view-buffer-revision.mvbr1-revision-model
        // obligation: spec.mother-view-buffer-revision.mvbr2-catalog-guardrails
        // obligation: spec.mother-view-buffer-revision.mvbr3-shape-history
        // obligation: rule-success.ReplaceBufferWhenUserRevisesViewShape
        if request.shape_id.trim().is_empty() {
            return Err(anyhow!("view shape id must not be empty"));
        }
        if request.reason.trim().is_empty() {
            return Err(anyhow!("view shape revision reason must not be empty"));
        }

        let mut previous_shape = self
            .shapes
            .get(&request.shape_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown view shape '{}'", request.shape_id))?;
        if !previous_shape.active {
            return Err(anyhow!("inactive view shape '{}'", request.shape_id));
        }

        let revised_requirements = match request.requirements.clone() {
            Some(requirements) => {
                self.validate_revision_requirements(&requirements)?;
                requirements
            }
            None => previous_shape.requirements.clone(),
        };
        let revised_title = match request.title.as_deref() {
            Some(title) if title.trim().is_empty() => {
                return Err(anyhow!("view shape revision title must not be empty"));
            }
            Some(title) => title.trim().to_string(),
            None => previous_shape.title.clone(),
        };
        let revised_major_mode = request
            .major_mode
            .clone()
            .unwrap_or_else(|| previous_shape.major_mode.clone());
        let revised_minor_modes = request
            .minor_modes
            .clone()
            .unwrap_or_else(|| previous_shape.minor_modes.clone());
        if revised_title == previous_shape.title
            && revised_major_mode == previous_shape.major_mode
            && revised_minor_modes == previous_shape.minor_modes
            && revised_requirements == previous_shape.requirements
        {
            return Err(anyhow!("view shape revision must change shape metadata"));
        }

        let revised_shape_id = self.next_revised_shape_id(&previous_shape);
        let revised_shape = ViewShape {
            shape_id: revised_shape_id.clone(),
            title: revised_title,
            source_ref: previous_shape.source_ref.clone(),
            scope: request.revision_scope.clone(),
            version: previous_shape.version + 1,
            active: true,
            major_mode: revised_major_mode,
            minor_modes: revised_minor_modes,
            maturity: ViewShapeMaturity::Exploratory,
            payload_contract: previous_shape.payload_contract.clone(),
            payload_version: previous_shape.payload_version,
            vision_id: previous_shape.vision_id.clone(),
            project_uid: previous_shape.project_uid.clone(),
            replaced_by: None,
            requirements: revised_requirements,
        };

        previous_shape.active = false;
        previous_shape.replaced_by = Some(revised_shape_id.clone());
        self.shapes
            .insert(previous_shape.shape_id.clone(), previous_shape.clone());
        self.shapes
            .insert(revised_shape.shape_id.clone(), revised_shape.clone());

        let mut revision = ViewShapeRevision {
            revision_id: format!(
                "{}::revision::{}",
                previous_shape.shape_id,
                uuid::Uuid::new_v4().simple()
            ),
            user_id: request.user_id,
            agent_id: request.agent_id,
            previous_shape_id: previous_shape.shape_id.clone(),
            revised_shape_id: revised_shape.shape_id.clone(),
            previous_buffer_id: request.previous_buffer_id.clone(),
            replacement_buffer_id: None,
            revision_scope: request.revision_scope,
            revision_origin: ViewShapeRevisionOrigin::UserCorrection,
            revision_state: ViewShapeRevisionState::Applied,
            reason: request.reason.trim().to_string(),
            created_at: Utc::now(),
        };

        let mut replaced_buffer = None;
        let mut replacement_open_outcome = None;
        if let Some(previous_buffer_id) = request.previous_buffer_id {
            let previous_buffer = self
                .buffers
                .get(&previous_buffer_id)
                .cloned()
                .ok_or_else(|| anyhow!("unknown view buffer '{}'", previous_buffer_id))?;
            if previous_buffer.shape_id != previous_shape.shape_id {
                return Err(anyhow!(
                    "view buffer '{}' is not linked to shape '{}'",
                    previous_buffer.buffer_id,
                    previous_shape.shape_id
                ));
            }
            if !previous_buffer.state.is_connectable() {
                return Err(anyhow!(
                    "view buffer '{}' is not replaceable in state {:?}",
                    previous_buffer.buffer_id,
                    previous_buffer.state
                ));
            }
            let open_outcome = self.open_buffer(OpenBufferRequest {
                shape_id: revised_shape.shape_id.clone(),
            })?;
            if let OpenBufferOutcome::Opened(opened) = &open_outcome {
                let mut replaced = previous_buffer;
                replaced.state = BufferState::Replaced;
                replaced.replaced_at = Some(Utc::now());
                replaced.replacement_buffer_id = Some(opened.buffer.buffer_id.clone());
                revision.replacement_buffer_id = Some(opened.buffer.buffer_id.clone());
                self.buffers
                    .insert(replaced.buffer_id.clone(), replaced.clone());
                replaced_buffer = Some(replaced);
            }
            replacement_open_outcome = Some(open_outcome);
        }

        Ok(RevisedViewShapeOutcome {
            revision,
            previous_shape,
            revised_shape,
            replaced_buffer,
            replacement_open_outcome,
        })
    }

    fn validate_revision_requirements(&self, requirements: &[ViewRequirement]) -> Result<()> {
        let required_requirements: Vec<&ViewRequirement> = requirements
            .iter()
            .filter(|requirement| requirement.required)
            .collect();
        if required_requirements.is_empty() {
            return Err(anyhow!(
                "view shape revision requires at least one required fact"
            ));
        }
        for requirement in required_requirements {
            if requirement.fact_path.trim().is_empty() {
                return Err(anyhow!("view shape revision has blank fact_path"));
            }
            if requirement.purpose.trim().is_empty() {
                return Err(anyhow!(
                    "view shape revision requirement '{}' has blank purpose",
                    requirement.fact_path
                ));
            }
            if self.catalog.fact(&requirement.fact_path).is_none() {
                return Err(anyhow!(
                    "view shape revision references uncatalogued fact '{}'",
                    requirement.fact_path
                ));
            }
            if !self.catalog.observed_required_fact(requirement) {
                return Err(anyhow!(
                    "view shape revision required fact '{}' is not observed from an available source",
                    requirement.fact_path
                ));
            }
        }
        Ok(())
    }

    pub fn open_request_shape(
        &mut self,
        detail: &ViewRequestDetail,
        request: OpenRequestShapeRequest,
    ) -> Result<OpenRequestShapeOutcome> {
        // obligation: spec.mother-view-request-ux.mvru4-open-linked-shape-action
        // obligation: spec.mother-view-request-ux.mvru5-non-mutating-history
        // obligation: spec.mother-view-request-ux.mvru6-no-fake-data-guardrails
        if request.request_id != detail.request.request_id {
            return Err(anyhow!(
                "request detail '{}' does not match open action '{}'",
                detail.request.request_id,
                request.request_id
            ));
        }
        if request.shape_id.is_none() && detail.available_actions.len() > 1 {
            return Err(anyhow!(
                "shape_id is required when request '{}' has multiple open actions",
                request.request_id
            ));
        }
        let action = detail
            .linked_action_for_shape(request.shape_id.as_deref())
            .ok_or_else(|| match request.shape_id.as_deref() {
                Some(shape_id) => anyhow!(
                    "shape '{}' is not an openable shape linked to request '{}'",
                    shape_id,
                    request.request_id
                ),
                None => anyhow!(
                    "request '{}' has no openable linked shape",
                    request.request_id
                ),
            })?;
        let shape_id = action.shape_id.clone();
        let open_outcome = self.open_buffer(OpenBufferRequest {
            shape_id: shape_id.clone(),
        })?;
        Ok(OpenRequestShapeOutcome {
            request_id: request.request_id,
            shape_id,
            open_outcome,
        })
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

    fn next_revised_shape_id(&self, previous: &ViewShape) -> String {
        format!(
            "{}::revision::{}",
            previous.shape_id,
            uuid::Uuid::new_v4().simple()
        )
    }

    fn next_adapted_shape_id(&self, precedent: &ViewShape) -> String {
        format!(
            "{}::adapted::{}",
            precedent.shape_id,
            uuid::Uuid::new_v4().simple()
        )
    }

    fn next_initial_shape_id(&self, request_id: &str) -> String {
        format!("initial::{}::{}", request_id, uuid::Uuid::new_v4().simple())
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
                proposed_initial_shape: None,
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
                proposed_initial_shape: None,
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
                proposed_initial_shape: None,
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
                    proposed_initial_shape: None,
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
                proposed_initial_shape: None,
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
    fn no_match_request_does_not_open_buffer() {
        // obligation: spec.mother-view-request-composer.mvrc5-fail-closed-outcomes
        let mut service = ViewBufferService::with_catalog(status_catalog());

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: None,
                    match_kind: ShapeMatchKind::None,
                    confidence: 0.0,
                }),
                proposed_initial_shape: None,
            })
            .expect("request should compose");

        assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
        assert!(composed.shape_adaptation.is_none());
        assert!(composed.adapted_shape.is_none());
        assert!(composed.shape_creation.is_none());
        assert!(composed.created_shape.is_none());
        assert!(composed.open_outcome.is_none());
        assert_eq!(service.list_buffers().len(), 0);
    }

    #[test]
    fn view_initial_shape_creation_creates_exploratory_shape_without_opening_buffer() {
        // obligation: spec.mother-view-initial-shape-creation.mvisc2-catalog-guardrails
        // obligation: spec.mother-view-initial-shape-creation.mvisc3-initial-shape-creation
        // obligation: spec.mother-view-initial-shape-creation.mvisc5-compose-integration
        // obligation: rule-success.CreateInitialShapeWhenNoShapeMatches
        let mut service = ViewBufferService::with_catalog(status_catalog());
        let requirements = vec![required("mother.status.version", "display Mother version")];

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show runtime summary".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: None,
                    match_kind: ShapeMatchKind::None,
                    confidence: 0.0,
                }),
                proposed_initial_shape: Some(ProposedInitialShape {
                    title: "Mother Runtime Summary".to_string(),
                    major_mode: MajorMode::Table,
                    minor_modes: vec![MinorMode::Pinned],
                    requirements: requirements.clone(),
                    vision_id: Some("vision-1".to_string()),
                    project_uid: Some("2bdc808e".to_string()),
                }),
            })
            .expect("request should compose");

        assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
        assert!(composed.open_outcome.is_none());
        assert_eq!(service.list_buffers().len(), 0);
        let creation = composed
            .shape_creation
            .as_ref()
            .expect("no-match request should report shape creation");
        let created_shape = composed
            .created_shape
            .as_ref()
            .expect("no-match request should return created shape");
        assert_eq!(creation.created_shape_id, created_shape.shape_id);
        assert!(!creation.opens_buffer);
        assert_eq!(creation.requirements, requirements);
        assert!(created_shape
            .shape_id
            .starts_with(&format!("initial::{}::", composed.request.request_id)));
        assert_eq!(created_shape.title, "Mother Runtime Summary");
        assert_eq!(created_shape.source_ref, "local-allium-view-library");
        assert_eq!(created_shape.scope, ViewShapeScope::MotherUser);
        assert_eq!(created_shape.version, 1);
        assert!(created_shape.active);
        assert_eq!(created_shape.major_mode, MajorMode::Table);
        assert_eq!(created_shape.minor_modes, vec![MinorMode::Pinned]);
        assert_eq!(created_shape.maturity, ViewShapeMaturity::Exploratory);
        assert_eq!(created_shape.payload_contract, PayloadContract::FramedJson);
        assert_eq!(created_shape.payload_version, 1);
        assert_eq!(created_shape.vision_id, Some("vision-1".to_string()));
        assert_eq!(created_shape.project_uid, Some("2bdc808e".to_string()));
        assert_eq!(created_shape.replaced_by, None);
        assert_eq!(created_shape.requirements, requirements);
        assert!(service
            .list_shapes()
            .iter()
            .any(|shape| shape.shape_id == created_shape.shape_id));
    }

    #[test]
    fn view_initial_shape_creation_fails_closed_for_invalid_proposals() {
        // obligation: spec.mother-view-initial-shape-creation.mvisc6-fail-closed-guardrails
        // obligation: rule-failure.CreateInitialShapeWhenNoShapeMatches.1
        let cases = [
            (None, "no initial shape proposal"),
            (
                Some(ProposedInitialShape {
                    title: "  ".to_string(),
                    major_mode: MajorMode::Table,
                    minor_modes: vec![],
                    requirements: vec![required("mother.status.version", "display Mother version")],
                    vision_id: None,
                    project_uid: None,
                }),
                "title must not be empty",
            ),
            (
                Some(ProposedInitialShape {
                    title: "Mother Runtime Summary".to_string(),
                    major_mode: MajorMode::Table,
                    minor_modes: vec![],
                    requirements: vec![],
                    vision_id: None,
                    project_uid: None,
                }),
                "at least one required fact",
            ),
            (
                Some(ProposedInitialShape {
                    title: "Mother Runtime Summary".to_string(),
                    major_mode: MajorMode::Table,
                    minor_modes: vec![],
                    requirements: vec![required("mother.status.missing", "display missing fact")],
                    vision_id: None,
                    project_uid: None,
                }),
                "uncatalogued fact",
            ),
        ];

        for (proposal, expected_reason) in cases {
            let mut service = ViewBufferService::with_catalog(status_catalog());
            let composed = service
                .compose_request(ComposeViewRequest {
                    user_id: "local-user".to_string(),
                    agent_id: "pi".to_string(),
                    raw_request: "show runtime summary".to_string(),
                    proposed_match: Some(ProposedShapeMatch {
                        shape_id: None,
                        match_kind: ShapeMatchKind::None,
                        confidence: 0.0,
                    }),
                    proposed_initial_shape: proposal,
                })
                .expect("request should compose");

            assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
            assert!(composed.shape_creation.is_none());
            assert!(composed.created_shape.is_none());
            assert!(composed.open_outcome.is_none());
            assert_eq!(service.list_buffers().len(), 0);
            assert_eq!(service.list_shapes().len(), 1);
            assert!(composed
                .reason
                .expect("reason should explain fail-closed result")
                .contains(expected_reason));
        }
    }

    #[test]
    fn view_initial_shape_creation_ignores_initial_proposal_for_non_none_matches() {
        // obligation: spec.mother-view-initial-shape-creation.mvisc6-fail-closed-guardrails
        let mut service = ViewBufferService::with_catalog(status_catalog());

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show something like mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                    match_kind: ShapeMatchKind::Similar,
                    confidence: SHAPE_MATCH_CONFIDENCE_THRESHOLD,
                }),
                proposed_initial_shape: Some(ProposedInitialShape {
                    title: "Should Not Be Created".to_string(),
                    major_mode: MajorMode::Table,
                    minor_modes: vec![],
                    requirements: vec![required("mother.status.version", "display Mother version")],
                    vision_id: None,
                    project_uid: None,
                }),
            })
            .expect("request should compose");

        assert!(composed.shape_creation.is_none());
        assert!(composed.created_shape.is_none());
        assert!(composed.shape_adaptation.is_some());
        assert!(composed.adapted_shape.is_some());
        assert_eq!(service.list_buffers().len(), 0);
    }

    #[test]
    fn view_shape_adaptation_creates_exploratory_shape_without_opening_buffer() {
        // obligation: spec.mother-view-shape-adaptation.mvsa2-adapted-shape-creation
        // obligation: spec.mother-view-shape-adaptation.mvsa4-compose-integration
        // obligation: rule-success.AdaptSimilarShapeWhenNoExactShapeExists
        let precedent = mother_status_shape();
        let mut service =
            ViewBufferService::with_catalog_and_shapes(status_catalog(), vec![precedent.clone()]);

        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show something like mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                    match_kind: ShapeMatchKind::Similar,
                    confidence: SHAPE_MATCH_CONFIDENCE_THRESHOLD,
                }),
                proposed_initial_shape: None,
            })
            .expect("request should compose");

        assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
        assert!(composed.open_outcome.is_none());
        assert_eq!(service.list_buffers().len(), 0);
        let adaptation = composed
            .shape_adaptation
            .as_ref()
            .expect("similar match should report adaptation");
        let adapted_shape = composed
            .adapted_shape
            .as_ref()
            .expect("similar match should return adapted shape");
        assert_eq!(adaptation.precedent_shape_id, MOTHER_STATUS_SHAPE_ID);
        assert_eq!(adaptation.adapted_shape_id, adapted_shape.shape_id);
        assert!(!adaptation.opens_buffer);
        assert!(adapted_shape
            .shape_id
            .starts_with("mother.status.default::adapted::"));
        assert_eq!(adapted_shape.title, "Adapted Mother Status");
        assert_eq!(adapted_shape.source_ref, precedent.source_ref);
        assert_eq!(adapted_shape.scope, precedent.scope);
        assert_eq!(adapted_shape.version, 1);
        assert!(adapted_shape.active);
        assert_eq!(adapted_shape.major_mode, precedent.major_mode);
        assert_eq!(adapted_shape.minor_modes, precedent.minor_modes);
        assert_eq!(adapted_shape.maturity, ViewShapeMaturity::Exploratory);
        assert_eq!(adapted_shape.payload_contract, precedent.payload_contract);
        assert_eq!(adapted_shape.payload_version, precedent.payload_version);
        assert_eq!(adapted_shape.vision_id, precedent.vision_id);
        assert_eq!(adapted_shape.project_uid, precedent.project_uid);
        assert_eq!(adapted_shape.replaced_by, None);
        assert_eq!(adapted_shape.requirements, precedent.requirements);
        assert!(service
            .list_shapes()
            .iter()
            .any(|shape| shape.shape_id == adapted_shape.shape_id));
    }

    #[test]
    fn view_shape_adaptation_fails_closed_for_invalid_similar_matches() {
        // obligation: spec.mother-view-shape-adaptation.mvsa5-fail-closed-guardrails
        // obligation: rule-failure.AdaptSimilarShapeWhenNoExactShapeExists.1
        let cases = [
            (
                Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                0.2,
                "below threshold",
            ),
            (None, 0.9, "missing shape_id"),
            (Some("missing.shape".to_string()), 0.9, "unknown view shape"),
        ];

        for (shape_id, confidence, expected_reason) in cases {
            let mut service = ViewBufferService::with_catalog(status_catalog());
            let composed = service
                .compose_request(ComposeViewRequest {
                    user_id: "local-user".to_string(),
                    agent_id: "pi".to_string(),
                    raw_request: "show something like mother status".to_string(),
                    proposed_match: Some(ProposedShapeMatch {
                        shape_id,
                        match_kind: ShapeMatchKind::Similar,
                        confidence,
                    }),
                    proposed_initial_shape: None,
                })
                .expect("request should compose");

            assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
            assert!(composed.shape_adaptation.is_none());
            assert!(composed.adapted_shape.is_none());
            assert!(composed.open_outcome.is_none());
            assert_eq!(service.list_buffers().len(), 0);
            assert_eq!(service.list_shapes().len(), 1);
            assert!(composed
                .reason
                .expect("reason should explain fail-closed result")
                .contains(expected_reason));
        }

        let mut inactive_shape = mother_status_shape();
        inactive_shape.active = false;
        let mut service =
            ViewBufferService::with_catalog_and_shapes(status_catalog(), vec![inactive_shape]);
        let composed = service
            .compose_request(ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show something like mother status".to_string(),
                proposed_match: Some(ProposedShapeMatch {
                    shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                    match_kind: ShapeMatchKind::Similar,
                    confidence: 0.9,
                }),
                proposed_initial_shape: None,
            })
            .expect("request should compose");

        assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
        assert!(composed.shape_adaptation.is_none());
        assert!(composed.adapted_shape.is_none());
        assert!(composed.open_outcome.is_none());
        assert_eq!(service.list_buffers().len(), 0);
        assert!(composed
            .reason
            .expect("reason should explain inactive precedent")
            .contains("inactive view shape"));
    }

    #[test]
    fn view_shape_adaptation_composed_request_can_report_without_opening_buffer() {
        // obligation: spec.mother-view-shape-adaptation.mvsa1-adaptation-model
        // obligation: rule-success.AdaptSimilarShapeWhenNoExactShapeExists
        let request = DisplayRequest {
            request_id: "req_1".to_string(),
            user_id: "local-user".to_string(),
            agent_id: "pi".to_string(),
            raw_request: "show something like mother status".to_string(),
            requested_at: Utc::now(),
            outcome: DisplayRequestOutcome::Unable,
        };
        let shape_match = ShapeMatch {
            request_id: request.request_id.clone(),
            shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
            match_kind: ShapeMatchKind::Similar,
            confidence: 0.9,
        };
        let adaptation = ViewShapeAdaptation::created_without_opening(
            request.request_id.clone(),
            MOTHER_STATUS_SHAPE_ID.to_string(),
            "mother.status.default::adapted::test".to_string(),
        );

        let composed = ComposedViewRequest {
            request,
            shape_match: Some(shape_match),
            shape_adaptation: Some(adaptation),
            adapted_shape: None,
            shape_creation: None,
            created_shape: None,
            open_outcome: None,
            reason: None,
        };

        assert_eq!(composed.request.outcome, DisplayRequestOutcome::Unable);
        assert_eq!(
            composed
                .shape_adaptation
                .as_ref()
                .expect("adaptation should be structured")
                .precedent_shape_id,
            MOTHER_STATUS_SHAPE_ID
        );
        assert!(composed.open_outcome.is_none());
    }

    #[test]
    fn view_buffer_revision_creates_revised_shape_and_replaces_buffer() {
        // obligation: spec.mother-view-buffer-revision.mvbr3-shape-history
        // obligation: spec.mother-view-buffer-revision.mvbr4-buffer-replacement
        let mut service = ViewBufferService::with_catalog(status_catalog());
        let opened = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .expect("open should evaluate");
        let OpenBufferOutcome::Opened(opened) = opened else {
            panic!("expected opened buffer");
        };

        let outcome = service
            .revise_view_shape(ReviseViewShapeRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
                previous_buffer_id: Some(opened.buffer.buffer_id.clone()),
                revision_scope: ViewShapeScope::MotherUser,
                reason: "show readiness first".to_string(),
                title: Some("Mother Readiness".to_string()),
                major_mode: None,
                minor_modes: Some(vec![MinorMode::Pinned, MinorMode::Sorted]),
                requirements: None,
            })
            .expect("revision should apply");

        assert_eq!(outcome.previous_shape.active, false);
        assert_eq!(
            outcome.previous_shape.replaced_by.as_deref(),
            Some(outcome.revised_shape.shape_id.as_str())
        );
        assert_eq!(outcome.revised_shape.title, "Mother Readiness");
        assert_eq!(outcome.revised_shape.version, 2);
        let replaced = outcome
            .replaced_buffer
            .as_ref()
            .expect("previous buffer should be replaced");
        assert_eq!(replaced.state, BufferState::Replaced);
        assert!(replaced.replacement_buffer_id.is_some());
        assert!(matches!(
            outcome.replacement_open_outcome,
            Some(OpenBufferOutcome::Opened(_))
        ));
        assert_eq!(service.list_buffers().len(), 2);
    }

    #[test]
    fn view_buffer_revision_fails_closed_for_invalid_changes() {
        // obligation: spec.mother-view-buffer-revision.mvbr2-catalog-guardrails
        // obligation: spec.mother-view-buffer-revision.mvbr7-fail-closed-guardrails
        let cases = [
            (
                ReviseViewShapeRequest {
                    user_id: "local-user".to_string(),
                    agent_id: "pi".to_string(),
                    shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
                    previous_buffer_id: None,
                    revision_scope: ViewShapeScope::MotherUser,
                    reason: " ".to_string(),
                    title: Some("Mother Readiness".to_string()),
                    major_mode: None,
                    minor_modes: None,
                    requirements: None,
                },
                "reason must not be empty",
            ),
            (
                ReviseViewShapeRequest {
                    user_id: "local-user".to_string(),
                    agent_id: "pi".to_string(),
                    shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
                    previous_buffer_id: None,
                    revision_scope: ViewShapeScope::MotherUser,
                    reason: "no-op".to_string(),
                    title: None,
                    major_mode: None,
                    minor_modes: None,
                    requirements: None,
                },
                "must change shape metadata",
            ),
            (
                ReviseViewShapeRequest {
                    user_id: "local-user".to_string(),
                    agent_id: "pi".to_string(),
                    shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
                    previous_buffer_id: None,
                    revision_scope: ViewShapeScope::MotherUser,
                    reason: "unknown fact".to_string(),
                    title: Some("Mother Readiness".to_string()),
                    major_mode: None,
                    minor_modes: None,
                    requirements: Some(vec![required("missing.fact", "display missing fact")]),
                },
                "uncatalogued fact",
            ),
        ];

        for (request, expected) in cases {
            let mut service = ViewBufferService::with_catalog(status_catalog());
            let error = service.revise_view_shape(request).unwrap_err();
            assert!(error.to_string().contains(expected), "got: {error}");
            let shape = service
                .list_shapes()
                .into_iter()
                .find(|shape| shape.shape_id == MOTHER_STATUS_SHAPE_ID)
                .expect("shape should remain");
            assert!(shape.active);
            assert!(shape.replaced_by.is_none());
        }
    }

    #[test]
    fn view_request_ux_opens_only_linked_shape_without_mutating_request() {
        // obligation: spec.mother-view-request-ux.mvru4-open-linked-shape-action
        // obligation: spec.mother-view-request-ux.mvru5-non-mutating-history
        // obligation: spec.mother-view-request-ux.mvru6-no-fake-data-guardrails
        let request = DisplayRequest {
            request_id: "req_ux".to_string(),
            user_id: "local-user".to_string(),
            agent_id: "pi".to_string(),
            raw_request: "show mother status".to_string(),
            requested_at: Utc::now(),
            outcome: DisplayRequestOutcome::Unable,
        };
        let shape = mother_status_shape();
        let detail = ViewRequestDetail::from_parts(
            request.clone(),
            Some(ShapeMatch {
                request_id: request.request_id.clone(),
                shape_id: Some(shape.shape_id.clone()),
                match_kind: ShapeMatchKind::ExplicitUserChoice,
                confidence: 1.0,
            }),
            None,
            None,
            None,
            None,
            Some(shape),
        );
        let mut service = ViewBufferService::with_catalog(status_catalog());

        let outcome = service
            .open_request_shape(
                &detail,
                OpenRequestShapeRequest {
                    request_id: request.request_id.clone(),
                    shape_id: Some(MOTHER_STATUS_SHAPE_ID.to_string()),
                },
            )
            .expect("linked shape should open");

        assert_eq!(outcome.request_id, request.request_id);
        assert_eq!(outcome.shape_id, MOTHER_STATUS_SHAPE_ID);
        assert!(matches!(outcome.open_outcome, OpenBufferOutcome::Opened(_)));
        assert_eq!(detail.request.outcome, DisplayRequestOutcome::Unable);
        assert_eq!(service.list_buffers().len(), 1);

        let error = service
            .open_request_shape(
                &detail,
                OpenRequestShapeRequest {
                    request_id: "req_ux".to_string(),
                    shape_id: Some("unlinked.shape".to_string()),
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("not an openable shape linked"));
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
