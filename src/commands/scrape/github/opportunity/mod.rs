//! Opportunity provider system
//!
//! TOML-configured detection for bounties and hackathon opportunities.
//! Supports multiple platforms: Algora, DoraHacks, ETHGlobal, StarkNet.

mod detector;
mod provider;

pub use detector::detect_opportunity;
pub use provider::{OpportunityInfo, OpportunityProvider};

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

/// Configuration file format for opportunity providers
#[derive(Debug, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub providers: HashMap<String, OpportunityProvider>,
}

/// Load opportunity providers from TOML config or use defaults
pub fn load_providers() -> Result<Vec<OpportunityProvider>> {
    let config_path = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".patina/opportunity-providers.toml");

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: ProvidersConfig = toml::from_str(&content)?;

        let providers: Vec<OpportunityProvider> = config
            .providers
            .into_iter()
            .map(|(name, mut p)| {
                // Ensure name is set from the key
                if p.name.is_empty() {
                    p.name = name;
                }
                p
            })
            .filter(|p| p.enabled)
            .collect();

        Ok(providers)
    } else {
        // Return built-in defaults
        Ok(default_providers())
    }
}

/// Built-in default providers (used when no config file exists)
pub fn default_providers() -> Vec<OpportunityProvider> {
    vec![
        // Algora - GitHub issue bounties (most GitHub-native)
        OpportunityProvider {
            name: "algora".to_string(),
            enabled: true,
            labels: vec![
                "💎 Bounty".to_string(),
                "💎 bounty".to_string(),
                "algora".to_string(),
                "💰 Rewarded".to_string(),
            ],
            url_patterns: vec!["algora.io".to_string(), "console.algora.io".to_string()],
            amount_patterns: vec![
                r"\$(\d+(?:,\d{3})*(?:\.\d{2})?)".to_string(),
                r"(\d+(?:,\d{3})*)\s*USD".to_string(),
            ],
            currencies: vec!["USD".to_string(), "USDC".to_string()],
        },
        // DoraHacks - Hackathon bounties
        OpportunityProvider {
            name: "dorahacks".to_string(),
            enabled: true,
            labels: vec![
                "dorahacks".to_string(),
                "buidl".to_string(),
                "hackathon".to_string(),
            ],
            url_patterns: vec!["dorahacks.io".to_string(), "buidlbox.io".to_string()],
            amount_patterns: vec![
                r"\$(\d+(?:,\d{3})*)".to_string(),
                r"(\d+(?:,\d{3})*)\s*USDC".to_string(),
                r"(\d+(?:,\d{3})*)\s*USDT".to_string(),
            ],
            currencies: vec!["USDC".to_string(), "USDT".to_string(), "USD".to_string()],
        },
        // ETHGlobal - Ethereum hackathons
        OpportunityProvider {
            name: "ethglobal".to_string(),
            enabled: true,
            labels: vec![
                "ethglobal".to_string(),
                "hackathon".to_string(),
                "partner prize".to_string(),
            ],
            url_patterns: vec!["ethglobal.com".to_string()],
            amount_patterns: vec![
                r"(\d+(?:\.\d+)?)\s*ETH".to_string(),
                r"\$(\d+(?:,\d{3})*)".to_string(),
            ],
            currencies: vec!["ETH".to_string(), "USDC".to_string()],
        },
        // StarkNet - Cairo ecosystem (includes Dojo, Pragma)
        OpportunityProvider {
            name: "starknet".to_string(),
            enabled: true,
            labels: vec![
                "starknet".to_string(),
                "strk".to_string(),
                "cairo".to_string(),
                "dojo".to_string(),
                "pragma".to_string(),
            ],
            url_patterns: vec![
                "starknet.io".to_string(),
                "taikai.network".to_string(),
                "onlydust".to_string(),
            ],
            amount_patterns: vec![
                r"(\d+(?:,\d{3})*)\s*STRK".to_string(),
                r"\$(\d+(?:,\d{3})*)".to_string(),
            ],
            currencies: vec!["STRK".to_string(), "USDC".to_string()],
        },
        // Legacy/generic bounty detection (backwards compatibility)
        OpportunityProvider {
            name: "generic".to_string(),
            enabled: true,
            labels: vec![
                "bounty".to_string(),
                "reward".to_string(),
                "paid".to_string(),
                "💰".to_string(),
            ],
            url_patterns: vec![],
            amount_patterns: vec![
                r"(?i)bounty[:\s]+\$?(\d+[\d,]*)".to_string(),
                r"(?i)reward[:\s]+\$?(\d+[\d,]*)".to_string(),
                r"💰\s*\$?(\d+[\d,]*)".to_string(),
            ],
            currencies: vec!["USD".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_providers() {
        let providers = default_providers();
        assert!(!providers.is_empty());

        // Should have algora
        assert!(providers.iter().any(|p| p.name == "algora"));

        // Should have dorahacks
        assert!(providers.iter().any(|p| p.name == "dorahacks"));

        // Should have starknet
        assert!(providers.iter().any(|p| p.name == "starknet"));
    }

    #[test]
    fn test_parse_toml_config() {
        let toml = r#"
            [providers.custom]
            name = "custom"
            enabled = true
            labels = ["custom-bounty"]
            url_patterns = ["custom.io"]
            amount_patterns = ['\\$(\d+)']
            currencies = ["USD"]
        "#;

        let config: ProvidersConfig = toml::from_str(toml).unwrap();
        assert!(config.providers.contains_key("custom"));

        let custom = &config.providers["custom"];
        assert_eq!(custom.labels, vec!["custom-bounty"]);
    }
}
