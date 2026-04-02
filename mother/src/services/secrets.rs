use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::ChildHealth;

const DEFAULT_TTL_SECS: u64 = 600;

struct CacheEntry {
    secrets: HashMap<String, String>,
    expires_at: Instant,
}

pub struct SecretsService {
    cache: Mutex<Option<CacheEntry>>,
}

impl Default for SecretsService {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretsService {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(None),
        }
    }

    pub fn health(&self) -> ChildHealth {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        match cache.as_ref() {
            Some(entry) if entry.expires_at > Instant::now() => ChildHealth::Healthy,
            Some(_) => ChildHealth::Degraded("cache expired".into()),
            None => ChildHealth::Healthy,
        }
    }

    pub fn get(&self) -> Result<serde_json::Value> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        match cache.as_ref() {
            Some(entry) if entry.expires_at > Instant::now() => {
                Ok(serde_json::to_value(&entry.secrets)?)
            }
            _ => bail!("no cached secrets"),
        }
    }

    pub fn cache(&self, payload: &serde_json::Value) -> Result<serde_json::Value> {
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
        let ttl = Duration::from_secs(req.ttl_secs);

        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        *cache = Some(CacheEntry {
            secrets: req.secrets,
            expires_at: Instant::now() + ttl,
        });

        Ok(serde_json::json!({"status": "cached"}))
    }

    pub fn lock(&self) -> serde_json::Value {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        *cache = None;
        serde_json::json!({"status": "locked"})
    }
}
