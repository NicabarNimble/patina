//! Mother-owned Emacs-like view buffers.
//!
//! This module implements the Mother view-buffer runtime described by
//! `layer/allium/mother/mother-view-composer-target.allium` and scoped by
//! `mother-view-buffer-runtime`.

mod catalog;
mod model;
mod payload;
mod service;
pub(crate) mod store;

pub use catalog::{
    DataCatalog, MarkdownArtifactFacts, MotherStatusFacts, MOTHER_STATUS_SHAPE_ID,
    PROJECT_README_FACT_PREFIX, PROJECT_README_SHAPE_ID, PROJECT_README_SOURCE_ID,
};
pub use model::{
    Buffer, BufferState, CataloguedFact, CataloguedSource, CataloguedSourceKind, DisplayPattern,
    DisplayPatternKind, DisplayRequest, DisplayRequestOutcome, FactKind, Frame, FrameKind,
    MajorMode, MatureViewArtifactRequest, MaturedViewArtifactOutcome, MinorMode, ObservabilityGap,
    ObservabilityGapStatus, ObservabilityImprovementArtifact, ObservationState, PayloadContract,
    ProposedObservabilityImprovement, ShapeMatch, ShapeMatchKind, SourceAvailability,
    ViewDerivation, ViewMaturationEvent, ViewMaturationOrigin, ViewMaturationTargetKind,
    ViewRequestAction, ViewRequestActionKind, ViewRequestDetail, ViewRequirement, ViewShape,
    ViewShapeAdaptation, ViewShapeCreation, ViewShapeMaturity, ViewShapeRevision,
    ViewShapeRevisionOrigin, ViewShapeRevisionState, ViewShapeScope, Window, WindowConnectionState,
};
pub use payload::{FramedJsonPayload, PayloadFrame};
pub use service::{
    builtin_view_shapes, mother_status_shape, readme_markdown_shape, ComposeViewRequest,
    ComposedViewRequest, ConnectWindowRequest, DisconnectWindowRequest, KillBufferRequest,
    LinkObservabilityGapRequest, OpenBufferOutcome, OpenBufferRequest, OpenRequestShapeOutcome,
    OpenRequestShapeRequest, OpenedBuffer, ProposedInitialShape, ProposedShapeMatch,
    ResolveObservabilityGapRequest, ReviseViewShapeRequest, RevisedViewShapeOutcome,
    ViewBufferService, SHAPE_MATCH_CONFIDENCE_THRESHOLD,
};
