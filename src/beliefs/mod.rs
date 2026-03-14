//! Beliefs-facing module surface.
//!
//! This is a phase-A ownership facade for the mother-child-toy-beliefs reorg.
//! Belief runtime/storage internals are being moved here slice-by-slice.

pub mod belief_host;
pub mod graph;
pub mod graph_host;

pub use graph::{
    BeliefEntry, BeliefStatus, Edge, EdgeType, EdgeUsageStats, Graph, Node, NodeType, WeightChange,
    WeightLearningReport, DEFAULT_ALPHA, MIN_SAMPLES, WEIGHT_MAX, WEIGHT_MIN,
};
