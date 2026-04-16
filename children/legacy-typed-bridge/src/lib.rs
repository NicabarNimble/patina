wit_bindgen::generate!({
    path: "wit",
    world: "legacy-typed-bridge",
    generate_all,
});

use patina_sdk::toys;
use std::collections::BTreeSet;

struct LegacyTypedBridge;

fn map_legacy_toy(alias: &str) -> Option<&'static str> {
    match alias.trim().to_ascii_lowercase().as_str() {
        "log" => Some("logging"),
        "state" => Some("keyvalue"),
        "store" => Some("keyvalue"),
        "fs" => Some("filesystem"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::map_legacy_toy;

    #[test]
    fn maps_known_aliases() {
        assert_eq!(map_legacy_toy("log"), Some("logging"));
        assert_eq!(map_legacy_toy("state"), Some("keyvalue"));
        assert_eq!(map_legacy_toy("store"), Some("keyvalue"));
        assert_eq!(map_legacy_toy("fs"), Some("filesystem"));
    }

    #[test]
    fn rejects_unknown_aliases() {
        assert_eq!(map_legacy_toy("unknown"), None);
    }
}

impl exports::patina::legacy_typed_bridge::seam::Guest for LegacyTypedBridge {
    fn translate(
        request: exports::patina::legacy_typed_bridge::seam::BridgeRequest,
    ) -> exports::patina::legacy_typed_bridge::seam::BridgeResponse {
        let mut typed = BTreeSet::new();
        let mut denied = BTreeSet::new();

        for toy in &request.legacy_toys {
            match map_legacy_toy(toy) {
                Some(mapped) => {
                    typed.insert(mapped.to_string());
                }
                None => {
                    denied.insert(toy.trim().to_ascii_lowercase());
                }
            }
        }

        let typed_toys = typed.into_iter().collect::<Vec<_>>();
        let denied_toys = denied.into_iter().collect::<Vec<_>>();
        let verdict = if denied_toys.is_empty() {
            exports::patina::legacy_typed_bridge::seam::BridgeVerdict::Allow
        } else {
            exports::patina::legacy_typed_bridge::seam::BridgeVerdict::Deny
        };

        toys::log::info(
            "legacy-typed-bridge",
            &format!(
                "translate action={} typed={} denied={}",
                request.action,
                typed_toys.len(),
                denied_toys.len()
            ),
        );

        exports::patina::legacy_typed_bridge::seam::BridgeResponse {
            action: request.action,
            verdict,
            typed_toys,
            denied_toys,
            payload: request.payload,
        }
    }
}

export!(LegacyTypedBridge);
