use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    Buffer, FramedJsonPayload, MajorMode, MinorMode, ObservabilityGap, PayloadContract,
    ViewRequirement, ViewShape, ViewShapeScope,
};
use crate::view_buffer::catalog::{DataCatalog, MOTHER_STATUS_SHAPE_ID, MOTHER_STATUS_SOURCE_ID};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenBufferRequest {
    pub shape_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenedBuffer {
    pub buffer: Buffer,
    pub payload: FramedJsonPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum OpenBufferOutcome {
    Opened(OpenedBuffer),
    ObservabilityGap(ObservabilityGap),
}

#[derive(Debug, Clone)]
pub struct ViewBufferService {
    catalog: DataCatalog,
    shapes: BTreeMap<String, ViewShape>,
    buffers: BTreeMap<String, Buffer>,
    gaps: BTreeMap<String, ObservabilityGap>,
    next_id: u64,
}

impl ViewBufferService {
    pub fn with_catalog(catalog: DataCatalog) -> Self {
        let proof_shape = mother_status_shape();
        Self {
            catalog,
            shapes: BTreeMap::from([(proof_shape.shape_id.clone(), proof_shape)]),
            buffers: BTreeMap::new(),
            gaps: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn list_buffers(&self) -> Vec<Buffer> {
        self.buffers.values().cloned().collect()
    }

    pub fn list_gaps(&self) -> Vec<ObservabilityGap> {
        self.gaps.values().cloned().collect()
    }

    pub fn open_buffer(&mut self, request: OpenBufferRequest) -> Result<OpenBufferOutcome> {
        let shape = self
            .shapes
            .get(&request.shape_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown view shape '{}'", request.shape_id))?;

        if let Some(missing) = shape
            .requirements
            .iter()
            .find(|requirement| !self.catalog.observed_required_fact(requirement))
        {
            let gap = self.record_gap(&shape, missing, Utc::now());
            return Ok(OpenBufferOutcome::ObservabilityGap(gap));
        }

        let buffer_id = self.next_buffer_id(&shape);
        let buffer = Buffer::live_from_shape(buffer_id, &shape, Utc::now());
        let payload = FramedJsonPayload::new(&buffer, &shape, self.payload_json_for_shape(&shape));
        self.buffers
            .insert(buffer.buffer_id.clone(), buffer.clone());
        Ok(OpenBufferOutcome::Opened(OpenedBuffer { buffer, payload }))
    }

    fn record_gap(
        &mut self,
        shape: &ViewShape,
        missing: &ViewRequirement,
        created_at: DateTime<Utc>,
    ) -> ObservabilityGap {
        let gap_id = format!("gap_{}_{}", self.next_id, sanitize_id(&missing.fact_path));
        self.next_id += 1;
        let missing_source_id = self
            .catalog
            .fact(&missing.fact_path)
            .map(|fact| fact.source_id.clone())
            .or_else(|| Some(MOTHER_STATUS_SOURCE_ID.to_string()));
        let gap = ObservabilityGap::open(
            gap_id,
            Some(shape.shape_id.clone()),
            missing.fact_path.clone(),
            missing_source_id,
            format!("required fact '{}' is not observed", missing.fact_path),
            created_at,
        );
        self.gaps.insert(gap.gap_id.clone(), gap.clone());
        gap
    }

    fn next_buffer_id(&mut self, shape: &ViewShape) -> String {
        let id = format!("buf_{}_{}", self.next_id, sanitize_id(&shape.shape_id));
        self.next_id += 1;
        id
    }

    fn payload_json_for_shape(&self, shape: &ViewShape) -> serde_json::Value {
        let rows: Vec<_> = shape
            .requirements
            .iter()
            .filter(|requirement| requirement.required)
            .filter_map(|requirement| {
                self.catalog.value(&requirement.fact_path).map(|value| {
                    json!({
                        "fact_path": requirement.fact_path,
                        "purpose": requirement.purpose,
                        "value": value,
                    })
                })
            })
            .collect();

        json!({
            "major_mode": shape.major_mode,
            "minor_modes": shape.minor_modes,
            "columns": ["fact_path", "purpose", "value"],
            "rows": rows,
        })
    }
}

pub fn mother_status_shape() -> ViewShape {
    ViewShape {
        shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
        title: "Mother Status".to_string(),
        scope: ViewShapeScope::MotherUser,
        version: 1,
        active: true,
        major_mode: MajorMode::Table,
        minor_modes: vec![MinorMode::Pinned],
        payload_contract: PayloadContract::FramedJson,
        payload_version: 1,
        requirements: vec![
            required("mother.status.version", "display Mother binary version"),
            required(
                "mother.status.control_plane_ready",
                "display control-plane readiness",
            ),
            required(
                "mother.status.registered_projects",
                "display registered project count",
            ),
            required(
                "mother.status.children_ready_count",
                "display ready child count",
            ),
            required("mother.status.children_total", "display total child count"),
        ],
    }
}

fn required(fact_path: &str, purpose: &str) -> ViewRequirement {
    ViewRequirement {
        fact_path: fact_path.to_string(),
        required: true,
        purpose: purpose.to_string(),
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::view_buffer::{
        BufferState, MotherStatusFacts, ObservabilityGapStatus, SourceAvailability,
    };

    fn status_catalog() -> DataCatalog {
        DataCatalog::mother_status(MotherStatusFacts {
            version: "0.67.1".to_string(),
            uptime_secs: 42,
            control_plane_ready: true,
            registered_projects: 48,
            children_ready_count: 1,
            children_total: 2,
            startup_profile: "full".to_string(),
            memory_pressure: "ok".to_string(),
            observed_at: Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap(),
        })
    }

    #[test]
    fn opens_live_buffer_when_required_facts_are_observed() {
        // obligation: rule-success.OpenLiveBufferWhenRequiredFactsAreObserved
        let mut service = ViewBufferService::with_catalog(status_catalog());

        let outcome = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .expect("open should evaluate");

        let OpenBufferOutcome::Opened(opened) = outcome else {
            panic!("expected opened buffer");
        };
        assert_eq!(opened.buffer.state, BufferState::Live);
        assert_eq!(opened.buffer.name, "*Mother Status*");
        assert_eq!(opened.payload.frame.protocol, "patina:view-buffer");
        assert_eq!(service.list_buffers().len(), 1);
        assert_eq!(service.list_gaps().len(), 0);
    }

    #[test]
    fn records_gap_and_refuses_buffer_when_required_fact_missing() {
        // obligation: rule-success.RecordObservabilityGapWhenRequiredFactIsMissing
        // obligation: rule-failure.OpenLiveBufferWhenRequiredFactsAreObserved.2
        let catalog = status_catalog().without_fact("mother.status.children_total");
        let mut service = ViewBufferService::with_catalog(catalog);

        let outcome = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .expect("open should evaluate");

        let OpenBufferOutcome::ObservabilityGap(gap) = outcome else {
            panic!("expected observability gap");
        };
        assert_eq!(gap.status, ObservabilityGapStatus::Open);
        assert_eq!(gap.missing_fact_path, "mother.status.children_total");
        assert_eq!(service.list_buffers().len(), 0);
        assert_eq!(service.list_gaps().len(), 1);
    }

    #[test]
    fn records_gap_and_refuses_buffer_when_required_source_unavailable() {
        // obligation: rule-success.RecordObservabilityGapWhenRequiredFactIsMissing
        // obligation: rule-failure.OpenLiveBufferWhenRequiredFactsAreObserved.2
        let catalog = status_catalog()
            .with_source_availability(MOTHER_STATUS_SOURCE_ID, SourceAvailability::Unavailable);
        let mut service = ViewBufferService::with_catalog(catalog);

        let outcome = service
            .open_buffer(OpenBufferRequest {
                shape_id: MOTHER_STATUS_SHAPE_ID.to_string(),
            })
            .expect("open should evaluate");

        let OpenBufferOutcome::ObservabilityGap(gap) = outcome else {
            panic!("expected observability gap");
        };
        assert_eq!(gap.status, ObservabilityGapStatus::Open);
        assert_eq!(
            gap.missing_source_id,
            Some(MOTHER_STATUS_SOURCE_ID.to_string())
        );
        assert_eq!(service.list_buffers().len(), 0);
        assert_eq!(service.list_gaps().len(), 1);
    }
}
