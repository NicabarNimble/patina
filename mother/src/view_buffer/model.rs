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
    fn buffer_state_connectability_matches_allium_lifecycle() {
        assert!(BufferState::Live.is_connectable());
        assert!(BufferState::Stale.is_connectable());
        assert!(BufferState::Blocked.is_connectable());
        assert!(!BufferState::Replaced.is_connectable());
        assert!(!BufferState::Killed.is_connectable());
    }
}
