use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum BridgeVerdict {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeRequest {
    pub action: String,
    #[serde(default)]
    pub legacy_toys: Vec<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeToyDecision {
    pub legacy_toy: String,
    pub typed_toy: Option<String>,
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResponse {
    pub action: String,
    pub verdict: BridgeVerdict,
    pub typed_toys: Vec<String>,
    pub denied_toys: Vec<String>,
    pub decisions: Vec<BridgeToyDecision>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeExposure {
    pub requires_bridge: bool,
    pub denied_aliases: Vec<String>,
}

pub fn evaluate_bridge_request(request: &BridgeRequest) -> BridgeResponse {
    let mut typed = BTreeSet::new();
    let mut denied = BTreeSet::new();
    let mut decisions = Vec::with_capacity(request.legacy_toys.len());

    for toy in &request.legacy_toys {
        match map_legacy_toy(toy) {
            Some(mapped) => {
                typed.insert(mapped.to_string());
                decisions.push(BridgeToyDecision {
                    legacy_toy: normalize_alias(toy),
                    typed_toy: Some(mapped.to_string()),
                    allowed: true,
                    reason: None,
                });
            }
            None => {
                let normalized = normalize_alias(toy);
                denied.insert(normalized.clone());
                decisions.push(BridgeToyDecision {
                    legacy_toy: normalized,
                    typed_toy: None,
                    allowed: false,
                    reason: Some("unknown-legacy-toy".to_string()),
                });
            }
        }
    }

    let typed_toys = typed.into_iter().collect::<Vec<_>>();
    let denied_toys = denied.into_iter().collect::<Vec<_>>();
    let verdict = if denied_toys.is_empty() {
        BridgeVerdict::Allow
    } else {
        BridgeVerdict::Deny
    };

    BridgeResponse {
        action: request.action.clone(),
        verdict,
        typed_toys,
        denied_toys,
        decisions,
        payload: request.payload.clone(),
    }
}

pub fn bridge_exposure_for_toys(toys: &[String]) -> BridgeExposure {
    let request = BridgeRequest {
        action: "atlas-lane-check".to_string(),
        legacy_toys: toys.to_vec(),
        payload: serde_json::Value::Null,
    };
    let evaluated = evaluate_bridge_request(&request);

    BridgeExposure {
        requires_bridge: !evaluated.typed_toys.is_empty(),
        denied_aliases: evaluated.denied_toys,
    }
}

pub fn map_legacy_toy(alias: &str) -> Option<&'static str> {
    match normalize_alias(alias).as_str() {
        "log" => Some("logging"),
        "state" => Some("keyvalue"),
        "store" => Some("keyvalue"),
        "fs" => Some("filesystem"),
        _ => None,
    }
}

fn normalize_alias(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_legacy_aliases_to_typed_toys() {
        let request = BridgeRequest {
            action: "dispatch".to_string(),
            legacy_toys: vec!["log".into(), "store".into(), "state".into(), "fs".into()],
            payload: serde_json::json!({"k": "v"}),
        };

        let response = evaluate_bridge_request(&request);
        assert_eq!(response.verdict, BridgeVerdict::Allow);
        assert_eq!(
            response.typed_toys,
            vec![
                "filesystem".to_string(),
                "keyvalue".to_string(),
                "logging".to_string()
            ]
        );
        assert!(response.denied_toys.is_empty());
    }

    #[test]
    fn unknown_alias_denies_request_fail_closed() {
        let request = BridgeRequest {
            action: "dispatch".to_string(),
            legacy_toys: vec!["log".into(), "magic".into()],
            payload: serde_json::Value::Null,
        };

        let response = evaluate_bridge_request(&request);
        assert_eq!(response.verdict, BridgeVerdict::Deny);
        assert_eq!(response.denied_toys, vec!["magic".to_string()]);
        assert_eq!(response.typed_toys, vec!["logging".to_string()]);
        assert!(response
            .decisions
            .iter()
            .any(|decision| decision.legacy_toy == "magic" && !decision.allowed));
    }

    #[test]
    fn exposure_marks_bridge_usage_and_denied_aliases() {
        let exposure = bridge_exposure_for_toys(&[
            "log".to_string(),
            "state".to_string(),
            "unknown".to_string(),
        ]);

        assert!(exposure.requires_bridge);
        assert_eq!(exposure.denied_aliases, vec!["unknown".to_string()]);
    }
}
