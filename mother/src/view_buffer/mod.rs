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

pub use catalog::{DataCatalog, MotherStatusFacts, MOTHER_STATUS_SHAPE_ID};
pub use model::{
    Buffer, BufferState, CataloguedFact, CataloguedSource, CataloguedSourceKind, DisplayRequest,
    DisplayRequestOutcome, FactKind, Frame, FrameKind, MajorMode, MinorMode, ObservabilityGap,
    ObservabilityGapStatus, ObservationState, PayloadContract, ShapeMatch, ShapeMatchKind,
    SourceAvailability, ViewRequirement, ViewShape, ViewShapeMaturity, ViewShapeScope, Window,
    WindowConnectionState,
};
pub use payload::{FramedJsonPayload, PayloadFrame};
pub use service::{
    mother_status_shape, ConnectWindowRequest, DisconnectWindowRequest, KillBufferRequest,
    OpenBufferOutcome, OpenBufferRequest, OpenedBuffer, ViewBufferService,
};
