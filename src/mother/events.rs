use anyhow::Result;

use super::{KnowledgeRuntimeStore, PendingEvent};

pub use mother_crate::events::{KNOWN_STREAMS, ack_through, list_streams};

pub fn pull(stream: &str, after_offset: Option<u64>, limit: u32) -> Result<Vec<PendingEvent>> {
    let conn = crate::eventlog::open_events_db()?;
    mother_crate::events::pull(&conn, stream, after_offset, limit)
}
