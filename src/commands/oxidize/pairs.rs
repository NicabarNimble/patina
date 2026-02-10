//! Training pair generators for projections
//!
//! Generate (anchor, positive, negative) triplets from eventlog for contrastive learning

/// A training triplet: anchor, positive (similar), negative (dissimilar)
#[derive(Debug, Clone)]
pub struct TrainingPair {
    pub anchor: String,
    pub positive: String,
    pub negative: String,
}
