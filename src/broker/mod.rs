//! Broker — routing engine for Mother.
//!
//! Routes facts from children to destination events.db files based on
//! sources.toml declarations. Manages child lifecycle (spawn, fetch,
//! shutdown) for native children via the pipe protocol.

pub mod connection;
pub mod http;
pub mod lifecycle;
pub mod routing;
pub mod sources;
pub mod spawn;
