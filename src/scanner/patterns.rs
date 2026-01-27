//! Secret detection patterns.

use super::Severity;

pub struct Pattern {
    pub name: &'static str,
    pub regex: &'static str,
    pub severity: Severity,
}

pub static PATTERNS: &[Pattern] = &[
    // === High Severity: Known Formats ===
    Pattern {
        name: "github_token",
        regex: r"(?:gh[oprsu]|github_pat)_[\dA-Za-z_]{36,}",
        severity: Severity::High,
    },
    Pattern {
        name: "gitlab_token",
        regex: r"glpat-[\dA-Za-z_=-]{20,}",
        severity: Severity::High,
    },
    Pattern {
        name: "aws_secret",
        regex: r#"(?i)aws.{0,20}['"][0-9a-zA-Z/+]{40}['"]"#,
        severity: Severity::High,
    },
    Pattern {
        name: "openai_key",
        regex: r"sk-[A-Za-z0-9]{48}",
        severity: Severity::High,
    },
    Pattern {
        name: "anthropic_key",
        regex: r"sk-ant-[\dA-Za-z_-]{90,110}",
        severity: Severity::High,
    },
    Pattern {
        name: "age_secret_key",
        regex: r"AGE-SECRET-KEY-1[\dA-Z]{58}",
        severity: Severity::High,
    },
    Pattern {
        name: "stripe_key",
        regex: r"[rs]k_live_[\dA-Za-z]{24,}",
        severity: Severity::High,
    },
    Pattern {
        name: "slack_token",
        regex: r"xox[aboprs]-(?:\d+-)+[\da-z]+",
        severity: Severity::High,
    },
    Pattern {
        name: "private_key_pem",
        regex: r"-{5}BEGIN (?:RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-{5}",
        severity: Severity::High,
    },
    // === Medium Severity: Heuristics ===
    Pattern {
        name: "url_credentials",
        regex: r"[a-z]+://[^:]+:[^@]{8,}@[\w./-]+",
        severity: Severity::Medium,
    },
    Pattern {
        name: "generic_secret",
        regex: r#"(?i)(?:password|secret|token|api_key)\s*[:=]\s*["'][^"']{8,}["']"#,
        severity: Severity::Medium,
    },
];
