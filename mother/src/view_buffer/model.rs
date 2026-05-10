use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BufferState {
    Live,
    Stale,
    Blocked,
    Replaced,
    Killed,
}

impl BufferState {
    pub fn is_connectable(&self) -> bool {
        matches!(self, Self::Live | Self::Stale | Self::Blocked)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MajorMode {
    Table,
    List,
    Graph,
    Timeline,
    Log,
    Markdown,
    Document,
    Browser,
    Image,
    Artifact,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MinorMode {
    Compact,
    Filtered,
    Grouped,
    Pinned,
    Alerting,
    FollowTail,
    Highlighted,
    Sorted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadContract {
    FramedJson,
    TypedWit,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewShapeScope {
    MotherUser,
    Vision,
    Project,
    BufferLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewShapeMaturity {
    Exploratory,
    Candidate,
    Stable,
    Promoted,
}

impl ViewShapeMaturity {
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Exploratory => Some(Self::Candidate),
            Self::Candidate => Some(Self::Stable),
            Self::Stable => Some(Self::Promoted),
            Self::Promoted => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMaturationTargetKind {
    Shape,
    Derivation,
    Pattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMaturationOrigin {
    UserRequested,
    MotherSuggested,
    AgentInferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayPatternKind {
    Grouping,
    Sorting,
    Filtering,
    Highlighting,
    Alerting,
    Sectioning,
    ModeBehavior,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CataloguedSourceKind {
    Registry,
    Eventlog,
    Database,
    Artifact,
    File,
    Log,
    IndexedExternalSource,
    DerivedIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceAvailability {
    Available,
    Unavailable,
    Stale,
}

impl SourceAvailability {
    pub fn is_available(&self) -> bool {
        self == &Self::Available
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationState {
    Observed,
    Stale,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FactKind {
    Raw,
    Derived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrameKind {
    Sveltekit,
    Tui,
    Emacs,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowConnectionState {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservabilityGapStatus {
    Open,
    LinkedToWorkItem,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayRequestOutcome {
    Pending,
    BufferOpened,
    ObservabilityGapReported,
    Unable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeMatchKind {
    Exact,
    ExplicitUserChoice,
    Similar,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CataloguedSource {
    pub source_id: String,
    pub source_kind: CataloguedSourceKind,
    pub availability: SourceAvailability,
    pub last_observed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CataloguedFact {
    pub fact_path: String,
    pub source_id: String,
    pub fact_kind: FactKind,
    pub observation_state: ObservationState,
}

impl CataloguedFact {
    pub fn is_observed(&self) -> bool {
        self.observation_state == ObservationState::Observed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewRequirement {
    pub fact_path: String,
    pub required: bool,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewShape {
    pub shape_id: String,
    pub title: String,
    pub source_ref: String,
    pub scope: ViewShapeScope,
    pub version: u32,
    pub active: bool,
    pub major_mode: MajorMode,
    pub minor_modes: Vec<MinorMode>,
    pub maturity: ViewShapeMaturity,
    pub payload_contract: PayloadContract,
    pub payload_version: u32,
    pub vision_id: Option<String>,
    pub project_uid: Option<String>,
    pub replaced_by: Option<String>,
    pub requirements: Vec<ViewRequirement>,
}

impl ViewShape {
    pub fn requires_fact(&self, fact_path: &str) -> bool {
        self.requirements
            .iter()
            .any(|requirement| requirement.required && requirement.fact_path == fact_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDerivation {
    pub derivation_id: String,
    pub shape_id: String,
    pub label: String,
    pub expression_ref: String,
    pub input_fact_paths: Vec<String>,
    pub maturity: ViewShapeMaturity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayPattern {
    pub pattern_id: String,
    pub shape_id: String,
    pub pattern_kind: DisplayPatternKind,
    pub maturity: ViewShapeMaturity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewMaturationEvent {
    pub maturation_id: String,
    pub target_kind: ViewMaturationTargetKind,
    pub shape_id: Option<String>,
    pub derivation_id: Option<String>,
    pub pattern_id: Option<String>,
    pub origin: ViewMaturationOrigin,
    pub from_maturity: ViewShapeMaturity,
    pub to_maturity: ViewShapeMaturity,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityImprovementArtifact {
    pub artifact_id: String,
    pub source_gap_id: Option<String>,
    pub source_maturation_id: Option<String>,
    pub desired_fact_path: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub work_item_created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedObservabilityImprovement {
    pub desired_fact_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatureViewArtifactRequest {
    pub target_kind: ViewMaturationTargetKind,
    #[serde(default)]
    pub shape_id: Option<String>,
    #[serde(default)]
    pub derivation_id: Option<String>,
    #[serde(default)]
    pub pattern_id: Option<String>,
    pub origin: ViewMaturationOrigin,
    pub to_maturity: ViewShapeMaturity,
    #[serde(default)]
    pub observability_improvement: Option<ProposedObservabilityImprovement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaturedViewArtifactOutcome {
    pub event: ViewMaturationEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<ViewShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation: Option<ViewDerivation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<DisplayPattern>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observability_improvement: Option<ObservabilityImprovementArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayRequest {
    pub request_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub raw_request: String,
    pub requested_at: DateTime<Utc>,
    pub outcome: DisplayRequestOutcome,
}

impl DisplayRequest {
    pub fn pending(
        request_id: String,
        user_id: String,
        agent_id: String,
        raw_request: String,
        requested_at: DateTime<Utc>,
    ) -> Self {
        Self {
            request_id,
            user_id,
            agent_id,
            raw_request,
            requested_at,
            outcome: DisplayRequestOutcome::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeMatch {
    pub request_id: String,
    pub shape_id: Option<String>,
    pub match_kind: ShapeMatchKind,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewShapeCreation {
    pub request_id: String,
    pub created_shape_id: String,
    pub opens_buffer: bool,
    pub request_outcome: DisplayRequestOutcome,
    pub requirements: Vec<ViewRequirement>,
}

impl ViewShapeCreation {
    pub fn created_without_opening(
        request_id: String,
        created_shape_id: String,
        requirements: Vec<ViewRequirement>,
    ) -> Self {
        Self {
            request_id,
            created_shape_id,
            opens_buffer: false,
            request_outcome: DisplayRequestOutcome::Unable,
            requirements,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewShapeAdaptation {
    pub request_id: String,
    pub precedent_shape_id: String,
    pub adapted_shape_id: String,
    pub opens_buffer: bool,
    pub request_outcome: DisplayRequestOutcome,
}

impl ViewShapeAdaptation {
    pub fn created_without_opening(
        request_id: String,
        precedent_shape_id: String,
        adapted_shape_id: String,
    ) -> Self {
        Self {
            request_id,
            precedent_shape_id,
            adapted_shape_id,
            opens_buffer: false,
            request_outcome: DisplayRequestOutcome::Unable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewRequestActionKind {
    OpenMatchedShape,
    OpenAdaptedShape,
    OpenCreatedShape,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewRequestAction {
    pub kind: ViewRequestActionKind,
    pub shape_id: String,
    pub label: String,
}

impl ViewRequestAction {
    pub fn open_matched_shape(shape_id: String) -> Self {
        Self {
            kind: ViewRequestActionKind::OpenMatchedShape,
            shape_id,
            label: "Open matched shape".to_string(),
        }
    }

    pub fn open_adapted_shape(shape_id: String) -> Self {
        Self {
            kind: ViewRequestActionKind::OpenAdaptedShape,
            shape_id,
            label: "Open adapted shape".to_string(),
        }
    }

    pub fn open_created_shape(shape_id: String) -> Self {
        Self {
            kind: ViewRequestActionKind::OpenCreatedShape,
            shape_id,
            label: "Open created shape".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewShapeRevisionOrigin {
    UserCorrection,
    UserRequest,
    AgentAdaptation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewShapeRevisionState {
    Applied,
    Proposed,
    Rejected,
    Reverted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewShapeRevision {
    pub revision_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub previous_shape_id: String,
    pub revised_shape_id: String,
    pub previous_buffer_id: Option<String>,
    pub replacement_buffer_id: Option<String>,
    pub revision_scope: ViewShapeScope,
    pub revision_origin: ViewShapeRevisionOrigin,
    pub revision_state: ViewShapeRevisionState,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewRequestDetail {
    pub request: DisplayRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_match: Option<ShapeMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_adaptation: Option<ViewShapeAdaptation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapted_shape: Option<ViewShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_creation: Option<ViewShapeCreation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_shape: Option<ViewShape>,
    pub available_actions: Vec<ViewRequestAction>,
}

impl ViewRequestDetail {
    pub fn from_parts(
        request: DisplayRequest,
        shape_match: Option<ShapeMatch>,
        shape_adaptation: Option<ViewShapeAdaptation>,
        adapted_shape: Option<ViewShape>,
        shape_creation: Option<ViewShapeCreation>,
        created_shape: Option<ViewShape>,
        matched_shape: Option<ViewShape>,
    ) -> Self {
        // obligation: spec.mother-view-request-ux.mvru1-detail-model
        let mut available_actions = Vec::new();
        if let Some(shape) = matched_shape.filter(|shape| shape.active) {
            available_actions.push(ViewRequestAction::open_matched_shape(shape.shape_id));
        }
        if let Some(shape) = adapted_shape.as_ref().filter(|shape| shape.active) {
            available_actions.push(ViewRequestAction::open_adapted_shape(
                shape.shape_id.clone(),
            ));
        }
        if let Some(shape) = created_shape.as_ref().filter(|shape| shape.active) {
            available_actions.push(ViewRequestAction::open_created_shape(
                shape.shape_id.clone(),
            ));
        }

        Self {
            request,
            shape_match,
            shape_adaptation,
            adapted_shape,
            shape_creation,
            created_shape,
            available_actions,
        }
    }

    pub fn linked_action_for_shape(&self, shape_id: Option<&str>) -> Option<&ViewRequestAction> {
        match shape_id {
            Some(shape_id) => self
                .available_actions
                .iter()
                .find(|action| action.shape_id == shape_id),
            None if self.available_actions.len() == 1 => self.available_actions.first(),
            None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Buffer {
    pub buffer_id: String,
    pub name: String,
    pub shape_id: String,
    pub state: BufferState,
    pub created_at: DateTime<Utc>,
    pub stale_at: Option<DateTime<Utc>>,
    pub blocked_at: Option<DateTime<Utc>>,
    pub replaced_at: Option<DateTime<Utc>>,
    pub killed_at: Option<DateTime<Utc>>,
    pub replacement_buffer_id: Option<String>,
    pub major_mode: MajorMode,
    pub minor_modes: Vec<MinorMode>,
    pub payload_contract: PayloadContract,
    pub payload_version: u32,
}

impl Buffer {
    pub fn live_from_shape(
        buffer_id: String,
        shape: &ViewShape,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            buffer_id,
            name: format!("*{}*", shape.title),
            shape_id: shape.shape_id.clone(),
            state: BufferState::Live,
            created_at,
            stale_at: None,
            blocked_at: None,
            replaced_at: None,
            killed_at: None,
            replacement_buffer_id: None,
            major_mode: shape.major_mode.clone(),
            minor_modes: shape.minor_modes.clone(),
            payload_contract: shape.payload_contract.clone(),
            payload_version: shape.payload_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    pub frame_id: String,
    pub frame_kind: FrameKind,
    pub connected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    pub window_id: String,
    pub frame_id: String,
    pub buffer_id: Option<String>,
    pub connection_state: WindowConnectionState,
    pub connected_at: Option<DateTime<Utc>>,
    pub disconnected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityGap {
    pub gap_id: String,
    pub shape_id: Option<String>,
    pub missing_fact_path: String,
    pub missing_source_id: Option<String>,
    pub reason: String,
    pub status: ObservabilityGapStatus,
    pub linked_work_item_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl ObservabilityGap {
    pub fn open(
        gap_id: String,
        shape_id: Option<String>,
        missing_fact_path: String,
        missing_source_id: Option<String>,
        reason: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            gap_id,
            shape_id,
            missing_fact_path,
            missing_source_id,
            reason,
            status: ObservabilityGapStatus::Open,
            linked_work_item_id: None,
            created_at,
            resolved_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_buffer_from_shape_carries_emacs_mode_and_payload_contract() {
        // obligation: entity-state.Buffer + rule-success.OpenLiveBufferWhenRequiredFactsAreObserved
        let shape = ViewShape {
            shape_id: "mother.status.default".to_string(),
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
            requirements: vec![ViewRequirement {
                fact_path: "mother.status.control_plane_ready".to_string(),
                required: true,
                purpose: "display control-plane readiness".to_string(),
            }],
        };

        let buffer = Buffer::live_from_shape("buf_1".to_string(), &shape, Utc::now());

        assert_eq!(shape.source_ref, "local-allium-view-library");
        assert_eq!(shape.maturity, ViewShapeMaturity::Stable);
        assert_eq!(shape.vision_id, None);
        assert_eq!(shape.project_uid, None);
        assert_eq!(shape.replaced_by, None);
        assert_eq!(buffer.name, "*Mother Status*");
        assert_eq!(buffer.state, BufferState::Live);
        assert_eq!(buffer.major_mode, MajorMode::Table);
        assert_eq!(buffer.minor_modes, vec![MinorMode::Pinned]);
        assert_eq!(buffer.payload_contract, PayloadContract::FramedJson);
        assert!(shape.requires_fact("mother.status.control_plane_ready"));
    }

    #[test]
    fn display_request_and_shape_match_use_allium_vocabulary() {
        // obligation: spec.mother-view-request-composer.mvrc1-request-model
        // obligation: entity-state.DisplayRequest + entity-state.ShapeMatch
        let requested_at = Utc::now();
        let request = DisplayRequest::pending(
            "req_1".to_string(),
            "local-user".to_string(),
            "pi".to_string(),
            "show mother status".to_string(),
            requested_at,
        );
        let shape_match = ShapeMatch {
            request_id: request.request_id.clone(),
            shape_id: Some("mother.status.default".to_string()),
            match_kind: ShapeMatchKind::ExplicitUserChoice,
            confidence: 1.0,
        };

        assert_eq!(request.outcome, DisplayRequestOutcome::Pending);
        assert_eq!(shape_match.match_kind, ShapeMatchKind::ExplicitUserChoice);
        assert_eq!(
            serde_json::to_value(&shape_match.match_kind).unwrap(),
            serde_json::json!("explicit_user_choice")
        );
        assert_eq!(
            serde_json::to_value(&DisplayRequestOutcome::BufferOpened).unwrap(),
            serde_json::json!("buffer_opened")
        );
    }

    #[test]
    fn view_initial_shape_creation_records_non_open_request_semantics() {
        // obligation: spec.mother-view-initial-shape-creation.mvisc1-creation-model
        // obligation: rule-success.CreateInitialShapeWhenNoShapeMatches
        let requirements = vec![ViewRequirement {
            fact_path: "mother.status.version".to_string(),
            required: true,
            purpose: "display Mother binary version".to_string(),
        }];
        let creation = ViewShapeCreation::created_without_opening(
            "req_1".to_string(),
            "initial::req_1::test".to_string(),
            requirements.clone(),
        );

        assert_eq!(creation.request_id, "req_1");
        assert_eq!(creation.created_shape_id, "initial::req_1::test");
        assert!(!creation.opens_buffer);
        assert_eq!(creation.request_outcome, DisplayRequestOutcome::Unable);
        assert_eq!(creation.requirements, requirements);
        assert_eq!(
            serde_json::to_value(&creation).unwrap(),
            serde_json::json!({
                "request_id": "req_1",
                "created_shape_id": "initial::req_1::test",
                "opens_buffer": false,
                "request_outcome": "unable",
                "requirements": [{
                    "fact_path": "mother.status.version",
                    "required": true,
                    "purpose": "display Mother binary version"
                }]
            })
        );
    }

    #[test]
    fn view_shape_adaptation_records_precedent_and_non_open_request_semantics() {
        // obligation: spec.mother-view-shape-adaptation.mvsa1-adaptation-model
        // obligation: rule-success.AdaptSimilarShapeWhenNoExactShapeExists
        let adaptation = ViewShapeAdaptation::created_without_opening(
            "req_1".to_string(),
            "mother.status.default".to_string(),
            "mother.status.default::adapted::test".to_string(),
        );

        assert_eq!(adaptation.request_id, "req_1");
        assert_eq!(adaptation.precedent_shape_id, "mother.status.default");
        assert_eq!(
            adaptation.adapted_shape_id,
            "mother.status.default::adapted::test"
        );
        assert!(!adaptation.opens_buffer);
        assert_eq!(adaptation.request_outcome, DisplayRequestOutcome::Unable);
        assert_eq!(
            serde_json::to_value(&adaptation).unwrap(),
            serde_json::json!({
                "request_id": "req_1",
                "precedent_shape_id": "mother.status.default",
                "adapted_shape_id": "mother.status.default::adapted::test",
                "opens_buffer": false,
                "request_outcome": "unable"
            })
        );
    }

    #[test]
    fn view_observability_workflow_gap_detail_tracks_link_and_resolution() {
        // obligation: spec.mother-view-observability-workflow.mvow1-gap-detail-model
        // obligation: rule-success.LinkObservabilityGapToWorkItem
        let mut gap = ObservabilityGap::open(
            "gap_1".to_string(),
            Some("mother.status.default".to_string()),
            "mother.status.version".to_string(),
            Some("mother.status".to_string()),
            "missing version".to_string(),
            Utc::now(),
        );
        gap.status = ObservabilityGapStatus::LinkedToWorkItem;
        gap.linked_work_item_id = Some("work/MOTHER-123".to_string());
        gap.status = ObservabilityGapStatus::Resolved;
        gap.resolved_at = Some(Utc::now());

        assert_eq!(gap.linked_work_item_id.as_deref(), Some("work/MOTHER-123"));
        assert_eq!(gap.status, ObservabilityGapStatus::Resolved);
        assert!(gap.resolved_at.is_some());
    }

    #[test]
    fn view_maturation_model_records_forward_artifact_promotion() {
        // obligation: spec.mother-view-maturation.mvmat1-maturation-model
        // obligation: rule-success.PromoteMatureViewArtifact
        assert_eq!(
            ViewShapeMaturity::Exploratory.next(),
            Some(ViewShapeMaturity::Candidate)
        );
        assert_eq!(ViewShapeMaturity::Promoted.next(), None);

        let created_at = Utc::now();
        let event = ViewMaturationEvent {
            maturation_id: "maturation_1".to_string(),
            target_kind: ViewMaturationTargetKind::Derivation,
            shape_id: Some("mother.status.default".to_string()),
            derivation_id: Some("derivation_1".to_string()),
            pattern_id: None,
            origin: ViewMaturationOrigin::UserRequested,
            from_maturity: ViewShapeMaturity::Candidate,
            to_maturity: ViewShapeMaturity::Stable,
            created_at,
        };
        let improvement = ObservabilityImprovementArtifact {
            artifact_id: "maturation_1::observability-improvement".to_string(),
            source_gap_id: None,
            source_maturation_id: Some(event.maturation_id.clone()),
            desired_fact_path: "mother.status.memory_pressure".to_string(),
            reason: "stable derivation should become catalogued".to_string(),
            created_at,
            work_item_created: false,
        };

        assert_eq!(event.target_kind, ViewMaturationTargetKind::Derivation);
        assert_eq!(event.to_maturity, ViewShapeMaturity::Stable);
        assert_eq!(improvement.work_item_created, false);
        assert_eq!(
            improvement.source_maturation_id.as_deref(),
            Some("maturation_1")
        );
        assert_eq!(
            serde_json::to_value(&event.origin).unwrap(),
            serde_json::json!("user_requested")
        );
    }

    #[test]
    fn view_buffer_revision_records_shape_and_buffer_history() {
        // obligation: spec.mother-view-buffer-revision.mvbr1-revision-model
        // obligation: rule-success.ReplaceBufferWhenUserRevisesViewShape
        let created_at = Utc::now();
        let revision = ViewShapeRevision {
            revision_id: "mother.status.default::revision::test".to_string(),
            user_id: "local-user".to_string(),
            agent_id: "pi".to_string(),
            previous_shape_id: "mother.status.default".to_string(),
            revised_shape_id: "mother.status.default::revision::next".to_string(),
            previous_buffer_id: Some("buf_1".to_string()),
            replacement_buffer_id: Some("buf_2".to_string()),
            revision_scope: ViewShapeScope::MotherUser,
            revision_origin: ViewShapeRevisionOrigin::UserCorrection,
            revision_state: ViewShapeRevisionState::Applied,
            reason: "show readiness first".to_string(),
            created_at,
        };

        assert_eq!(revision.revision_state, ViewShapeRevisionState::Applied);
        assert_eq!(
            serde_json::to_value(&revision.revision_origin).unwrap(),
            serde_json::json!("user_correction")
        );
        assert_eq!(revision.replacement_buffer_id.as_deref(), Some("buf_2"));
    }

    #[test]
    fn view_request_ux_detail_exposes_created_shape_action() {
        // obligation: spec.mother-view-request-ux.mvru1-detail-model
        let request = DisplayRequest::pending(
            "req_1".to_string(),
            "local-user".to_string(),
            "pi".to_string(),
            "show runtime summary".to_string(),
            Utc::now(),
        );
        let creation = ViewShapeCreation::created_without_opening(
            request.request_id.clone(),
            "initial::req_1::test".to_string(),
            vec![ViewRequirement {
                fact_path: "mother.status.version".to_string(),
                required: true,
                purpose: "display Mother version".to_string(),
            }],
        );
        let created_shape = ViewShape {
            shape_id: creation.created_shape_id.clone(),
            title: "Mother Runtime Summary".to_string(),
            source_ref: "local-allium-view-library".to_string(),
            scope: ViewShapeScope::MotherUser,
            version: 1,
            active: true,
            major_mode: MajorMode::Table,
            minor_modes: vec![MinorMode::Pinned],
            maturity: ViewShapeMaturity::Exploratory,
            payload_contract: PayloadContract::FramedJson,
            payload_version: 1,
            vision_id: None,
            project_uid: None,
            replaced_by: None,
            requirements: creation.requirements.clone(),
        };

        let detail = ViewRequestDetail::from_parts(
            request,
            None,
            None,
            None,
            Some(creation),
            Some(created_shape),
            None,
        );

        assert_eq!(detail.available_actions.len(), 1);
        assert_eq!(
            detail.available_actions[0].kind,
            ViewRequestActionKind::OpenCreatedShape
        );
        assert_eq!(
            detail
                .linked_action_for_shape(Some("initial::req_1::test"))
                .map(|action| action.label.as_str()),
            Some("Open created shape")
        );
        assert!(detail
            .linked_action_for_shape(Some("unlinked.shape"))
            .is_none());
    }

    #[test]
    fn buffer_state_connectability_matches_allium_lifecycle() {
        assert!(BufferState::Live.is_connectable());
        assert!(BufferState::Stale.is_connectable());
        assert!(BufferState::Blocked.is_connectable());
        assert!(!BufferState::Replaced.is_connectable());
        assert!(!BufferState::Killed.is_connectable());
    }
}
