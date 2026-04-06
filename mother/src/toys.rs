use std::collections::{HashMap, HashSet};

pub mod git;
pub mod layer_fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantedIngressSource {
    pub name: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Default)]
pub struct GrantedToys {
    pub http: bool,
    pub events: bool,
    pub messaging: bool,
    pub filesystem: bool,
    pub sql: bool,
    pub lake_names: HashSet<String>,
    pub ingress_sources: HashMap<String, GrantedIngressSource>,
    pub connector: bool,
    pub github: bool,
    pub session: bool,
    pub query: bool,
    pub measure: bool,
    pub graph: bool,
    pub belief: bool,
}
