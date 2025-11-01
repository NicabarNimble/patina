//! Vector search types and utilities

/// Vector tables in the database
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorTable {
    Beliefs,
    Observations,
}

impl VectorTable {
    pub fn table_name(&self) -> &'static str {
        match self {
            VectorTable::Beliefs => "belief_vectors",
            VectorTable::Observations => "observation_vectors",
        }
    }
}

/// Filter for vector search
#[derive(Debug, Clone)]
pub struct VectorFilter {
    pub field: String,
    pub value: String,
}

/// Result of a vector search
#[derive(Debug, Clone)]
pub struct VectorMatch {
    pub row_id: i64,
    pub distance: f32,
    pub similarity: f32,
}

impl VectorMatch {
    pub fn new(row_id: i64, distance: f32) -> Self {
        Self {
            row_id,
            distance,
            similarity: 1.0 - distance, // Cosine distance to similarity
        }
    }
}
