//! Session caching for secrets.
//!
//! Caches decrypted secrets in the `patina serve` daemon to avoid
//! repeated Touch ID prompts within a session.
//!
//! Flow:
//! 1. `secrets run` checks serve for cached values
//! 2. Cache hit → use cached values (no Touch ID)
//! 3. Cache miss → decrypt locally (Touch ID), cache in serve
//! 4. serve not running → decrypt locally (Touch ID), no caching

use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

/// Default cache TTL in seconds (10 minutes).
const DEFAULT_TTL_SECS: u64 = 600;

/// Serve daemon base URL.
fn serve_url() -> String {
    // TODO: Make configurable via environment or config
    "http://127.0.0.1:50051".to_string()
}

/// Check if the serve daemon is running.
pub fn is_serve_running() -> bool {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    client
        .get(format!("{}/health", serve_url()))
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Try to get cached secrets from serve.
///
/// Returns None if serve is not running or cache is empty/expired.
pub fn get_cached_secrets() -> Option<HashMap<String, String>> {
    if !is_serve_running() {
        return None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;

    let response = client
        .get(format!("{}/secrets/cache", serve_url()))
        .send()
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    response.json().ok()
}

/// Store secrets in the serve cache.
///
/// Returns Ok(true) if cached, Ok(false) if serve not running.
pub fn cache_secrets(secrets: &HashMap<String, String>) -> Result<bool> {
    if !is_serve_running() {
        return Ok(false);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client
        .post(format!("{}/secrets/cache", serve_url()))
        .json(&CacheRequest {
            secrets: secrets.clone(),
            ttl_secs: DEFAULT_TTL_SECS,
        })
        .send()?;

    Ok(response.status().is_success())
}

/// Clear the secrets cache (lock).
///
/// Returns Ok(true) if cleared, Ok(false) if serve not running.
pub fn clear_cache() -> Result<bool> {
    if !is_serve_running() {
        return Ok(false);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client
        .post(format!("{}/secrets/lock", serve_url()))
        .send()?;

    Ok(response.status().is_success())
}

/// Request body for caching secrets.
#[derive(Debug, serde::Serialize)]
struct CacheRequest {
    secrets: HashMap<String, String>,
    ttl_secs: u64,
}

/// Get secrets with session caching.
///
/// 1. Try cache first (no Touch ID)
/// 2. On miss, call decrypt_fn (may trigger Touch ID)
/// 3. Cache result for next time
pub fn get_secrets_with_cache<F>(decrypt_fn: F) -> Result<HashMap<String, String>>
where
    F: FnOnce() -> Result<HashMap<String, String>>,
{
    // Try cache first
    if let Some(cached) = get_cached_secrets() {
        return Ok(cached);
    }

    // Decrypt (may trigger Touch ID)
    let secrets = decrypt_fn()?;

    // Cache for next time (ignore errors - caching is optional)
    let _ = cache_secrets(&secrets);

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serve_url() {
        assert!(serve_url().starts_with("http://"));
    }

    // Integration tests require serve to be running
    // #[test]
    // fn test_cache_roundtrip() {
    //     let mut secrets = HashMap::new();
    //     secrets.insert("test".to_string(), "value".to_string());
    //     cache_secrets(&secrets).unwrap();
    //     let cached = get_cached_secrets().unwrap();
    //     assert_eq!(cached.get("test"), Some(&"value".to_string()));
    //     clear_cache().unwrap();
    // }
}
