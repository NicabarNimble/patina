pub mod canonical;
pub mod capabilities;
pub mod config;
pub mod error;
pub mod fact;
pub mod http;
pub mod manifest;

pub use canonical::{canonical_json, content_hash};
pub use capabilities::{Capabilities, HealthStatus, Status};
pub use config::{
    AuthConfig, ConnectorToy, DuckLakeGrant, Escalation, FetchParams, GrantInjection, IngestParams,
    IngestRecord, InitializeParams, RunResult, StorageToy, TypeReport,
};
pub use error::PipeError;
pub use fact::{Fact, FetchResult, IngestProvenance, IngestResult};
pub use http::{PipeHttpRequest, PipeHttpResponse};
pub use manifest::ChildManifest;
