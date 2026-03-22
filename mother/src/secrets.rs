//! Secrets cache child — caches decrypted secrets to avoid repeated Touch ID prompts.
//!
//! Actions:
//! - "get"   → return cached secrets (or error if expired/empty)
//! - "cache" → store secrets with TTL
//! - "lock"  → clear the cache

use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::{ChildHealth, ChildRequest, ChildResponse, MotherChild, MotherHost};

/// Default TTL for cached secrets (10 minutes)
const DEFAULT_TTL_SECS: u64 = 600;

struct CacheEntry {
    secrets: HashMap<String, String>,
    expires_at: Instant,
}

/// Secrets cache child — first MotherChild implementation.
pub struct SecretsCacheChild {
    cache: Mutex<Option<CacheEntry>>,
}

impl SecretsCacheChild {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(None),
        }
    }
}

impl MotherChild for SecretsCacheChild {
    fn name(&self) -> &str {
        "secrets"
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        match cache.as_ref() {
            Some(entry) if entry.expires_at > Instant::now() => ChildHealth::Healthy,
            Some(_) => ChildHealth::Degraded("cache expired".into()),
            None => ChildHealth::Healthy,
        }
    }

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
        match request.action.as_str() {
            "get" => self.handle_get(),
            "cache" => self.handle_cache(&request.payload),
            "lock" => self.handle_lock(),
            "health" => self.handle_health(),
            _ => bail!("secrets: unknown action '{}'", request.action),
        }
    }
}

impl SecretsCacheChild {
    fn handle_get(&self) -> Result<ChildResponse> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        match cache.as_ref() {
            Some(entry) if entry.expires_at > Instant::now() => Ok(ChildResponse {
                payload: serde_json::to_value(&entry.secrets)?,
            }),
            _ => bail!("no cached secrets"),
        }
    }

    fn handle_cache(&self, payload: &serde_json::Value) -> Result<ChildResponse> {
        #[derive(Deserialize)]
        struct CacheRequest {
            secrets: HashMap<String, String>,
            #[serde(default = "default_ttl")]
            ttl_secs: u64,
        }
        fn default_ttl() -> u64 {
            DEFAULT_TTL_SECS
        }

        let req: CacheRequest = serde_json::from_value(payload.clone())?;
        let ttl = std::time::Duration::from_secs(req.ttl_secs);

        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        *cache = Some(CacheEntry {
            secrets: req.secrets,
            expires_at: Instant::now() + ttl,
        });

        Ok(ChildResponse {
            payload: serde_json::json!({"status": "cached"}),
        })
    }

    fn handle_lock(&self) -> Result<ChildResponse> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        *cache = None;

        Ok(ChildResponse {
            payload: serde_json::json!({"status": "locked"}),
        })
    }

    fn handle_health(&self) -> Result<ChildResponse> {
        let status = match self.health() {
            ChildHealth::Healthy => "healthy",
            ChildHealth::Degraded(_) => "degraded",
            ChildHealth::Unhealthy(_) => "unhealthy",
        };

        Ok(ChildResponse {
            payload: serde_json::json!({"status": status}),
        })
    }
}
