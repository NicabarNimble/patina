use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantedIngressSource {
    pub name: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Default)]
pub struct GrantedToys {
    pub fetch: bool,
    pub lake_names: HashSet<String>,
    pub ingress_sources: HashMap<String, GrantedIngressSource>,
    pub connector: bool,
    pub query: bool,
    pub measure: bool,
    pub graph: bool,
    pub belief: bool,
}
