//! Canonical spec frontmatter format
//!
//! This module defines the contract for spec files. All commands that read/write
//! spec frontmatter should use these types and functions.
//!
//! Design rationale:
//! - [[system-owns-format]]: Rust struct owns the format, deterministic output
//! - [[milestones-in-specs]]: Data lives in specs, derive indexes
//!
//! # Example
//!
//! ```ignore
//! use patina::spec::{parse_spec_file, serialize_spec_file};
//!
//! let content = std::fs::read_to_string("SPEC.md")?;
//! let (mut frontmatter, body) = parse_spec_file(&content)?;
//! frontmatter.status = Some("ready".to_string());
//! let new_content = serialize_spec_file(&frontmatter, &body)?;
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// Spec Frontmatter Types
// ============================================================================

/// Sessions can be either a simple list or a structured object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Sessions {
    /// Simple list of session IDs: [20260108-200725, ...]
    List(Vec<String>),
    /// Structured with origin and work: { origin: ..., work: [...] }
    Structured {
        #[serde(skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        work: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        updated: Option<String>,
    },
}

/// Milestone in spec frontmatter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecMilestoneEntry {
    pub version: String,
    pub name: String,
    pub status: String,
}

/// Complete spec frontmatter - the canonical contract for spec files
///
/// All fields except `r#type` and `id` are optional to handle legacy specs.
/// Use `#[serde(skip_serializing_if)]` to avoid writing empty fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpecFrontmatter {
    /// Spec type: feat, fix, refactor, explore, etc.
    #[serde(default)]
    pub r#type: String,

    /// Unique identifier (matches filename convention)
    #[serde(default)]
    pub id: String,

    /// Status: draft, ready, active, paused, blocked, complete, abandoned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Creation date (YYYY-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    /// Last update date (YYYY-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,

    /// Target version (e.g., "v0.12.0") — spec-as-work-item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Specs that block this one — spec-as-work-item
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,

    /// Specs that this one blocks — spec-as-work-item
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,

    /// Session references
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Sessions>,

    /// Related specs/files
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,

    /// Belief references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beliefs: Vec<String>,

    /// Schema references (fact types this spec introduces)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub schemas: Vec<String>,

    /// External references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,

    /// Version milestones
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub milestones: Vec<SpecMilestoneEntry>,

    /// Current milestone being worked on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_milestone: Option<String>,

    /// Why this spec was paused (required on pause)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_reason: Option<String>,

    /// When paused (ISO 8601 date, UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_date: Option<String>,

    /// Tag ref for resume diffs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_at_tag: Option<String>,

    /// Why this spec was blocked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,

    /// When blocked (ISO 8601 date, UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_date: Option<String>,

    /// Parent spec ID (set by split)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_from: Option<String>,
}

// ============================================================================
// Parse / Serialize
// ============================================================================

/// Parse spec file into frontmatter and body
///
/// Expects file to start with `---` YAML frontmatter delimiter.
pub fn parse_spec_file(content: &str) -> Result<(SpecFrontmatter, String)> {
    // Extract frontmatter between --- markers
    let content = content
        .strip_prefix("---")
        .ok_or_else(|| anyhow::anyhow!("Spec file must start with '---' frontmatter delimiter"))?;

    // Handle both \n--- and \r\n--- line endings
    let end = content.find("\n---").ok_or_else(|| {
        anyhow::anyhow!("Spec file must have closing '---' frontmatter delimiter")
    })?;

    let frontmatter_str = &content[..end];
    let body = &content[end + 4..]; // Skip "\n---"

    let frontmatter: SpecFrontmatter = serde_yaml::from_str(frontmatter_str)
        .with_context(|| format!("Failed to parse frontmatter:\n{}", frontmatter_str))?;

    Ok((frontmatter, body.to_string()))
}

/// Serialize spec back to file content
///
/// Produces deterministic YAML output with consistent field ordering.
pub fn serialize_spec_file(frontmatter: &SpecFrontmatter, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter)?;
    Ok(format!("---\n{}---{}", yaml, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_roundtrip() {
        let content = r#"---
type: feat
id: test-spec
status: ready
target: v0.12.0
blocked_by:
  - other-spec
blocks: []
---

# Test Spec

Body content here.
"#;

        let (frontmatter, body) = parse_spec_file(content).expect("should parse");
        assert_eq!(frontmatter.id, "test-spec");
        assert_eq!(frontmatter.status, Some("ready".to_string()));
        assert_eq!(frontmatter.target, Some("v0.12.0".to_string()));
        assert_eq!(frontmatter.blocked_by, vec!["other-spec"]);
        assert!(body.contains("# Test Spec"));

        let output = serialize_spec_file(&frontmatter, &body).expect("should serialize");
        let (fm2, _) = parse_spec_file(&output).expect("should re-parse");
        assert_eq!(fm2.id, frontmatter.id);
        assert_eq!(fm2.status, frontmatter.status);
    }

    #[test]
    fn test_optional_fields() {
        let content = r#"---
type: explore
id: minimal
---

# Minimal spec
"#;

        let (frontmatter, _) = parse_spec_file(content).expect("should parse minimal");
        assert_eq!(frontmatter.id, "minimal");
        assert_eq!(frontmatter.status, None);
        assert!(frontmatter.blocked_by.is_empty());
    }
}
