use anyhow::Result;
use std::path::Path;
use super::compliance::{ComplianceChecker, ComplianceResult};

pub struct PatternAuditor {
    checker: ComplianceChecker,
}

impl PatternAuditor {
    pub fn new() -> Self {
        Self {
            checker: ComplianceChecker::new(),
        }
    }

    // LLM interaction happens through the UI (Claude Code, Gemini CLI, etc)
    // Not through direct API calls

    pub fn check_compliance(
        &self,
        code: &str,
        pattern: &str,
        file_path: &Path,
    ) -> Result<ComplianceResult> {
        self.checker.check_compliance(code, pattern, file_path)
    }

    pub fn audit_directory(&self, dir: &Path) -> Result<AuditReport> {
        let mut report = AuditReport::new();
        
        self.audit_dir_recursive(dir, &mut report)?;
        
        Ok(report)
    }

    fn audit_dir_recursive(&self, dir: &Path, report: &mut AuditReport) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Skip common directories
                let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if dir_name == "target" || dir_name == ".git" || dir_name == "node_modules" {
                    continue;
                }
                
                self.audit_dir_recursive(&path, report)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                report.files_checked += 1;
                // In a real implementation, we'd check this file against patterns
            }
        }
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct AuditReport {
    pub files_checked: usize,
    pub patterns_found: usize,
    pub compliant_count: usize,
    pub violation_count: usize,
    pub partial_count: usize,
    pub findings: Vec<AuditFinding>,
}

impl AuditReport {
    pub fn new() -> Self {
        Self {
            files_checked: 0,
            patterns_found: 0,
            compliant_count: 0,
            violation_count: 0,
            partial_count: 0,
            findings: Vec::new(),
        }
    }

    pub fn add_finding(&mut self, finding: AuditFinding) {
        match &finding.compliance {
            super::ComplianceStatus::Compliant => self.compliant_count += 1,
            super::ComplianceStatus::Violation => self.violation_count += 1,
            super::ComplianceStatus::Partial => self.partial_count += 1,
            super::ComplianceStatus::Unknown => {}
        }
        self.findings.push(finding);
    }

    pub fn compliance_rate(&self) -> f32 {
        let total = self.compliant_count + self.violation_count + self.partial_count;
        if total == 0 {
            0.0
        } else {
            (self.compliant_count as f32 / total as f32) * 100.0
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str("# Pattern Audit Report\n\n");
        md.push_str(&format!("- Files checked: {}\n", self.files_checked));
        md.push_str(&format!("- Patterns found: {}\n", self.patterns_found));
        md.push_str(&format!("- Compliance rate: {:.1}%\n\n", self.compliance_rate()));
        
        md.push_str("## Summary\n\n");
        md.push_str(&format!("- ✓ Compliant: {}\n", self.compliant_count));
        md.push_str(&format!("- ✗ Violations: {}\n", self.violation_count));
        md.push_str(&format!("- ◐ Partial: {}\n\n", self.partial_count));
        
        if !self.findings.is_empty() {
            md.push_str("## Findings\n\n");
            
            for finding in &self.findings {
                md.push_str(&format!(
                    "### {} {}\n",
                    finding.compliance.symbol(),
                    finding.file_path.display()
                ));
                md.push_str(&format!("- Pattern: {}\n", finding.pattern_id));
                md.push_str(&format!("- Status: {}\n", finding.compliance.to_string()));
                md.push_str(&format!("- {}\n", finding.explanation));
                
                if let Some(fix) = &finding.suggested_fix {
                    md.push_str(&format!("- Fix: {}\n", fix));
                }
                
                md.push_str("\n");
            }
        }
        
        md
    }
}

#[derive(Debug)]
pub struct AuditFinding {
    pub file_path: std::path::PathBuf,
    pub pattern_id: String,
    pub compliance: super::ComplianceStatus,
    pub explanation: String,
    pub suggested_fix: Option<String>,
}