//! MCP (Model Context Protocol) server
//!
//! JSON-RPC 2.0 over stdio. No external SDK - blocking I/O, minimal dependencies.

mod protocol;
mod server;

pub use server::run_mcp_server;
