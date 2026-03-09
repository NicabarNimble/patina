//! Embedding ID offsets — single source of truth
//!
//! USearch indexes store all content types in a single vector space.
//! Each domain gets a billion-wide ID slot so enrichment can determine
//! the content type from the key alone.
//!
//! Slot layout:
//!   0 ..  999_999_999  — eventlog entries (sessions, raw events)
//!   1B .. 1_999_999_999 — code facts (function_facts rowid)
//!   2B .. 2_999_999_999 — patterns (patterns rowid)
//!   3B .. 3_999_999_999 — commits (commits rowid)
//!   4B .. 4_999_999_999 — beliefs (beliefs rowid)
//!   5B .. 5_999_999_999 — connector events (eventlog seq for schema-declared facts)
//!
//! Next available slot: 6_000_000_000

pub const CODE_ID_OFFSET: i64 = 1_000_000_000;
pub const PATTERN_ID_OFFSET: i64 = 2_000_000_000;
pub const COMMIT_ID_OFFSET: i64 = 3_000_000_000;
pub const BELIEF_ID_OFFSET: i64 = 4_000_000_000;
/// Connector event offset. Named FORGE_ID_OFFSET for backward compatibility
/// with existing embedding indices (renaming would invalidate stored keys).
pub const FORGE_ID_OFFSET: i64 = 5_000_000_000;
