use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::state::{
    ChildRegistryEntryUpdate, ChildRegistrySourceRecord, ChildRegistryStore, MotherRuntimeStore,
};

pub trait ChildRegistryProvider: Send + Sync {
    fn kind(&self) -> &'static str;
    fn sync(&self, source: &ChildRegistrySourceRecord) -> Result<Vec<DiscoveredChildRelease>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredChildRelease {
    pub child_name: String,
    pub version: String,
    pub source_release_ref: String,
    pub artifact_url: String,
    pub manifest_url: String,
    pub checksums_url: Option<String>,
    pub artifact_sha256: String,
    pub manifest_sha256: String,
    pub signature_ref: Option<String>,
    pub patina_min: Option<String>,
    pub operations_json: Option<String>,
    pub needs_toys_json: Option<String>,
    pub needs_scopes_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSyncReport {
    pub source_id: String,
    pub provider_kind: String,
    pub discovered_count: usize,
    pub upserted_count: usize,
    pub skipped_count: usize,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ChildRegistrySyncEngine {
    store: ChildRegistryStore,
}

impl ChildRegistrySyncEngine {
    pub fn new(runtime_store: MotherRuntimeStore) -> Self {
        Self {
            store: runtime_store.child_registry_store(),
        }
    }

    pub fn sync_source(
        &self,
        source_id: &str,
        provider: &dyn ChildRegistryProvider,
    ) -> Result<SourceSyncReport> {
        let source = self
            .store
            .get_source(source_id)?
            .ok_or_else(|| anyhow::anyhow!("unknown child registry source '{}'", source_id))?;

        if source.provider_kind != provider.kind() {
            anyhow::bail!(
                "provider kind mismatch for source '{}': source is '{}' but provider is '{}'",
                source_id,
                source.provider_kind,
                provider.kind()
            );
        }

        if !source.enabled {
            self.store
                .set_source_sync_status(source_id, "skipped_disabled", None)?;
            return Ok(SourceSyncReport {
                source_id: source_id.to_string(),
                provider_kind: source.provider_kind,
                discovered_count: 0,
                upserted_count: 0,
                skipped_count: 0,
                status: "skipped_disabled".to_string(),
            });
        }

        let discovered = match provider.sync(&source) {
            Ok(discovered) => discovered,
            Err(error) => {
                let message = error.to_string();
                let _ = self
                    .store
                    .set_source_sync_status(source_id, "failed", Some(&message));
                return Err(error).with_context(|| format!("syncing source '{}'", source_id));
            }
        };

        let mut upserted_count = 0usize;
        let mut skipped_count = 0usize;

        for release in &discovered {
            if release.artifact_sha256.trim().is_empty()
                || release.manifest_sha256.trim().is_empty()
            {
                skipped_count += 1;
                continue;
            }

            let existing = self
                .store
                .get_entry_by_child_version(&release.child_name, &release.version)?;

            let (entry_id, state, state_reason) = match existing {
                Some(existing) => (existing.entry_id, existing.state, existing.state_reason),
                None => (
                    entry_id_for_release(source_id, &release.child_name, &release.version),
                    "candidate".to_string(),
                    Some(format!("newly discovered from source '{}'", source_id)),
                ),
            };

            let update = ChildRegistryEntryUpdate {
                entry_id,
                child_name: release.child_name.clone(),
                version: release.version.clone(),
                source_id: source_id.to_string(),
                source_release_ref: release.source_release_ref.clone(),
                artifact_url: release.artifact_url.clone(),
                manifest_url: release.manifest_url.clone(),
                checksums_url: release.checksums_url.clone(),
                artifact_sha256: release.artifact_sha256.clone(),
                manifest_sha256: release.manifest_sha256.clone(),
                signature_ref: release.signature_ref.clone(),
                patina_min: release.patina_min.clone(),
                operations_json: release.operations_json.clone(),
                needs_toys_json: release.needs_toys_json.clone(),
                needs_scopes_json: release.needs_scopes_json.clone(),
                state,
                state_reason,
            };

            self.store.upsert_entry(&update).with_context(|| {
                format!(
                    "upserting child registry entry '{}@{}'",
                    release.child_name, release.version
                )
            })?;
            upserted_count += 1;
        }

        self.store
            .set_source_sync_status(source_id, "success", None)
            .with_context(|| format!("updating sync status for source '{}'", source_id))?;

        Ok(SourceSyncReport {
            source_id: source_id.to_string(),
            provider_kind: source.provider_kind,
            discovered_count: discovered.len(),
            upserted_count,
            skipped_count,
            status: "success".to_string(),
        })
    }
}

fn entry_id_for_release(source_id: &str, child_name: &str, version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_id.as_bytes());
    hasher.update(b"::");
    hasher.update(child_name.as_bytes());
    hasher.update(b"::");
    hasher.update(version.as_bytes());
    let digest = hasher.finalize();
    let short = digest[..12]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    format!("entry_{}", short)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ChildRegistrySourceUpdate;

    #[derive(Debug)]
    struct FakeProvider {
        kind: &'static str,
        result: std::result::Result<Vec<DiscoveredChildRelease>, anyhow::Error>,
    }

    impl ChildRegistryProvider for FakeProvider {
        fn kind(&self) -> &'static str {
            self.kind
        }

        fn sync(&self, _source: &ChildRegistrySourceRecord) -> Result<Vec<DiscoveredChildRelease>> {
            match &self.result {
                Ok(items) => Ok(items.clone()),
                Err(error) => Err(anyhow::anyhow!(error.to_string())),
            }
        }
    }

    fn temp_store() -> MotherRuntimeStore {
        let path = std::env::temp_dir().join(format!(
            "patina-mother-child-registry-sync-{}.db",
            uuid::Uuid::new_v4()
        ));
        MotherRuntimeStore::new(path)
    }

    #[test]
    fn sync_source_upserts_discovered_releases_and_marks_success() {
        let store = temp_store();
        store
            .upsert_child_registry_source(&ChildRegistrySourceUpdate {
                source_id: "src_github_slate".to_string(),
                provider_kind: "github".to_string(),
                provider_config_json: r#"{"owner":"NicabarNimble","repo":"patina-child-slate"}"#
                    .to_string(),
                enabled: true,
            })
            .unwrap();

        let engine = ChildRegistrySyncEngine::new(store.clone());
        let provider = FakeProvider {
            kind: "github",
            result: Ok(vec![DiscoveredChildRelease {
                child_name: "slate-manager".to_string(),
                version: "0.2.0".to_string(),
                source_release_ref: "v0.2.0".to_string(),
                artifact_url: "https://example.invalid/slate.wasm".to_string(),
                manifest_url: "https://example.invalid/child.toml".to_string(),
                checksums_url: Some("https://example.invalid/checksums.txt".to_string()),
                artifact_sha256: "a3d24c4036f88fe4ca64f70556f0eae2e4ef6f878b6c51481e4a4e5c4b2b8f66"
                    .to_string(),
                manifest_sha256: "c26bcdf6529d8adf4ceac76714566491582f59d0bc889ef9e4d8ce96aa95f4c4"
                    .to_string(),
                signature_ref: None,
                patina_min: Some("0.64.4".to_string()),
                operations_json: None,
                needs_toys_json: None,
                needs_scopes_json: None,
            }]),
        };

        let report = engine.sync_source("src_github_slate", &provider).unwrap();
        assert_eq!(report.status, "success");
        assert_eq!(report.discovered_count, 1);
        assert_eq!(report.upserted_count, 1);

        let entries = store
            .list_child_registry_entries(Some("slate-manager"))
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, "0.2.0");
        assert_eq!(entries[0].state, "candidate");

        let source = store
            .get_child_registry_source("src_github_slate")
            .unwrap()
            .expect("source row");
        assert_eq!(source.last_sync_status.as_deref(), Some("success"));
        assert!(source.last_error.is_none());
    }

    #[test]
    fn sync_source_marks_failed_status_when_provider_errors() {
        let store = temp_store();
        store
            .upsert_child_registry_source(&ChildRegistrySourceUpdate {
                source_id: "src_github_slate".to_string(),
                provider_kind: "github".to_string(),
                provider_config_json: r#"{"owner":"NicabarNimble","repo":"patina-child-slate"}"#
                    .to_string(),
                enabled: true,
            })
            .unwrap();

        let engine = ChildRegistrySyncEngine::new(store.clone());
        let provider = FakeProvider {
            kind: "github",
            result: Err(anyhow::anyhow!("GitHub API returned 403")),
        };

        let error = engine
            .sync_source("src_github_slate", &provider)
            .unwrap_err();
        assert!(format!("{:#}", error).contains("GitHub API returned 403"));

        let source = store
            .get_child_registry_source("src_github_slate")
            .unwrap()
            .expect("source row");
        assert_eq!(source.last_sync_status.as_deref(), Some("failed"));
        assert!(source
            .last_error
            .unwrap_or_default()
            .contains("GitHub API returned 403"));
    }
}
