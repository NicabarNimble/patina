use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterfaceControlOperation {
    HandshakeV1,
    EnvelopeResolveV1,
    EnvelopeHeartbeatV1,
    EnvelopeEndV1,
}

impl InterfaceControlOperation {
    fn parse(operation_id: &str) -> Option<Self> {
        match operation_id {
            "patina:interface/handshake.v1" => Some(Self::HandshakeV1),
            "patina:interface/envelope.resolve.v1" => Some(Self::EnvelopeResolveV1),
            "patina:interface/envelope.heartbeat.v1" => Some(Self::EnvelopeHeartbeatV1),
            "patina:interface/envelope.end.v1" => Some(Self::EnvelopeEndV1),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::HandshakeV1 => "patina:interface/handshake.v1",
            Self::EnvelopeResolveV1 => "patina:interface/envelope.resolve.v1",
            Self::EnvelopeHeartbeatV1 => "patina:interface/envelope.heartbeat.v1",
            Self::EnvelopeEndV1 => "patina:interface/envelope.end.v1",
        }
    }
}

pub(super) fn dispatch_interface_control_call(
    request: mother_crate::http_api::InterfaceControlCallRequest,
) -> Result<serde_json::Value> {
    let operation = InterfaceControlOperation::parse(&request.operation_id).ok_or_else(|| {
        anyhow::Error::new(mother_crate::http_api::LifecycleError::invalid_request(
            format!("unsupported interface operation '{}'", request.operation_id),
        ))
    })?;

    Ok(serde_json::json!({
        "adapter": "native",
        "operation_id": operation.as_str(),
        "status": "scaffold",
        "implemented": false,
        "args": request.args,
        "correlation": request.correlation,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_operation_id() {
        let error =
            dispatch_interface_control_call(mother_crate::http_api::InterfaceControlCallRequest {
                operation_id: "patina:interface/unknown.v1".to_string(),
                args: serde_json::json!([]),
                correlation: None,
            })
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unsupported interface operation"),
            "got: {}",
            error
        );
    }

    #[test]
    fn accepts_known_operation_id_with_scaffold_result() {
        let response =
            dispatch_interface_control_call(mother_crate::http_api::InterfaceControlCallRequest {
                operation_id: "patina:interface/handshake.v1".to_string(),
                args: serde_json::json!({"project_uid": "2bdc808e"}),
                correlation: Some(mother_crate::http_api::InterfaceControlCorrelation {
                    project_uid: Some("2bdc808e".to_string()),
                    interface: Some("pi".to_string()),
                    launch_id: Some("launch-1".to_string()),
                }),
            })
            .expect("known operation should be accepted");

        assert_eq!(response.get("status"), Some(&serde_json::json!("scaffold")));
        assert_eq!(
            response
                .get("operation_id")
                .and_then(|value| value.as_str()),
            Some("patina:interface/handshake.v1")
        );
    }
}
