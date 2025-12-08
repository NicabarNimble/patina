//! Mothership client for Patina
//!
//! Provides HTTP client for communicating with a running `patina serve` daemon.
//! Used by containers to query the Mac mothership.
//!
//! # Environment Variable
//! Set `PATINA_MOTHERSHIP=host:port` to enable remote queries.
//! Example: `PATINA_MOTHERSHIP=host.docker.internal:50051`

mod internal;

use anyhow::Result;

pub use internal::{Client, ScryRequest, ScryResponse, ScryResultJson};

/// Default port for mothership daemon
pub const DEFAULT_PORT: u16 = 50051;

/// Environment variable for mothership address
pub const ENV_MOTHERSHIP: &str = "PATINA_MOTHERSHIP";

/// Check if mothership is configured via environment
pub fn is_configured() -> bool {
    std::env::var(ENV_MOTHERSHIP).is_ok()
}

/// Get the mothership address from environment
/// Returns None if not configured
pub fn get_address() -> Option<String> {
    std::env::var(ENV_MOTHERSHIP).ok()
}

/// Create a client connected to the configured mothership
/// Returns None if PATINA_MOTHERSHIP is not set
pub fn connect() -> Option<Client> {
    get_address().map(Client::new)
}

/// Check if the mothership is reachable (health check)
pub fn is_available() -> bool {
    if let Some(client) = connect() {
        client.health().is_ok()
    } else {
        false
    }
}

/// Query the mothership with scry
/// Returns Err if mothership is not configured or unreachable
pub fn scry(request: ScryRequest) -> Result<ScryResponse> {
    let client = connect().ok_or_else(|| anyhow::anyhow!("PATINA_MOTHERSHIP not set"))?;
    client.scry(request)
}
