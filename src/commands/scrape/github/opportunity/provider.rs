//! Opportunity provider definitions
//!
//! Data-driven provider configuration for bounty/hackathon detection.

use serde::Deserialize;

/// Configuration for an opportunity provider (bounty platform, hackathon, etc.)
#[derive(Debug, Clone, Deserialize)]
pub struct OpportunityProvider {
    /// Provider name (e.g., "algora", "dorahacks")
    #[serde(default)]
    pub name: String,

    /// Whether this provider is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Labels that indicate an opportunity (e.g., "💎 Bounty", "hackathon")
    #[serde(default)]
    pub labels: Vec<String>,

    /// URL patterns in issue body that indicate this provider
    #[serde(default)]
    pub url_patterns: Vec<String>,

    /// Regex patterns to extract amount from issue body
    #[serde(default)]
    pub amount_patterns: Vec<String>,

    /// Supported currencies/tokens (e.g., "USD", "USDC", "ETH", "STRK")
    #[serde(default)]
    pub currencies: Vec<String>,
}

fn default_enabled() -> bool {
    true
}

/// Result of opportunity detection
#[derive(Debug, Clone, Default)]
pub struct OpportunityInfo {
    /// Whether an opportunity was detected
    pub is_opportunity: bool,

    /// Provider that matched (e.g., "algora", "dorahacks")
    pub provider: Option<String>,

    /// Amount string (e.g., "500 USDC", "0.5 ETH")
    pub amount: Option<String>,

    /// Currency/token (e.g., "USDC", "ETH", "STRK")
    pub currency: Option<String>,

    /// URL to the opportunity platform
    pub url: Option<String>,
}

impl OpportunityInfo {
    /// Create an empty (no opportunity) result
    pub fn none() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opportunity_info_default() {
        let info = OpportunityInfo::none();
        assert!(!info.is_opportunity);
        assert!(info.provider.is_none());
        assert!(info.amount.is_none());
    }

    #[test]
    fn test_provider_deserialize() {
        let toml = r#"
            name = "test"
            enabled = true
            labels = ["bounty", "reward"]
            url_patterns = ["test.io"]
            amount_patterns = ['\\$(\d+)']
            currencies = ["USD"]
        "#;

        let provider: OpportunityProvider = toml::from_str(toml).unwrap();
        assert_eq!(provider.name, "test");
        assert!(provider.enabled);
        assert_eq!(provider.labels.len(), 2);
    }
}
