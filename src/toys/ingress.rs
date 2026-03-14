//! Ingress toy host helpers.

use crate::mother::GrantedToys;

pub fn list_granted_sources(toys: &GrantedToys) -> Vec<(String, String)> {
    toys.ingress_sources
        .values()
        .map(|source| (source.name.clone(), source.endpoint.clone()))
        .collect()
}

pub fn resolve_source_endpoint(
    toys: &GrantedToys,
    plugin_name: &str,
    source_name: &str,
) -> Result<String, String> {
    toys.ingress_sources
        .get(source_name)
        .map(|source| source.endpoint.clone())
        .ok_or_else(|| {
            format!(
                "ingress source '{}' not granted for '{}'",
                source_name, plugin_name
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mother::{GrantedIngressSource, GrantedToys};

    #[test]
    fn lists_granted_sources() {
        let mut toys = GrantedToys::default();
        toys.ingress_sources.insert(
            "gh-main".into(),
            GrantedIngressSource {
                name: "gh-main".into(),
                endpoint: "https://api.github.com/repos/NicabarNimble/patina/issues".into(),
            },
        );
        let listed = list_granted_sources(&toys);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].0, "gh-main");
    }

    #[test]
    fn rejects_ungranted_source() {
        let toys = GrantedToys::default();
        let error = resolve_source_endpoint(&toys, "patina-ducklake", "gh-main").unwrap_err();
        assert!(error.contains("not granted"));
    }
}
