use serde::{Deserialize, Serialize};

pub const TOY_WIT_DIR: &str = env!("PATINA_SDK_DATA_WIT_DIR");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedLake {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LakeCursorRecord {
    pub source: String,
    pub data_type: String,
    pub cursor: Option<String>,
    pub written: u64,
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedConnectorBinding {
    pub binding_id: String,
    pub connection: String,
    pub owner: String,
    pub repo: String,
    pub types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorSyncResult {
    pub binding_id: String,
    pub data_type: String,
    pub cursor: Option<String>,
    pub rows_json: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubListParams {
    pub since: Option<String>,
    pub state: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubPage {
    pub items: String,
    pub has_next: bool,
    pub next_page: Option<u32>,
    pub rate_remaining: u32,
}

pub trait MeasureBackend {
    fn record(verb: &str, tool: &str, mode: &str, metrics_json: &str) -> Result<(), String>;
}

pub trait CheckpointBackend {
    fn load(stream: &str) -> Option<String>;
    fn save(stream: &str, checkpoint_json: &str) -> Result<(), String>;
}

pub trait LakeBackend {
    fn list_granted_lakes() -> Result<Vec<GrantedLake>, String>;
    fn load_cursor(lake: &str, source: &str, data_type: &str) -> Option<String>;
    fn save_cursor(
        lake: &str,
        source: &str,
        data_type: &str,
        cursor: Option<&str>,
        written: u64,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<(), String>;
    fn ensure_table(lake: &str, table: &str) -> Result<(), String>;
    fn append_json_batch(
        lake: &str,
        table: &str,
        source: &str,
        rows_json: &[String],
    ) -> Result<u64, String>;
    fn query_json(lake: &str, sql: &str) -> Result<String, String>;
}

pub trait ConnectorBackend {
    fn list_bindings() -> Result<Vec<GrantedConnectorBinding>, String>;
    fn upsert_binding(binding: &GrantedConnectorBinding)
        -> Result<GrantedConnectorBinding, String>;
    fn remove_binding(binding_id: &str) -> Result<(), String>;
    fn sync_binding(
        binding_id: &str,
        data_type: &str,
        since: Option<&str>,
    ) -> Result<ConnectorSyncResult, String>;
}

pub trait GithubBackend {
    fn list_issues(
        owner: &str,
        repo: &str,
        params: &GithubListParams,
    ) -> Result<GithubPage, String>;
    fn list_pulls(owner: &str, repo: &str, params: &GithubListParams)
        -> Result<GithubPage, String>;
    fn list_issue_comments(
        owner: &str,
        repo: &str,
        issue_number: u32,
    ) -> Result<GithubPage, String>;
    fn list_issue_events(owner: &str, repo: &str, issue_number: u32) -> Result<GithubPage, String>;
    fn list_pull_comments(owner: &str, repo: &str, pull_number: u32) -> Result<GithubPage, String>;
    fn list_reviews(owner: &str, repo: &str, pull_number: u32) -> Result<GithubPage, String>;
    fn list_review_comments(
        owner: &str,
        repo: &str,
        pull_number: u32,
        review_id: u64,
    ) -> Result<GithubPage, String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MeasureToy<B>(std::marker::PhantomData<B>);

impl<B> MeasureToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<B: MeasureBackend> MeasureToy<B> {
    pub fn record(
        &self,
        verb: &str,
        tool: &str,
        mode: &str,
        metrics_json: &str,
    ) -> Result<(), String> {
        B::record(verb, tool, mode, metrics_json)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CheckpointToy<B>(std::marker::PhantomData<B>);

impl<B> CheckpointToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<B: CheckpointBackend> CheckpointToy<B> {
    pub fn load(&self, stream: &str) -> Option<String> {
        B::load(stream)
    }

    pub fn save(&self, stream: &str, checkpoint_json: &str) -> Result<(), String> {
        B::save(stream, checkpoint_json)
    }
}

#[derive(Debug, Clone)]
pub struct LakeToy<B> {
    granted: GrantedLake,
    _marker: std::marker::PhantomData<B>,
}

impl<B> LakeToy<B> {
    pub fn new(granted: GrantedLake) -> Self {
        Self {
            granted,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn grant(&self) -> &GrantedLake {
        &self.granted
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LakeCatalog<B>(std::marker::PhantomData<B>);

impl<B> LakeCatalog<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<B: LakeBackend> LakeToy<B> {
    pub fn load_cursor(&self, source: &str, data_type: &str) -> Option<String> {
        B::load_cursor(&self.granted.name, source, data_type)
    }

    pub fn save_cursor(&self, record: &LakeCursorRecord) -> Result<(), String> {
        B::save_cursor(
            &self.granted.name,
            &record.source,
            &record.data_type,
            record.cursor.as_deref(),
            record.written,
            &record.status,
            record.last_error.as_deref(),
        )
    }

    pub fn ensure_table(&self, table: &str) -> Result<(), String> {
        B::ensure_table(&self.granted.name, table)
    }

    pub fn append_json_batch(
        &self,
        table: &str,
        source: &str,
        rows_json: &[String],
    ) -> Result<u64, String> {
        B::append_json_batch(&self.granted.name, table, source, rows_json)
    }

    pub fn query_json(&self, sql: &str) -> Result<String, String> {
        B::query_json(&self.granted.name, sql)
    }
}

impl<B: LakeBackend> LakeCatalog<B> {
    pub fn list(&self) -> Result<Vec<LakeToy<B>>, String> {
        Ok(B::list_granted_lakes()?
            .into_iter()
            .map(LakeToy::new)
            .collect())
    }

    pub fn require(&self, name: &str) -> Result<LakeToy<B>, String> {
        self.list()?
            .into_iter()
            .find(|lake| lake.grant().name == name)
            .ok_or_else(|| format!("lake '{}' not granted", name))
    }
}

#[derive(Debug, Clone)]
pub struct ConnectorBinding<B> {
    granted: GrantedConnectorBinding,
    _marker: std::marker::PhantomData<B>,
}

impl<B> ConnectorBinding<B> {
    pub fn new(granted: GrantedConnectorBinding) -> Self {
        Self {
            granted,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn grant(&self) -> &GrantedConnectorBinding {
        &self.granted
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ConnectorCatalog<B>(std::marker::PhantomData<B>);

impl<B> ConnectorCatalog<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<B: ConnectorBackend> ConnectorBinding<B> {
    pub fn sync(
        &self,
        data_type: &str,
        since: Option<&str>,
    ) -> Result<ConnectorSyncResult, String> {
        B::sync_binding(&self.granted.binding_id, data_type, since)
    }
}

impl<B: ConnectorBackend> ConnectorCatalog<B> {
    pub fn list(&self) -> Result<Vec<ConnectorBinding<B>>, String> {
        Ok(B::list_bindings()?
            .into_iter()
            .map(ConnectorBinding::new)
            .collect())
    }

    pub fn require(&self, binding_id: &str) -> Result<ConnectorBinding<B>, String> {
        self.list()?
            .into_iter()
            .find(|binding| binding.grant().binding_id == binding_id)
            .ok_or_else(|| format!("connector binding '{}' not granted", binding_id))
    }

    pub fn upsert(&self, binding: &GrantedConnectorBinding) -> Result<ConnectorBinding<B>, String> {
        B::upsert_binding(binding).map(ConnectorBinding::new)
    }

    pub fn remove(&self, binding_id: &str) -> Result<(), String> {
        B::remove_binding(binding_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GithubToy<B>(std::marker::PhantomData<B>);

impl<B> GithubToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<B: GithubBackend> GithubToy<B> {
    pub fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        params: &GithubListParams,
    ) -> Result<GithubPage, String> {
        B::list_issues(owner, repo, params)
    }

    pub fn list_pulls(
        &self,
        owner: &str,
        repo: &str,
        params: &GithubListParams,
    ) -> Result<GithubPage, String> {
        B::list_pulls(owner, repo, params)
    }

    pub fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u32,
    ) -> Result<GithubPage, String> {
        B::list_issue_comments(owner, repo, issue_number)
    }

    pub fn list_issue_events(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u32,
    ) -> Result<GithubPage, String> {
        B::list_issue_events(owner, repo, issue_number)
    }

    pub fn list_pull_comments(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u32,
    ) -> Result<GithubPage, String> {
        B::list_pull_comments(owner, repo, pull_number)
    }

    pub fn list_reviews(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u32,
    ) -> Result<GithubPage, String> {
        B::list_reviews(owner, repo, pull_number)
    }

    pub fn list_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u32,
        review_id: u64,
    ) -> Result<GithubPage, String> {
        B::list_review_comments(owner, repo, pull_number, review_id)
    }
}
