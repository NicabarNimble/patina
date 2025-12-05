//! Opportunity detection logic
//!
//! Detects bounties and hackathon opportunities from GitHub issues.

use regex::Regex;

use super::provider::{OpportunityInfo, OpportunityProvider};
use crate::commands::scrape::github::GitHubIssue;

/// Detect opportunity in a GitHub issue using configured providers
pub fn detect_opportunity(
    issue: &GitHubIssue,
    providers: &[OpportunityProvider],
) -> OpportunityInfo {
    for provider in providers {
        if !provider.enabled {
            continue;
        }

        // Check labels first (most reliable signal)
        let label_match = issue.labels.iter().any(|label| {
            let label_lower = label.name.to_lowercase();
            provider
                .labels
                .iter()
                .any(|p| label_lower.contains(&p.to_lowercase()))
        });

        // Check URL patterns in body
        let url_match = issue.body.as_ref().is_some_and(|body| {
            provider
                .url_patterns
                .iter()
                .any(|pattern| body.contains(pattern))
        });

        // Check for amount patterns in body (fallback detection)
        let (amount, currency) = extract_amount(issue, provider);
        let amount_match = amount.is_some();

        if label_match || url_match || amount_match {
            let url = extract_url(issue, provider);

            return OpportunityInfo {
                is_opportunity: true,
                provider: Some(provider.name.clone()),
                amount,
                currency,
                url,
            };
        }
    }

    OpportunityInfo::none()
}

/// Extract bounty amount from issue body using provider's patterns
fn extract_amount(issue: &GitHubIssue, provider: &OpportunityProvider) -> (Option<String>, Option<String>) {
    let body = match &issue.body {
        Some(b) => b,
        None => return (None, None),
    };

    for pattern in &provider.amount_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(body) {
                let amount = caps.get(1).map(|m| m.as_str().to_string());

                // Try to match currency from the pattern or use first configured
                let currency = caps
                    .get(2)
                    .map(|m| m.as_str().to_uppercase())
                    .or_else(|| provider.currencies.first().cloned());

                if amount.is_some() {
                    // Format amount with currency
                    let formatted = match (&amount, &currency) {
                        (Some(a), Some(c)) => Some(format!("{} {}", a, c)),
                        (Some(a), None) => Some(a.clone()),
                        _ => None,
                    };
                    return (formatted, currency);
                }
            }
        }
    }

    (None, None)
}

/// Extract URL to opportunity platform from issue body
fn extract_url(issue: &GitHubIssue, provider: &OpportunityProvider) -> Option<String> {
    let body = issue.body.as_ref()?;

    // Simple URL extraction - look for provider's URL patterns
    for pattern in &provider.url_patterns {
        // Find URLs containing the pattern
        let url_re = Regex::new(&format!(r"https?://[^\s<>\[\]]*{}[^\s<>\[\]]*", regex::escape(pattern))).ok()?;
        if let Some(m) = url_re.find(body) {
            return Some(m.as_str().to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::scrape::github::{Author, Label};

    fn make_issue(title: &str, body: Option<&str>, labels: Vec<&str>) -> GitHubIssue {
        GitHubIssue {
            number: 1,
            title: title.to_string(),
            body: body.map(|s| s.to_string()),
            state: "open".to_string(),
            labels: labels
                .into_iter()
                .map(|name| Label {
                    name: name.to_string(),
                })
                .collect(),
            author: Author {
                login: "test".to_string(),
            },
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            closed_at: None,
            url: "https://github.com/test/test/issues/1".to_string(),
        }
    }

    fn algora_provider() -> OpportunityProvider {
        OpportunityProvider {
            name: "algora".to_string(),
            enabled: true,
            labels: vec!["💎 Bounty".to_string(), "algora".to_string()],
            url_patterns: vec!["algora.io".to_string()],
            amount_patterns: vec![r"\$(\d+(?:,\d{3})*)".to_string()],
            currencies: vec!["USD".to_string()],
        }
    }

    #[test]
    fn test_detect_by_label() {
        let issue = make_issue("Test bounty", Some("Fix this bug"), vec!["💎 Bounty"]);
        let providers = vec![algora_provider()];

        let info = detect_opportunity(&issue, &providers);
        assert!(info.is_opportunity);
        assert_eq!(info.provider, Some("algora".to_string()));
    }

    #[test]
    fn test_detect_by_url() {
        let issue = make_issue(
            "Test issue",
            Some("Check out https://console.algora.io/bounty/123"),
            vec!["bug"],
        );
        let providers = vec![algora_provider()];

        let info = detect_opportunity(&issue, &providers);
        assert!(info.is_opportunity);
        assert_eq!(info.provider, Some("algora".to_string()));
    }

    #[test]
    fn test_extract_amount() {
        let issue = make_issue(
            "Bounty: $500",
            Some("This is a $500 bounty for fixing the bug"),
            vec!["💎 Bounty"],
        );
        let providers = vec![algora_provider()];

        let info = detect_opportunity(&issue, &providers);
        assert!(info.is_opportunity);
        assert_eq!(info.amount, Some("500 USD".to_string()));
    }

    #[test]
    fn test_no_match() {
        let issue = make_issue("Regular issue", Some("Just a bug report"), vec!["bug"]);
        let providers = vec![algora_provider()];

        let info = detect_opportunity(&issue, &providers);
        assert!(!info.is_opportunity);
        assert!(info.provider.is_none());
    }

    #[test]
    fn test_disabled_provider() {
        let issue = make_issue("Test", None, vec!["💎 Bounty"]);
        let mut provider = algora_provider();
        provider.enabled = false;

        let info = detect_opportunity(&issue, &[provider]);
        assert!(!info.is_opportunity);
    }
}
