use super::*;
use rusqlite::{params, OptionalExtension};

#[derive(Debug, Clone)]
pub struct ChildRegistryStore {
    runtime: MotherRuntimeStore,
}

impl ChildRegistryStore {
    pub fn new(runtime: MotherRuntimeStore) -> Self {
        Self { runtime }
    }

    pub fn upsert_source(&self, update: &ChildRegistrySourceUpdate) -> Result<()> {
        let conn = self.runtime.open()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO mother_child_sources (
                source_id,
                provider_kind,
                provider_config_json,
                enabled,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?5)
            ON CONFLICT(source_id) DO UPDATE SET
                provider_kind = excluded.provider_kind,
                provider_config_json = excluded.provider_config_json,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at
            "#,
            params![
                &update.source_id,
                &update.provider_kind,
                &update.provider_config_json,
                if update.enabled { 1 } else { 0 },
                now,
            ],
        )?;

        Ok(())
    }

    pub fn get_source(&self, source_id: &str) -> Result<Option<ChildRegistrySourceRecord>> {
        let conn = self.runtime.open()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT source_id, provider_kind, provider_config_json, enabled,
                   last_sync_at, last_sync_status, last_error,
                   created_at, updated_at
            FROM mother_child_sources
            WHERE source_id = ?1
            "#,
        )?;

        let row = stmt
            .query_row(params![source_id], |row| {
                Ok(ChildRegistrySourceRecord {
                    source_id: row.get(0)?,
                    provider_kind: row.get(1)?,
                    provider_config_json: row.get(2)?,
                    enabled: row.get::<_, i64>(3)? == 1,
                    last_sync_at: row.get(4)?,
                    last_sync_status: row.get(5)?,
                    last_error: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .optional()?;

        Ok(row)
    }

    pub fn list_sources(&self) -> Result<Vec<ChildRegistrySourceRecord>> {
        let conn = self.runtime.open()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT source_id, provider_kind, provider_config_json, enabled,
                   last_sync_at, last_sync_status, last_error,
                   created_at, updated_at
            FROM mother_child_sources
            ORDER BY updated_at DESC, source_id ASC
            "#,
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ChildRegistrySourceRecord {
                    source_id: row.get(0)?,
                    provider_kind: row.get(1)?,
                    provider_config_json: row.get(2)?,
                    enabled: row.get::<_, i64>(3)? == 1,
                    last_sync_at: row.get(4)?,
                    last_sync_status: row.get(5)?,
                    last_error: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn set_source_sync_status(
        &self,
        source_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        let conn = self.runtime.open()?;
        let now = Utc::now().to_rfc3339();
        let changed = conn.execute(
            r#"
            UPDATE mother_child_sources
            SET last_sync_at = ?2,
                last_sync_status = ?3,
                last_error = ?4,
                updated_at = ?2
            WHERE source_id = ?1
            "#,
            params![source_id, now, status, error_message],
        )?;

        if changed == 0 {
            anyhow::bail!("unknown child registry source '{}'", source_id);
        }

        Ok(())
    }

    pub fn upsert_entry(&self, update: &ChildRegistryEntryUpdate) -> Result<()> {
        let conn = self.runtime.open()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO mother_child_registry_entries (
                entry_id,
                child_name,
                version,
                source_id,
                source_release_ref,
                artifact_url,
                manifest_url,
                checksums_url,
                artifact_sha256,
                manifest_sha256,
                signature_ref,
                patina_min,
                operations_json,
                needs_toys_json,
                needs_scopes_json,
                state,
                state_reason,
                created_at,
                updated_at
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?18
            )
            ON CONFLICT(entry_id) DO UPDATE SET
                child_name = excluded.child_name,
                version = excluded.version,
                source_id = excluded.source_id,
                source_release_ref = excluded.source_release_ref,
                artifact_url = excluded.artifact_url,
                manifest_url = excluded.manifest_url,
                checksums_url = excluded.checksums_url,
                artifact_sha256 = excluded.artifact_sha256,
                manifest_sha256 = excluded.manifest_sha256,
                signature_ref = excluded.signature_ref,
                patina_min = excluded.patina_min,
                operations_json = excluded.operations_json,
                needs_toys_json = excluded.needs_toys_json,
                needs_scopes_json = excluded.needs_scopes_json,
                state = excluded.state,
                state_reason = excluded.state_reason,
                updated_at = excluded.updated_at
            "#,
            params![
                &update.entry_id,
                &update.child_name,
                &update.version,
                &update.source_id,
                &update.source_release_ref,
                &update.artifact_url,
                &update.manifest_url,
                update.checksums_url.as_deref(),
                &update.artifact_sha256,
                &update.manifest_sha256,
                update.signature_ref.as_deref(),
                update.patina_min.as_deref(),
                update.operations_json.as_deref(),
                update.needs_toys_json.as_deref(),
                update.needs_scopes_json.as_deref(),
                &update.state,
                update.state_reason.as_deref(),
                now,
            ],
        )?;

        Ok(())
    }

    pub fn set_entry_state(&self, entry_id: &str, state: &str, reason: Option<&str>) -> Result<()> {
        let conn = self.runtime.open()?;
        let now = Utc::now().to_rfc3339();
        let changed = conn.execute(
            r#"
            UPDATE mother_child_registry_entries
            SET state = ?2,
                state_reason = ?3,
                updated_at = ?4
            WHERE entry_id = ?1
            "#,
            params![entry_id, state, reason, now],
        )?;

        if changed == 0 {
            anyhow::bail!("unknown child registry entry '{}'", entry_id);
        }

        Ok(())
    }

    pub fn get_entry_by_child_version(
        &self,
        child_name: &str,
        version: &str,
    ) -> Result<Option<ChildRegistryEntryRecord>> {
        let conn = self.runtime.open()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT entry_id, child_name, version, source_id, source_release_ref,
                   artifact_url, manifest_url, checksums_url,
                   artifact_sha256, manifest_sha256,
                   signature_ref, patina_min,
                   operations_json, needs_toys_json, needs_scopes_json,
                   state, state_reason, created_at, updated_at
            FROM mother_child_registry_entries
            WHERE child_name = ?1 AND version = ?2
            LIMIT 1
            "#,
        )?;

        let row = stmt
            .query_row(params![child_name, version], |row| {
                Ok(ChildRegistryEntryRecord {
                    entry_id: row.get(0)?,
                    child_name: row.get(1)?,
                    version: row.get(2)?,
                    source_id: row.get(3)?,
                    source_release_ref: row.get(4)?,
                    artifact_url: row.get(5)?,
                    manifest_url: row.get(6)?,
                    checksums_url: row.get(7)?,
                    artifact_sha256: row.get(8)?,
                    manifest_sha256: row.get(9)?,
                    signature_ref: row.get(10)?,
                    patina_min: row.get(11)?,
                    operations_json: row.get(12)?,
                    needs_toys_json: row.get(13)?,
                    needs_scopes_json: row.get(14)?,
                    state: row.get(15)?,
                    state_reason: row.get(16)?,
                    created_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            })
            .optional()?;

        Ok(row)
    }

    pub fn list_entries(&self, child_name: Option<&str>) -> Result<Vec<ChildRegistryEntryRecord>> {
        let conn = self.runtime.open()?;

        if let Some(child_name) = child_name {
            let mut stmt = conn.prepare(
                r#"
                SELECT entry_id, child_name, version, source_id, source_release_ref,
                       artifact_url, manifest_url, checksums_url,
                       artifact_sha256, manifest_sha256,
                       signature_ref, patina_min,
                       operations_json, needs_toys_json, needs_scopes_json,
                       state, state_reason, created_at, updated_at
                FROM mother_child_registry_entries
                WHERE child_name = ?1
                ORDER BY updated_at DESC, version DESC
                "#,
            )?;
            let rows = stmt
                .query_map(params![child_name], |row| {
                    Ok(ChildRegistryEntryRecord {
                        entry_id: row.get(0)?,
                        child_name: row.get(1)?,
                        version: row.get(2)?,
                        source_id: row.get(3)?,
                        source_release_ref: row.get(4)?,
                        artifact_url: row.get(5)?,
                        manifest_url: row.get(6)?,
                        checksums_url: row.get(7)?,
                        artifact_sha256: row.get(8)?,
                        manifest_sha256: row.get(9)?,
                        signature_ref: row.get(10)?,
                        patina_min: row.get(11)?,
                        operations_json: row.get(12)?,
                        needs_toys_json: row.get(13)?,
                        needs_scopes_json: row.get(14)?,
                        state: row.get(15)?,
                        state_reason: row.get(16)?,
                        created_at: row.get(17)?,
                        updated_at: row.get(18)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            return Ok(rows);
        }

        let mut stmt = conn.prepare(
            r#"
            SELECT entry_id, child_name, version, source_id, source_release_ref,
                   artifact_url, manifest_url, checksums_url,
                   artifact_sha256, manifest_sha256,
                   signature_ref, patina_min,
                   operations_json, needs_toys_json, needs_scopes_json,
                   state, state_reason, created_at, updated_at
            FROM mother_child_registry_entries
            ORDER BY child_name ASC, updated_at DESC, version DESC
            "#,
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ChildRegistryEntryRecord {
                    entry_id: row.get(0)?,
                    child_name: row.get(1)?,
                    version: row.get(2)?,
                    source_id: row.get(3)?,
                    source_release_ref: row.get(4)?,
                    artifact_url: row.get(5)?,
                    manifest_url: row.get(6)?,
                    checksums_url: row.get(7)?,
                    artifact_sha256: row.get(8)?,
                    manifest_sha256: row.get(9)?,
                    signature_ref: row.get(10)?,
                    patina_min: row.get(11)?,
                    operations_json: row.get(12)?,
                    needs_toys_json: row.get(13)?,
                    needs_scopes_json: row.get(14)?,
                    state: row.get(15)?,
                    state_reason: row.get(16)?,
                    created_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn upsert_install(&self, update: &ChildInstallUpdate) -> Result<()> {
        let conn = self.runtime.open()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO mother_child_installs (
                install_id,
                entry_id,
                installed_name,
                installed_version,
                wasm_path,
                manifest_path,
                artifact_sha256_verified,
                manifest_sha256_verified,
                installed_at,
                installed_by,
                status,
                last_error,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?9)
            ON CONFLICT(install_id) DO UPDATE SET
                entry_id = excluded.entry_id,
                installed_name = excluded.installed_name,
                installed_version = excluded.installed_version,
                wasm_path = excluded.wasm_path,
                manifest_path = excluded.manifest_path,
                artifact_sha256_verified = excluded.artifact_sha256_verified,
                manifest_sha256_verified = excluded.manifest_sha256_verified,
                installed_by = excluded.installed_by,
                status = excluded.status,
                last_error = excluded.last_error,
                updated_at = excluded.updated_at
            "#,
            params![
                &update.install_id,
                &update.entry_id,
                &update.installed_name,
                &update.installed_version,
                &update.wasm_path,
                &update.manifest_path,
                &update.artifact_sha256_verified,
                &update.manifest_sha256_verified,
                now,
                update.installed_by.as_deref(),
                &update.status,
                update.last_error.as_deref(),
            ],
        )?;

        Ok(())
    }

    pub fn list_installs(&self, child_name: Option<&str>) -> Result<Vec<ChildInstallRecord>> {
        let conn = self.runtime.open()?;

        if let Some(child_name) = child_name {
            let mut stmt = conn.prepare(
                r#"
                SELECT install_id, entry_id, installed_name, installed_version,
                       wasm_path, manifest_path,
                       artifact_sha256_verified, manifest_sha256_verified,
                       installed_at, installed_by, status, last_error, updated_at
                FROM mother_child_installs
                WHERE installed_name = ?1
                ORDER BY installed_at DESC
                "#,
            )?;
            let rows = stmt
                .query_map(params![child_name], |row| {
                    Ok(ChildInstallRecord {
                        install_id: row.get(0)?,
                        entry_id: row.get(1)?,
                        installed_name: row.get(2)?,
                        installed_version: row.get(3)?,
                        wasm_path: row.get(4)?,
                        manifest_path: row.get(5)?,
                        artifact_sha256_verified: row.get(6)?,
                        manifest_sha256_verified: row.get(7)?,
                        installed_at: row.get(8)?,
                        installed_by: row.get(9)?,
                        status: row.get(10)?,
                        last_error: row.get(11)?,
                        updated_at: row.get(12)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            return Ok(rows);
        }

        let mut stmt = conn.prepare(
            r#"
            SELECT install_id, entry_id, installed_name, installed_version,
                   wasm_path, manifest_path,
                   artifact_sha256_verified, manifest_sha256_verified,
                   installed_at, installed_by, status, last_error, updated_at
            FROM mother_child_installs
            ORDER BY installed_name ASC, installed_at DESC
            "#,
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ChildInstallRecord {
                    install_id: row.get(0)?,
                    entry_id: row.get(1)?,
                    installed_name: row.get(2)?,
                    installed_version: row.get(3)?,
                    wasm_path: row.get(4)?,
                    manifest_path: row.get(5)?,
                    artifact_sha256_verified: row.get(6)?,
                    manifest_sha256_verified: row.get(7)?,
                    installed_at: row.get(8)?,
                    installed_by: row.get(9)?,
                    status: row.get(10)?,
                    last_error: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn upsert_project_assignment(&self, update: &ProjectChildAssignmentUpdate) -> Result<()> {
        let conn = self.runtime.open()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO mother_project_child_assignments (
                assignment_id,
                project_uid,
                project_id,
                child_name,
                entry_id,
                pinned_version,
                status,
                reason,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
            ON CONFLICT(assignment_id) DO UPDATE SET
                project_uid = excluded.project_uid,
                project_id = excluded.project_id,
                child_name = excluded.child_name,
                entry_id = excluded.entry_id,
                pinned_version = excluded.pinned_version,
                status = excluded.status,
                reason = excluded.reason,
                updated_at = excluded.updated_at
            "#,
            params![
                &update.assignment_id,
                &update.project_uid,
                update.project_id.as_deref(),
                &update.child_name,
                &update.entry_id,
                &update.pinned_version,
                &update.status,
                update.reason.as_deref(),
                now,
            ],
        )?;

        Ok(())
    }

    pub fn list_project_assignments(
        &self,
        project_uid: Option<&str>,
    ) -> Result<Vec<ProjectChildAssignmentRecord>> {
        let conn = self.runtime.open()?;

        if let Some(project_uid) = project_uid {
            let mut stmt = conn.prepare(
                r#"
                SELECT assignment_id, project_uid, project_id, child_name, entry_id,
                       pinned_version, status, reason, created_at, updated_at
                FROM mother_project_child_assignments
                WHERE project_uid = ?1
                ORDER BY updated_at DESC, child_name ASC
                "#,
            )?;
            let rows = stmt
                .query_map(params![project_uid], |row| {
                    Ok(ProjectChildAssignmentRecord {
                        assignment_id: row.get(0)?,
                        project_uid: row.get(1)?,
                        project_id: row.get(2)?,
                        child_name: row.get(3)?,
                        entry_id: row.get(4)?,
                        pinned_version: row.get(5)?,
                        status: row.get(6)?,
                        reason: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            return Ok(rows);
        }

        let mut stmt = conn.prepare(
            r#"
            SELECT assignment_id, project_uid, project_id, child_name, entry_id,
                   pinned_version, status, reason, created_at, updated_at
            FROM mother_project_child_assignments
            ORDER BY project_uid ASC, updated_at DESC, child_name ASC
            "#,
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ProjectChildAssignmentRecord {
                    assignment_id: row.get(0)?,
                    project_uid: row.get(1)?,
                    project_id: row.get(2)?,
                    child_name: row.get(3)?,
                    entry_id: row.get(4)?,
                    pinned_version: row.get(5)?,
                    status: row.get(6)?,
                    reason: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }
}

impl MotherRuntimeStore {
    pub fn child_registry_store(&self) -> ChildRegistryStore {
        ChildRegistryStore::new(self.clone())
    }

    pub fn upsert_child_registry_source(&self, update: &ChildRegistrySourceUpdate) -> Result<()> {
        self.child_registry_store().upsert_source(update)
    }

    pub fn get_child_registry_source(
        &self,
        source_id: &str,
    ) -> Result<Option<ChildRegistrySourceRecord>> {
        self.child_registry_store().get_source(source_id)
    }

    pub fn list_child_registry_sources(&self) -> Result<Vec<ChildRegistrySourceRecord>> {
        self.child_registry_store().list_sources()
    }

    pub fn set_child_registry_source_sync_status(
        &self,
        source_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.child_registry_store()
            .set_source_sync_status(source_id, status, error_message)
    }

    pub fn upsert_child_registry_entry(&self, update: &ChildRegistryEntryUpdate) -> Result<()> {
        self.child_registry_store().upsert_entry(update)
    }

    pub fn set_child_registry_entry_state(
        &self,
        entry_id: &str,
        state: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        self.child_registry_store()
            .set_entry_state(entry_id, state, reason)
    }

    pub fn get_child_registry_entry_by_child_version(
        &self,
        child_name: &str,
        version: &str,
    ) -> Result<Option<ChildRegistryEntryRecord>> {
        self.child_registry_store()
            .get_entry_by_child_version(child_name, version)
    }

    pub fn list_child_registry_entries(
        &self,
        child_name: Option<&str>,
    ) -> Result<Vec<ChildRegistryEntryRecord>> {
        self.child_registry_store().list_entries(child_name)
    }

    pub fn upsert_child_install(&self, update: &ChildInstallUpdate) -> Result<()> {
        self.child_registry_store().upsert_install(update)
    }

    pub fn list_child_installs(&self, child_name: Option<&str>) -> Result<Vec<ChildInstallRecord>> {
        self.child_registry_store().list_installs(child_name)
    }

    pub fn upsert_project_child_assignment(
        &self,
        update: &ProjectChildAssignmentUpdate,
    ) -> Result<()> {
        self.child_registry_store()
            .upsert_project_assignment(update)
    }

    pub fn list_project_child_assignments(
        &self,
        project_uid: Option<&str>,
    ) -> Result<Vec<ProjectChildAssignmentRecord>> {
        self.child_registry_store()
            .list_project_assignments(project_uid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRegistrySourceRecord {
    pub source_id: String,
    pub provider_kind: String,
    pub provider_config_json: String,
    pub enabled: bool,
    pub last_sync_at: Option<String>,
    pub last_sync_status: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRegistrySourceUpdate {
    pub source_id: String,
    pub provider_kind: String,
    pub provider_config_json: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRegistryEntryRecord {
    pub entry_id: String,
    pub child_name: String,
    pub version: String,
    pub source_id: String,
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
    pub state: String,
    pub state_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRegistryEntryUpdate {
    pub entry_id: String,
    pub child_name: String,
    pub version: String,
    pub source_id: String,
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
    pub state: String,
    pub state_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildInstallRecord {
    pub install_id: String,
    pub entry_id: String,
    pub installed_name: String,
    pub installed_version: String,
    pub wasm_path: String,
    pub manifest_path: String,
    pub artifact_sha256_verified: String,
    pub manifest_sha256_verified: String,
    pub installed_at: String,
    pub installed_by: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildInstallUpdate {
    pub install_id: String,
    pub entry_id: String,
    pub installed_name: String,
    pub installed_version: String,
    pub wasm_path: String,
    pub manifest_path: String,
    pub artifact_sha256_verified: String,
    pub manifest_sha256_verified: String,
    pub installed_by: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChildAssignmentRecord {
    pub assignment_id: String,
    pub project_uid: String,
    pub project_id: Option<String>,
    pub child_name: String,
    pub entry_id: String,
    pub pinned_version: String,
    pub status: String,
    pub reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChildAssignmentUpdate {
    pub assignment_id: String,
    pub project_uid: String,
    pub project_id: Option<String>,
    pub child_name: String,
    pub entry_id: String,
    pub pinned_version: String,
    pub status: String,
    pub reason: Option<String>,
}
