//! Beliefs-facing module surface.
//!
//! This is a phase-A ownership facade for the mother-child-toy-beliefs reorg.
//! Existing belief runtime/storage internals still live behind `mother`/graph
//! today; this module provides one canonical beliefs entrypoint while we move
//! internals in later slices.

pub use crate::mother::{
    BeliefEntry, BeliefStatus, Edge, EdgeType, EdgeUsageStats, Graph, Node, NodeType, WeightChange,
    WeightLearningReport,
};
