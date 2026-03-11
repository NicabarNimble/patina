use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct GrantedToys {
    pub fetch: bool,
    pub lake_names: HashSet<String>,
    pub query: bool,
    pub measure: bool,
    pub graph: bool,
    pub belief: bool,
}
