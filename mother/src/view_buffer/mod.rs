//! Mother-owned Emacs-like view buffers.
//!
//! This module implements the Mother view-buffer runtime described by
//! `layer/allium/mother/mother-view-composer-target.allium` and scoped by
//! `mother-view-buffer-runtime`.

mod model;
mod payload;

pub use model::{
    Buffer, BufferState, CataloguedFact, CataloguedSource, CataloguedSourceKind, FactKind, Frame,
    FrameKind, MajorMode, MinorMode, ObservabilityGap, ObservabilityGapStatus, ObservationState,
    PayloadContract, ViewRequirement, ViewShape, ViewShapeScope, Window, WindowConnectionState,
};
pub use payload::{FramedJsonPayload, PayloadFrame};
