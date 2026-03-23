use anyhow::Result;

pub use mother_crate::events::{ack_through, list_streams};
use mother_crate::PendingEvent;

pub fn pull(stream: &str, after_offset: Option<u64>, limit: u32) -> Result<Vec<PendingEvent>> {
    let conn = crate::eventlog::open_events_db()?;
    mother_crate::events::pull(&conn, stream, after_offset, limit)
}
