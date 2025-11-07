//! Storage layer for Patina - SQLite + USearch hybrid storage
//!
//! This module implements dual storage strategy:
//! - SQLite for structured data (events, metadata, relational queries)
//! - USearch for vector similarity search (ANN via HNSW indices)
//!
//! # Architecture
//!
//! Each domain (beliefs, patterns, code symbols) has its own storage wrapper
//! that owns both SQLite and USearch backends. No abstraction layer - direct
//! library usage for maximum performance and simplicity.
//!
//! # Example
//!
//! ```no_run
//! use patina::storage::BeliefStorage;
//!
//! let mut storage = BeliefStorage::open(".patina/storage")?;
//! // SQLite handles metadata, USearch handles vector search
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod beliefs;
pub mod observations;
pub mod types;

pub use beliefs::BeliefStorage;
pub use observations::ObservationStorage;
pub use types::{Belief, BeliefMetadata, Observation, ObservationMetadata};
