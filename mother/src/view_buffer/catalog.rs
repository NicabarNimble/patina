use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{
    CataloguedFact, CataloguedSource, CataloguedSourceKind, FactKind, ObservationState,
    SourceAvailability, ViewRequirement,
};

pub const MOTHER_STATUS_SOURCE_ID: &str = "mother.status";
pub const MOTHER_STATUS_SHAPE_ID: &str = "mother.status.default";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotherStatusFacts {
    pub version: String,
    pub uptime_secs: u64,
    pub control_plane_ready: bool,
    pub registered_projects: usize,
    pub children_ready_count: usize,
    pub children_total: usize,
    pub startup_profile: String,
    pub memory_pressure: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DataCatalog {
    sources: BTreeMap<String, CataloguedSource>,
    facts: BTreeMap<String, CataloguedFact>,
    values: BTreeMap<String, Value>,
}

impl DataCatalog {
    pub fn mother_status(status: MotherStatusFacts) -> Self {
        let mut catalog = Self::default();
        catalog.sources.insert(
            MOTHER_STATUS_SOURCE_ID.to_string(),
            CataloguedSource {
                source_id: MOTHER_STATUS_SOURCE_ID.to_string(),
                source_kind: CataloguedSourceKind::Registry,
                availability: SourceAvailability::Available,
                last_observed_at: Some(status.observed_at),
            },
        );

        catalog.observe_raw("mother.status.version", status.version);
        catalog.observe_raw("mother.status.uptime_secs", status.uptime_secs);
        catalog.observe_raw(
            "mother.status.control_plane_ready",
            status.control_plane_ready,
        );
        catalog.observe_raw(
            "mother.status.registered_projects",
            status.registered_projects,
        );
        catalog.observe_raw(
            "mother.status.children_ready_count",
            status.children_ready_count,
        );
        catalog.observe_raw("mother.status.children_total", status.children_total);
        catalog.observe_raw("mother.status.startup_profile", status.startup_profile);
        catalog.observe_raw("mother.status.memory.pressure", status.memory_pressure);
        catalog
    }

    pub fn sources(&self) -> impl Iterator<Item = &CataloguedSource> {
        self.sources.values()
    }

    pub fn facts(&self) -> impl Iterator<Item = &CataloguedFact> {
        self.facts.values()
    }

    pub fn fact(&self, fact_path: &str) -> Option<&CataloguedFact> {
        self.facts.get(fact_path)
    }

    pub fn value(&self, fact_path: &str) -> Option<&Value> {
        self.values.get(fact_path)
    }

    pub fn observed_required_fact(&self, requirement: &ViewRequirement) -> bool {
        !requirement.required
            || self
                .fact(&requirement.fact_path)
                .map(|fact| {
                    fact.is_observed()
                        && self
                            .sources
                            .get(&fact.source_id)
                            .map(|source| source.availability.is_available())
                            .unwrap_or(false)
                })
                .unwrap_or(false)
    }

    pub fn with_source_availability(
        mut self,
        source_id: &str,
        availability: SourceAvailability,
    ) -> Self {
        if let Some(source) = self.sources.get_mut(source_id) {
            source.availability = availability;
        }
        self
    }

    pub fn without_fact(mut self, fact_path: &str) -> Self {
        self.facts.remove(fact_path);
        self.values.remove(fact_path);
        self
    }

    fn observe_raw(&mut self, fact_path: &str, value: impl Into<Value>) {
        self.facts.insert(
            fact_path.to_string(),
            CataloguedFact {
                fact_path: fact_path.to_string(),
                source_id: MOTHER_STATUS_SOURCE_ID.to_string(),
                fact_kind: FactKind::Raw,
                observation_state: ObservationState::Observed,
            },
        );
        self.values.insert(fact_path.to_string(), value.into());
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;

    fn status() -> MotherStatusFacts {
        MotherStatusFacts {
            version: "0.67.1".to_string(),
            uptime_secs: 42,
            control_plane_ready: true,
            registered_projects: 48,
            children_ready_count: 1,
            children_total: 2,
            startup_profile: "full".to_string(),
            memory_pressure: "ok".to_string(),
            observed_at: Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap(),
        }
    }

    #[test]
    fn mother_status_catalog_exposes_observed_status_facts() {
        // obligation: entity-state.MotherDataCatalog + entity-state.CataloguedFact
        let catalog = DataCatalog::mother_status(status());

        assert_eq!(catalog.sources().count(), 1);
        assert!(catalog
            .sources()
            .next()
            .expect("source should exist")
            .availability
            .is_available());
        assert_eq!(catalog.facts().count(), 8);
        assert_eq!(
            catalog.value("mother.status.version"),
            Some(&json!("0.67.1"))
        );
        assert!(catalog
            .fact("mother.status.control_plane_ready")
            .expect("fact should exist")
            .is_observed());
    }

    #[test]
    fn missing_required_fact_is_not_observed() {
        // obligation: rule-failure.OpenLiveBufferWhenRequiredFactsAreObserved.2
        let catalog = DataCatalog::mother_status(status()).without_fact("mother.status.version");
        let requirement = ViewRequirement {
            fact_path: "mother.status.version".to_string(),
            required: true,
            purpose: "display version".to_string(),
        };

        assert!(!catalog.observed_required_fact(&requirement));
    }

    #[test]
    fn unavailable_source_means_required_fact_is_not_observed_for_opening() {
        // obligation: rule-failure.OpenLiveBufferWhenRequiredFactsAreObserved.2
        let catalog = DataCatalog::mother_status(status())
            .with_source_availability(MOTHER_STATUS_SOURCE_ID, SourceAvailability::Unavailable);
        let requirement = ViewRequirement {
            fact_path: "mother.status.version".to_string(),
            required: true,
            purpose: "display version".to_string(),
        };

        assert!(!catalog.observed_required_fact(&requirement));
    }
}
