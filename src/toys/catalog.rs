//! Canonical toy capability catalog.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToyCapability {
    pub id: &'static str,
    pub grant_field: &'static str,
    pub owner: &'static str,
}

pub const TOY_CATALOG: &[ToyCapability] = &[
    ToyCapability {
        id: "fetch",
        grant_field: "GrantedToys::fetch",
        owner: "toys::http",
    },
    ToyCapability {
        id: "lake",
        grant_field: "GrantedToys::lake_names",
        owner: "toys::lake",
    },
    ToyCapability {
        id: "ingress",
        grant_field: "GrantedToys::ingress_sources",
        owner: "toys::ingress",
    },
    ToyCapability {
        id: "connector",
        grant_field: "GrantedToys::connector",
        owner: "toys::connector",
    },
    ToyCapability {
        id: "query",
        grant_field: "GrantedToys::query",
        owner: "toys::query",
    },
    ToyCapability {
        id: "measure",
        grant_field: "GrantedToys::measure",
        owner: "measure",
    },
    ToyCapability {
        id: "graph",
        grant_field: "GrantedToys::graph",
        owner: "mother::graph_host",
    },
    ToyCapability {
        id: "belief",
        grant_field: "GrantedToys::belief",
        owner: "mother::belief_host",
    },
];

pub fn all() -> &'static [ToyCapability] {
    TOY_CATALOG
}
