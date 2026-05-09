use serde::{Deserialize, Serialize};

use super::{Buffer, PayloadContract, ViewShape};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadFrame {
    pub protocol: String,
    pub version: u32,
    pub payload_contract: PayloadContract,
    pub shape_id: String,
    pub shape_version: u32,
    pub buffer_id: String,
    pub payload_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FramedJsonPayload {
    pub frame: PayloadFrame,
    pub json: serde_json::Value,
}

impl FramedJsonPayload {
    pub fn new(buffer: &Buffer, shape: &ViewShape, json: serde_json::Value) -> Self {
        Self {
            frame: PayloadFrame {
                protocol: "patina:view-buffer".to_string(),
                version: 1,
                payload_contract: buffer.payload_contract.clone(),
                shape_id: shape.shape_id.clone(),
                shape_version: shape.version,
                buffer_id: buffer.buffer_id.clone(),
                payload_version: buffer.payload_version,
            },
            json,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::view_buffer::{MajorMode, MinorMode, ViewRequirement, ViewShapeScope};

    #[test]
    fn framed_json_payload_uses_stable_wit_style_envelope() {
        // obligation: rule-success.OpenLiveBufferWhenRequiredFactsAreObserved
        let shape = ViewShape {
            shape_id: "mother.status.default".to_string(),
            title: "Mother Status".to_string(),
            scope: ViewShapeScope::MotherUser,
            version: 3,
            active: true,
            major_mode: MajorMode::Table,
            minor_modes: vec![MinorMode::Pinned],
            payload_contract: PayloadContract::FramedJson,
            payload_version: 2,
            requirements: vec![ViewRequirement {
                fact_path: "mother.status.version".to_string(),
                required: true,
                purpose: "display Mother version".to_string(),
            }],
        };
        let buffer = Buffer::live_from_shape("buf_status".to_string(), &shape, Utc::now());

        let payload = FramedJsonPayload::new(&buffer, &shape, json!({"rows": []}));

        assert_eq!(payload.frame.protocol, "patina:view-buffer");
        assert_eq!(payload.frame.payload_contract, PayloadContract::FramedJson);
        assert_eq!(payload.frame.shape_id, "mother.status.default");
        assert_eq!(payload.frame.shape_version, 3);
        assert_eq!(payload.frame.buffer_id, "buf_status");
        assert_eq!(payload.json, json!({"rows": []}));
    }
}
