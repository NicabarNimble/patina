use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

pub mod audit;
pub mod compliance;
pub mod database;

pub use audit::PatternAuditor;
pub use compliance::{ComplianceChecker, ComplianceResult};
pub use database::PointerDatabase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternPointer {
    pub id: Option<i64>,
    pub pattern_id: String,
    pub file_path: PathBuf,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub symbol: Option<String>,
    pub compliance: ComplianceStatus,
    pub explanation: Option<String>,
    pub suggested_fix: Option<String>,
    pub confidence: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub last_verified: DateTime<Utc>,
    pub commit_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceStatus {
    Compliant,
    Violation,
    Partial,
    Unknown,
}

impl ComplianceStatus {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "compliant" => Self::Compliant,
            "violation" => Self::Violation,
            "partial" => Self::Partial,
            _ => Self::Unknown,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Compliant => "compliant".to_string(),
            Self::Violation => "violation".to_string(),
            Self::Partial => "partial".to_string(),
            Self::Unknown => "unknown".to_string(),
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Self::Compliant => "✓",
            Self::Violation => "✗",
            Self::Partial => "◐",
            Self::Unknown => "?",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternSpread {
    pub pattern_id: String,
    pub total_implementations: usize,
    pub compliant_count: usize,
    pub violation_count: usize,
    pub partial_count: usize,
    pub compliance_rate: f32,
    pub last_updated: DateTime<Utc>,
}

impl PatternSpread {
    pub fn compliance_rate(&self) -> f32 {
        if self.total_implementations == 0 {
            0.0
        } else {
            (self.compliant_count as f32 / self.total_implementations as f32) * 100.0
        }
    }
}

pub struct PointerSystem {
    database: PointerDatabase,
    auditor: PatternAuditor,
}

impl PointerSystem {
    pub fn new(db_path: &Path) -> Result<Self> {
        Ok(Self {
            database: PointerDatabase::new(db_path)?,
            auditor: PatternAuditor::new(),
        })
    }

    pub fn create_pointer(
        &mut self,
        pattern_id: &str,
        file_path: &Path,
        symbol: Option<String>,
    ) -> Result<PatternPointer> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let pattern_content = self.load_pattern(pattern_id)?;
        
        let compliance = self.auditor
            .check_compliance(&content, &pattern_content, file_path)?;

        let pointer = PatternPointer {
            id: None,
            pattern_id: pattern_id.to_string(),
            file_path: file_path.to_path_buf(),
            line_start: compliance.line_start,
            line_end: compliance.line_end,
            symbol,
            compliance: compliance.status,
            explanation: Some(compliance.explanation),
            suggested_fix: compliance.suggested_fix,
            confidence: Some(compliance.confidence),
            created_at: Utc::now(),
            last_verified: Utc::now(),
            commit_hash: self.get_current_commit()?,
        };

        self.database.insert_pointer(&pointer)?;
        Ok(pointer)
    }

    /// Create a pointer with an LLM's assessment
    /// This is called when the LLM UI has already analyzed the code
    pub fn create_pointer_with_assessment(
        &mut self,
        pattern_id: &str,
        file_path: &Path,
        compliance: ComplianceStatus,
        explanation: String,
        suggested_fix: Option<String>,
        symbol: Option<String>,
    ) -> Result<PatternPointer> {
        // Verify the file exists
        if !file_path.exists() {
            anyhow::bail!("File does not exist: {}", file_path.display());
        }

        // Verify the pattern exists
        let _ = self.load_pattern(pattern_id)?;

        let pointer = PatternPointer {
            id: None,
            pattern_id: pattern_id.to_string(),
            file_path: file_path.to_path_buf(),
            line_start: None,
            line_end: None,
            symbol,
            compliance,
            explanation: Some(explanation),
            suggested_fix,
            confidence: Some(0.9), // High confidence since LLM analyzed it
            created_at: Utc::now(),
            last_verified: Utc::now(),
            commit_hash: self.get_current_commit()?,
        };

        self.database.insert_pointer(&pointer)?;
        Ok(pointer)
    }

    pub fn get_pointers_for_pattern(&self, pattern_id: &str) -> Result<Vec<PatternPointer>> {
        self.database.get_pointers_by_pattern(pattern_id)
    }

    pub fn get_pointers_for_file(&self, file_path: &Path) -> Result<Vec<PatternPointer>> {
        self.database.get_pointers_by_file(file_path)
    }

    pub fn audit_file(&mut self, file_path: &Path) -> Result<Vec<PatternPointer>> {
        let patterns = self.list_patterns()?;
        let mut pointers = Vec::new();

        for pattern_id in patterns {
            match self.create_pointer(&pattern_id, file_path, None) {
                Ok(pointer) => pointers.push(pointer),
                Err(e) => eprintln!("Warning: Failed to check pattern {}: {}", pattern_id, e),
            }
        }

        Ok(pointers)
    }

    pub fn get_pattern_spread(&self, pattern_id: &str) -> Result<PatternSpread> {
        self.database.get_pattern_spread(pattern_id)
    }

    fn load_pattern(&self, pattern_id: &str) -> Result<String> {
        let pattern_path = PathBuf::from("layer")
            .join("core")
            .join(format!("{}.md", pattern_id));
        
        if !pattern_path.exists() {
            let pattern_path = PathBuf::from("layer")
                .join("surface")
                .join(format!("{}.md", pattern_id));
            
            if pattern_path.exists() {
                return std::fs::read_to_string(&pattern_path)
                    .with_context(|| format!("Failed to read pattern: {}", pattern_id));
            }
        } else {
            return std::fs::read_to_string(&pattern_path)
                .with_context(|| format!("Failed to read pattern: {}", pattern_id));
        }
        
        anyhow::bail!("Pattern not found: {}", pattern_id)
    }

    fn list_patterns(&self) -> Result<Vec<String>> {
        let mut patterns = Vec::new();
        
        for dir in &["layer/core", "layer/surface"] {
            let path = PathBuf::from(dir);
            if path.exists() {
                for entry in std::fs::read_dir(&path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            patterns.push(stem.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(patterns)
    }

    fn get_current_commit(&self) -> Result<Option<String>> {
        let output = std::process::Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()?;
        
        if output.status.success() {
            Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
        } else {
            Ok(None)
        }
    }
}