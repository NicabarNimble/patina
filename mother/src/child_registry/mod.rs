mod github;
mod sync;

pub use github::GitHubChildRegistryProvider;
pub use sync::{
    ChildRegistryProvider, ChildRegistrySyncEngine, DiscoveredChildRelease, SourceSyncReport,
};
