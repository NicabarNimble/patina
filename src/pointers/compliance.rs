use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use super::ComplianceStatus;

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplianceResult {
    pub status: ComplianceStatus,
    pub explanation: String,
    pub suggested_fix: Option<String>,
    pub confidence: f32,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

pub struct ComplianceChecker {
    // The LLM (Claude, Gemini, etc) IS the UI
    // We don't call LLMs - they call us!
}

impl ComplianceChecker {
    pub fn new() -> Self {
        Self {}
    }

    /// Basic heuristic compliance check
    /// The real intelligence comes from the LLM UI calling us with its assessment
    pub fn check_compliance(
        &self,
        code: &str,
        pattern: &str,
        file_path: &Path,
    ) -> Result<ComplianceResult> {
        self.heuristic_check(code, pattern, file_path)
    }

    /// Create a compliance result from LLM assessment
    /// Called when the LLM has already analyzed and provides the result
    pub fn from_llm_assessment(
        status: ComplianceStatus,
        explanation: String,
        suggested_fix: Option<String>,
        confidence: f32,
    ) -> ComplianceResult {
        ComplianceResult {
            status,
            explanation,
            suggested_fix,
            confidence,
            line_start: None,
            line_end: None,
        }
    }

    // Note: LLM integration happens through the UI layer (Claude Code, Gemini CLI, etc)
    // The LLM analyzes code and calls our CLI with the assessment
    // Example: patina pattern point dependable-rust src/mod.rs --compliance=compliant


    fn heuristic_check(
        &self,
        code: &str,
        pattern: &str,
        _file_path: &Path,
    ) -> Result<ComplianceResult> {
        // Basic heuristic checks when no LLM is available
        let mut violations = Vec::new();
        
        // Check for dependable-rust pattern (≤150 lines in mod.rs)
        if pattern.contains("dependable-rust") || pattern.contains("150 lines") {
            let line_count = code.lines().count();
            if code.contains("mod.rs") && line_count > 150 {
                violations.push(format!("File exceeds 150 lines ({} lines)", line_count));
            }
        }

        // Check for error-context-chain pattern
        if pattern.contains("error-context") || pattern.contains("context chain") {
            if code.contains("Result<") && !code.contains("context(") && !code.contains("with_context(") {
                violations.push("Missing error context in Result handling".to_string());
            }
        }

        // Check for internal module pattern
        if pattern.contains("internal") && pattern.contains("module") {
            if !code.contains("mod internal") && !code.contains("internal::") {
                violations.push("Missing internal module structure".to_string());
            }
        }

        let status = if violations.is_empty() {
            ComplianceStatus::Compliant
        } else if violations.len() == 1 {
            ComplianceStatus::Partial
        } else {
            ComplianceStatus::Violation
        };

        let explanation = if violations.is_empty() {
            "Code appears to follow the pattern based on heuristic analysis".to_string()
        } else {
            format!("Found violations: {}", violations.join(", "))
        };

        Ok(ComplianceResult {
            status,
            explanation,
            suggested_fix: if !violations.is_empty() {
                Some(format!("Address these issues: {}", violations.join("; ")))
            } else {
                None
            },
            confidence: 0.3, // Low confidence for heuristic checks
            line_start: None,
            line_end: None,
        })
    }
}