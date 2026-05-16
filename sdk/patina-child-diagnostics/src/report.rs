use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStage {
    LocalDev,
    ComponentBuilt,
    ReleaseCandidate,
}

impl Default for DiagnosticStage {
    fn default() -> Self {
        Self::LocalDev
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticPhase {
    Manifest,
    Wit,
    Component,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFinding {
    pub severity: DiagnosticSeverity,
    pub phase: DiagnosticPhase,
    pub code: String,
    pub location: Option<PathBuf>,
    pub message: String,
    pub help: Option<String>,
}

impl DiagnosticFinding {
    pub fn error(
        phase: DiagnosticPhase,
        code: impl Into<String>,
        location: impl Into<Option<PathBuf>>,
        message: impl Into<String>,
        help: impl Into<Option<String>>,
    ) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            phase,
            code: code.into(),
            location: location.into(),
            message: message.into(),
            help: help.into(),
        }
    }

    pub fn warning(
        phase: DiagnosticPhase,
        code: impl Into<String>,
        location: impl Into<Option<PathBuf>>,
        message: impl Into<String>,
        help: impl Into<Option<String>>,
    ) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            phase,
            code: code.into(),
            location: location.into(),
            message: message.into(),
            help: help.into(),
        }
    }

    pub fn note(
        phase: DiagnosticPhase,
        code: impl Into<String>,
        location: impl Into<Option<PathBuf>>,
        message: impl Into<String>,
        help: impl Into<Option<String>>,
    ) -> Self {
        Self {
            severity: DiagnosticSeverity::Note,
            phase,
            code: code.into(),
            location: location.into(),
            message: message.into(),
            help: help.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub package_root: PathBuf,
    pub stage: DiagnosticStage,
    pub findings: Vec<DiagnosticFinding>,
}

impl DiagnosticReport {
    pub fn new(package_root: PathBuf, stage: DiagnosticStage) -> Self {
        Self {
            package_root,
            stage,
            findings: Vec::new(),
        }
    }

    pub fn push(&mut self, finding: DiagnosticFinding) {
        self.findings.push(finding);
    }

    pub fn extend(&mut self, findings: impl IntoIterator<Item = DiagnosticFinding>) {
        self.findings.extend(findings);
    }

    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == DiagnosticSeverity::Error)
    }

    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }

    pub fn assert_ok(&self) {
        if self.is_ok() {
            return;
        }

        panic!("Patina child diagnostics failed:\n{}", self.render_text());
    }

    pub fn render_text(&self) -> String {
        let mut out = String::new();
        for finding in &self.findings {
            let severity = match finding.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
                DiagnosticSeverity::Note => "note",
            };
            out.push_str(&format!(
                "{severity}[{}]: {}\n",
                finding.code, finding.message
            ));
            if let Some(location) = &finding.location {
                out.push_str(&format!("  --> {}\n", display_path(location)));
            }
            if let Some(help) = &finding.help {
                out.push_str(&format!("  help: {help}\n"));
            }
        }
        out
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
