//! Internal implementation for the measure command
//!
//! Queries the eventlog for measurement data from two sources:
//! - measure.* events (new, from Phase 1-2 tools)
//! - Existing typed events that carry measurement data

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::MeasureOptions;
use patina::eventlog;

// ============================================================================
// Domain Enums — construction-only, no Deserialize
// ============================================================================

/// Event source type — finite set of event_type strings from DB.
/// Explicit per-variant `serde(rename)` because values contain dots.
#[derive(Debug, Clone, Serialize)]
pub enum SourceType {
    #[serde(rename = "measure.capture")]
    MeasureCapture,
    #[serde(rename = "measure.index")]
    MeasureIndex,
    #[serde(rename = "measure.search")]
    MeasureSearch,
    #[serde(rename = "measure.believe")]
    MeasureBelieve,
    #[serde(rename = "measure.evolve")]
    MeasureEvolve,
    #[serde(rename = "git.commit")]
    GitCommit,
    #[serde(rename = "beliefs")]
    Beliefs,
    #[serde(rename = "session.ended")]
    SessionEnded,
}

impl SourceType {
    fn from_verb(verb: &str) -> Option<Self> {
        match verb {
            "capture" => Some(SourceType::MeasureCapture),
            "index" => Some(SourceType::MeasureIndex),
            "search" => Some(SourceType::MeasureSearch),
            "believe" => Some(SourceType::MeasureBelieve),
            "evolve" => Some(SourceType::MeasureEvolve),
            _ => None,
        }
    }
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::MeasureCapture => write!(f, "measure.capture"),
            SourceType::MeasureIndex => write!(f, "measure.index"),
            SourceType::MeasureSearch => write!(f, "measure.search"),
            SourceType::MeasureBelieve => write!(f, "measure.believe"),
            SourceType::MeasureEvolve => write!(f, "measure.evolve"),
            SourceType::GitCommit => write!(f, "git.commit"),
            SourceType::Beliefs => write!(f, "beliefs"),
            SourceType::SessionEnded => write!(f, "session.ended"),
        }
    }
}

/// Tool name — finite set of tool identifiers.
#[derive(Debug, Clone, Serialize)]
pub enum ToolName {
    #[serde(rename = "scrape")]
    Scrape,
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "eval")]
    Eval,
    #[serde(rename = "oxidize")]
    Oxidize,
    #[serde(rename = "doctor")]
    Doctor,
    #[serde(rename = "belief")]
    Belief,
}

impl ToolName {
    fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "scrape" => Some(ToolName::Scrape),
            "session" => Some(ToolName::Session),
            "eval" => Some(ToolName::Eval),
            "oxidize" => Some(ToolName::Oxidize),
            "doctor" => Some(ToolName::Doctor),
            "belief" => Some(ToolName::Belief),
            _ => None,
        }
    }
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolName::Scrape => write!(f, "scrape"),
            ToolName::Session => write!(f, "session"),
            ToolName::Eval => write!(f, "eval"),
            ToolName::Oxidize => write!(f, "oxidize"),
            ToolName::Doctor => write!(f, "doctor"),
            ToolName::Belief => write!(f, "belief"),
        }
    }
}

/// Single flat mode enum (Option C). VerbMetrics variant already enforces
/// verb-mode coherence — this enum prevents typos and enables exhaustive matching.
/// Doc comments note verb affinity per variant.
#[derive(Debug, Clone, Serialize)]
pub enum Mode {
    /// Capture mode: code parsing
    #[serde(rename = "code")]
    Code,
    /// Capture/believe mode: belief processing
    #[serde(rename = "beliefs")]
    Beliefs,
    /// Capture mode: layer/pattern processing
    #[serde(rename = "layer")]
    Layer,
    /// Capture mode: git scrape
    #[serde(rename = "git")]
    Git,
    /// Capture mode: health check
    #[serde(rename = "health-check")]
    HealthCheck,
    /// Search mode: eval quality
    #[serde(rename = "eval")]
    Eval,
    /// Search mode: audit
    #[serde(rename = "audit")]
    Audit,
    /// Evolve mode: session lifecycle
    #[serde(rename = "lifecycle")]
    Lifecycle,
    /// Capture mode: structural entropy metrics
    #[serde(rename = "structure")]
    Structure,
    /// Generic fallback mode
    #[serde(rename = "default")]
    Default,
}

impl Mode {
    fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "code" => Some(Mode::Code),
            "beliefs" => Some(Mode::Beliefs),
            "layer" => Some(Mode::Layer),
            "git" => Some(Mode::Git),
            "health-check" => Some(Mode::HealthCheck),
            "eval" => Some(Mode::Eval),
            "audit" => Some(Mode::Audit),
            "lifecycle" => Some(Mode::Lifecycle),
            "structure" => Some(Mode::Structure),
            "default" => Some(Mode::Default),
            _ => None,
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Code => write!(f, "code"),
            Mode::Beliefs => write!(f, "beliefs"),
            Mode::Layer => write!(f, "layer"),
            Mode::Git => write!(f, "git"),
            Mode::HealthCheck => write!(f, "health-check"),
            Mode::Eval => write!(f, "eval"),
            Mode::Audit => write!(f, "audit"),
            Mode::Lifecycle => write!(f, "lifecycle"),
            Mode::Structure => write!(f, "structure"),
            Mode::Default => write!(f, "default"),
        }
    }
}

// ============================================================================
// Data Structures
// ============================================================================

/// A single verb's measurement summary
#[derive(Debug, Serialize)]
pub struct VerbSummary {
    pub verb: String,
    pub status: VerbStatus,
    pub latest_timestamp: Option<String>,
    pub sources: Vec<SourceSummary>,
}

/// Verb health status — ordered by severity for worst-verb-wins.
///
/// `derive(Ord)` uses declaration order, so variants are arranged from
/// least severe (NoData) to most severe (Degraded). `max()` returns the worst.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum VerbStatus {
    NoData,
    Good,
    NeedsAttention,
    Degraded,
}

impl std::fmt::Display for VerbStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerbStatus::Good => write!(f, "good"),
            VerbStatus::NeedsAttention => write!(f, "needs attention"),
            VerbStatus::Degraded => write!(f, "degraded"),
            VerbStatus::NoData => write!(f, "no data"),
        }
    }
}

/// Metrics from a single source (tool+mode or existing event type)
#[derive(Debug, Serialize)]
pub struct SourceSummary {
    pub source_type: SourceType,
    pub tool: ToolName,
    pub mode: Mode,
    pub latest_metrics: VerbMetrics,
    pub timestamp: String,
    pub event_count: i64,
}

/// Full measurement report
#[derive(Debug, Serialize)]
pub struct MeasureReport {
    pub generated: String,
    pub verbs: Vec<VerbSummary>,
    pub total_measurement_events: i64,
    pub actions: Vec<String>,
}

/// History entry for verb drill-down
#[derive(Debug, Serialize)]
struct HistoryEntry {
    timestamp: String,
    tool: ToolName,
    mode: Mode,
    metrics: VerbMetrics,
}

// ============================================================================
// Typed Metric Variants
// ============================================================================

/// Typed metrics for each verb — replaces serde_json::Value as domain state.
///
/// Serializes flat (untagged) so JSON output shape is preserved.
/// Does NOT derive Deserialize — use `from_db()` for manual dispatch (ADR-1).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum VerbMetrics {
    CaptureCode(CaptureCodeMetrics),
    CaptureGit(CaptureGitMetrics),
    CaptureBeliefs(CaptureBeliefsMetrics),
    CaptureLayer(CaptureLayerMetrics),
    CaptureGitScrape(CaptureGitScrapeMetrics),
    CaptureHealthCheck(CaptureHealthCheckMetrics),
    CaptureStructure(CaptureStructureMetrics),
    Index(IndexMetrics),
    Search(SearchMetrics),
    Believe(BelieveMetrics),
    Evolve(EvolveMetrics),
    BelieveHistory(BelieveHistoryMetrics),
    EvolveHistory(EvolveHistoryMetrics),
    /// Fallback for unrecognized metric shapes — preserves data, logs warning.
    Raw(serde_json::Value),
}

impl VerbMetrics {
    /// Parse metrics JSON at the DB boundary, dispatching to the correct typed
    /// struct based on verb and mode context from the same DB row.
    ///
    /// History-only variants (BelieveHistory, EvolveHistory) are constructed
    /// directly in get_believe_history/get_evolve_history — they originate from
    /// belief.surface/session.ended events, not measure.* events. The dispatch
    /// paths are disjoint: from_db("believe", "beliefs", ...) → Believe (summary),
    /// while get_believe_history constructs BelieveHistory directly from SQL columns.
    pub fn from_db(verb: &str, mode: &str, json_str: &str) -> Self {
        let result = match (verb, mode) {
            ("capture", "code") => {
                serde_json::from_str::<CaptureCodeMetrics>(json_str).map(VerbMetrics::CaptureCode)
            }
            ("capture", "beliefs") => serde_json::from_str::<CaptureBeliefsMetrics>(json_str)
                .map(VerbMetrics::CaptureBeliefs),
            ("capture", "layer") => {
                serde_json::from_str::<CaptureLayerMetrics>(json_str).map(VerbMetrics::CaptureLayer)
            }
            ("capture", "git") => serde_json::from_str::<CaptureGitScrapeMetrics>(json_str)
                .map(VerbMetrics::CaptureGitScrape),
            ("capture", "health-check") => {
                serde_json::from_str::<CaptureHealthCheckMetrics>(json_str)
                    .map(VerbMetrics::CaptureHealthCheck)
            }
            ("capture", "structure") => {
                serde_json::from_str::<CaptureStructureMetrics>(json_str)
                    .map(VerbMetrics::CaptureStructure)
            }
            ("capture", _) => {
                tracing::warn!(mode, "Unknown capture mode — falling back to raw metrics");
                let value = serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
                return VerbMetrics::Raw(value);
            }
            ("index", _) => serde_json::from_str::<IndexMetrics>(json_str).map(VerbMetrics::Index),
            ("search", _) => {
                serde_json::from_str::<SearchMetrics>(json_str).map(VerbMetrics::Search)
            }
            ("believe", _) => {
                serde_json::from_str::<BelieveMetrics>(json_str).map(VerbMetrics::Believe)
            }
            ("evolve", _) => {
                serde_json::from_str::<EvolveMetrics>(json_str).map(VerbMetrics::Evolve)
            }
            _ => {
                tracing::warn!(verb, mode, "Unknown verb — falling back to raw metrics");
                let value = serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
                return VerbMetrics::Raw(value);
            }
        };

        result.unwrap_or_else(|e| {
            tracing::warn!(verb, mode, error = %e, "Falling back to raw metrics");
            let value = serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
            VerbMetrics::Raw(value)
        })
    }

    /// Raw key-value pairs — values are plain numbers, no unit suffixes.
    /// Used by format_metrics_inline for compact history table output.
    pub fn format_kv(&self) -> Vec<(String, String)> {
        match self {
            VerbMetrics::CaptureCode(m) => vec![
                ("files_parsed".into(), m.files_parsed.to_string()),
                ("functions_found".into(), m.functions_found.to_string()),
                ("types_found".into(), m.types_found.to_string()),
                ("fts_symbols".into(), m.fts_symbols.to_string()),
            ],
            VerbMetrics::CaptureGit(m) => vec![
                ("files_tracked".into(), m.files_tracked.to_string()),
                ("total_commits".into(), m.total_commits.to_string()),
            ],
            VerbMetrics::CaptureBeliefs(m) => vec![
                ("beliefs_processed".into(), m.beliefs_processed.to_string()),
                ("beliefs_verified".into(), m.beliefs_verified.to_string()),
                ("beliefs_skipped".into(), m.beliefs_skipped.to_string()),
                ("supports_edges".into(), m.supports_edges.to_string()),
                ("attacks_edges".into(), m.attacks_edges.to_string()),
                ("values_processed".into(), m.values_processed.to_string()),
                ("duration_ms".into(), m.duration_ms.to_string()),
            ],
            VerbMetrics::CaptureLayer(m) => vec![
                (
                    "patterns_processed".into(),
                    m.patterns_processed.to_string(),
                ),
                (
                    "sessions_processed".into(),
                    m.sessions_processed.to_string(),
                ),
                ("duration_ms".into(), m.duration_ms.to_string()),
            ],
            VerbMetrics::CaptureGitScrape(m) => vec![
                ("commits_processed".into(), m.commits_processed.to_string()),
                ("tracked_files".into(), m.tracked_files.to_string()),
                ("tags_indexed".into(), m.tags_indexed.to_string()),
                ("co_change_pairs".into(), m.co_change_pairs.to_string()),
                ("duration_ms".into(), m.duration_ms.to_string()),
            ],
            VerbMetrics::CaptureHealthCheck(m) => vec![
                ("beliefs".into(), m.beliefs.to_string()),
                ("sessions".into(), m.sessions.to_string()),
                ("layer_patterns".into(), m.layer_patterns.to_string()),
                ("missing_tools".into(), m.missing_tools.to_string()),
                ("new_tools".into(), m.new_tools.to_string()),
            ],
            VerbMetrics::CaptureStructure(m) => vec![
                ("module_count".into(), m.module_count.to_string()),
                (
                    "pub_interface_count".into(),
                    m.pub_interface_count.to_string(),
                ),
                ("dependency_count".into(), m.dependency_count.to_string()),
                ("coupling_avg".into(), format!("{:.1}", m.coupling_avg)),
                ("coupling_max".into(), m.coupling_max.to_string()),
            ],
            VerbMetrics::Index(m) => vec![(
                "documents_embedded".into(),
                m.documents_embedded.to_string(),
            )],
            VerbMetrics::Search(m) => {
                let mut pairs = Vec::new();
                if let Some(p5) = m.p_at_5 {
                    pairs.push(("p_at_5".into(), format!("{:.1}", p5 * 100.0)));
                }
                if let Some(mrr) = m.mrr {
                    pairs.push(("mrr".into(), format!("{:.3}", mrr)));
                }
                if let Some(r5) = m.recall_at_5 {
                    pairs.push(("recall_at_5".into(), format!("{:.1}", r5 * 100.0)));
                }
                pairs
            }
            VerbMetrics::Believe(m) => vec![
                ("total_beliefs".into(), m.total_beliefs.to_string()),
                ("floating_count".into(), m.floating_count.to_string()),
                ("grounded_count".into(), m.grounded_count.to_string()),
                ("contested_count".into(), m.contested_count.to_string()),
                ("avg_evidence".into(), format!("{:.2}", m.avg_evidence)),
                ("avg_health".into(), format!("{:.2}", m.avg_health)),
            ],
            VerbMetrics::Evolve(m) => vec![
                ("total_sessions".into(), m.total_sessions.to_string()),
                ("total_commits".into(), m.total_commits.to_string()),
                (
                    "total_files_changed".into(),
                    m.total_files_changed.to_string(),
                ),
                (
                    "total_beliefs_captured".into(),
                    m.total_beliefs_captured.to_string(),
                ),
                (
                    "total_patterns_modified".into(),
                    m.total_patterns_modified.to_string(),
                ),
            ],
            VerbMetrics::BelieveHistory(m) => vec![
                ("beliefs".into(), m.beliefs.to_string()),
                ("floating".into(), m.floating.to_string()),
                ("avg_evidence".into(), format!("{:.2}", m.avg_evidence)),
            ],
            VerbMetrics::EvolveHistory(m) => vec![
                ("commits".into(), m.commits.to_string()),
                ("files".into(), m.files.to_string()),
                ("beliefs".into(), m.beliefs.to_string()),
                ("patterns".into(), m.patterns.to_string()),
            ],
            VerbMetrics::Raw(v) => {
                if let Some(obj) = v.as_object() {
                    obj.iter()
                        .map(|(k, v)| {
                            let formatted = if let Some(f) = v.as_f64() {
                                if f == f.floor() && f.abs() < 1_000_000.0 {
                                    (f as i64).to_string()
                                } else {
                                    format!("{:.3}", f)
                                }
                            } else {
                                v.to_string()
                            };
                            (k.clone(), formatted)
                        })
                        .collect()
                } else {
                    vec![("value".into(), v.to_string())]
                }
            }
        }
    }

    /// Human-readable key-value pairs with unit suffixes (ms, %).
    /// Used by render_system_view for the current-state metric display.
    pub fn format_kv_display(&self) -> Vec<(String, String)> {
        self.format_kv()
            .into_iter()
            .map(|(k, v)| {
                let display_v = if k.ends_with("_ms") {
                    format!("{}ms", v)
                } else if k == "p_at_5" || k == "recall_at_5" {
                    format!("{}%", v)
                } else {
                    v
                };
                (k, display_v)
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureCodeMetrics {
    pub files_parsed: i64,
    pub functions_found: i64,
    pub types_found: i64,
    pub fts_symbols: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureGitMetrics {
    pub files_tracked: i64,
    pub total_commits: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureBeliefsMetrics {
    pub beliefs_processed: i64,
    pub beliefs_verified: i64,
    pub beliefs_skipped: i64,
    pub supports_edges: i64,
    pub attacks_edges: i64,
    pub values_processed: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureLayerMetrics {
    pub patterns_processed: i64,
    pub sessions_processed: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureGitScrapeMetrics {
    pub commits_processed: i64,
    pub tracked_files: i64,
    pub tags_indexed: i64,
    pub co_change_pairs: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureHealthCheckMetrics {
    pub beliefs: i64,
    pub sessions: i64,
    pub layer_patterns: i64,
    pub missing_tools: i64,
    pub new_tools: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureStructureMetrics {
    pub module_count: i64,
    pub pub_interface_count: i64,
    pub dependency_count: i64,
    pub coupling_avg: f64,
    pub coupling_max: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexMetrics {
    pub documents_embedded: i64,
}

/// Search quality metrics. Option<f64> per ADR-2: missing != zero.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchMetrics {
    pub p_at_5: Option<f64>,
    pub mrr: Option<f64>,
    pub recall_at_5: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BelieveMetrics {
    pub total_beliefs: i64,
    pub floating_count: i64,
    pub grounded_count: i64,
    #[serde(default)]
    pub contested_count: i64,
    pub avg_evidence: f64,
    pub avg_health: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvolveMetrics {
    pub total_sessions: i64,
    pub total_commits: i64,
    pub total_files_changed: i64,
    pub total_beliefs_captured: i64,
    pub total_patterns_modified: i64,
}

/// History-only metrics for believe drill-down. Field names are disjoint
/// from BelieveMetrics (beliefs vs total_beliefs) for unambiguous untagged serde.
#[derive(Debug, Serialize, Deserialize)]
pub struct BelieveHistoryMetrics {
    pub beliefs: i64,
    pub floating: i64,
    pub avg_evidence: f64,
}

/// History-only metrics for evolve drill-down. Field names are disjoint
/// from EvolveMetrics (commits vs total_commits) for unambiguous untagged serde.
#[derive(Debug, Serialize, Deserialize)]
pub struct EvolveHistoryMetrics {
    pub commits: i64,
    pub files: i64,
    pub beliefs: i64,
    pub patterns: i64,
}

// ============================================================================
// Full Report Types — typed health layer on existing infrastructure
// ============================================================================

/// Overall project health report — the `--full` JSON contract.
///
/// LLM query surface: every field is typed, no serde_json::Value.
/// Verbs are keyed by name (BTreeMap) for stable JSON paths.
#[derive(Debug, Serialize)]
pub struct FullMeasureReport {
    pub health: HealthSummary,
    pub verbs: std::collections::BTreeMap<String, FullVerbSummary>,
    pub event_counts: EventCounts,
}

/// Aggregate health across all verbs.
#[derive(Debug, Serialize)]
pub struct HealthSummary {
    pub status: VerbStatus,
    pub summary: String,
    pub assessed_at: String,
}

/// Extended verb summary with freshness and diagnostics.
#[derive(Debug, Serialize)]
pub struct FullVerbSummary {
    pub status: VerbStatus,
    pub latest_timestamp: Option<String>,
    pub age_hours: Option<f64>,
    pub freshness: Option<Freshness>,
    pub sources: Vec<SourceSummary>,
    pub diagnostics: Vec<Diagnostic>,
}

/// An actionable diagnostic derived from typed metrics.
#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}

/// Event count breakdown.
#[derive(Debug, Serialize)]
pub struct EventCounts {
    pub total_runtime_events: i64,
    pub by_type: std::collections::BTreeMap<String, i64>,
}

/// Data freshness — domain-aware interpretation of age.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Freshness {
    Fresh,
    Aging,
    Stale,
}

impl std::fmt::Display for Freshness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Freshness::Fresh => write!(f, "fresh"),
            Freshness::Aging => write!(f, "aging"),
            Freshness::Stale => write!(f, "stale"),
        }
    }
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Severity {
    Warning,
    Error,
}

/// Freshness thresholds per verb (hours).
///
/// Hardcoded — measure is opinionated about what "healthy" means.
/// `(fresh_ceiling, aging_ceiling)` — above aging_ceiling is stale.
const FRESHNESS_THRESHOLDS: &[(&str, f64, f64)] = &[
    ("capture", 24.0, 72.0),   // Active project scrapes daily
    ("index", 48.0, 168.0),    // 48h fresh, 7d aging ceiling
    ("search", 168.0, 720.0),  // 7d fresh, 30d aging ceiling
    ("believe", 168.0, 720.0), // 7d fresh, 30d aging ceiling
    ("evolve", 168.0, 720.0),  // 7d fresh, 30d aging ceiling
];

impl Freshness {
    /// Compute freshness for a verb given its age in hours.
    pub fn for_verb(verb: &str, age_hours: f64) -> Self {
        let (fresh_ceil, aging_ceil) = FRESHNESS_THRESHOLDS
            .iter()
            .find(|(v, _, _)| *v == verb)
            .map(|(_, f, a)| (*f, *a))
            .unwrap_or((168.0, 720.0)); // default to believe/evolve thresholds

        if age_hours < fresh_ceil {
            Freshness::Fresh
        } else if age_hours < aging_ceil {
            Freshness::Aging
        } else {
            Freshness::Stale
        }
    }
}

// ============================================================================
// Derivation — freshness, diagnostics, health
// ============================================================================

/// Compute age in hours from an RFC3339 or YYYY-MM-DD timestamp.
fn compute_age_hours(timestamp: &str) -> Option<f64> {
    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let duration = chrono::Utc::now().signed_duration_since(ts);
        return Some(duration.num_seconds() as f64 / 3600.0);
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(timestamp, "%Y-%m-%d") {
        let today = chrono::Utc::now().date_naive();
        let days = (today - date).num_days();
        return Some(days as f64 * 24.0);
    }
    None
}

impl BelieveMetrics {
    /// Diagnostics computed from typed fields.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        if self.floating_count > 0 && self.total_beliefs > 0 {
            let pct = (self.floating_count as f64 / self.total_beliefs as f64) * 100.0;
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "{} beliefs have no code grounding ({:.0}% floating)",
                    self.floating_count, pct
                ),
            });
        }
        if self.contested_count > 0 {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "{} beliefs have active attacks without resolution",
                    self.contested_count
                ),
            });
        }
        if self.avg_health < 0.5 && self.total_beliefs > 0 {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!("average belief health is low ({:.2})", self.avg_health),
            });
        }
        diags
    }
}

impl SearchMetrics {
    /// Diagnostics computed from typed fields.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        if let Some(p5) = self.p_at_5 {
            if p5 < 0.4 {
                diags.push(Diagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "search precision is low (P@5={:.0}%, threshold 40%)",
                        p5 * 100.0
                    ),
                });
            }
        }
        diags
    }
}

impl CaptureHealthCheckMetrics {
    /// Diagnostics computed from typed fields.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        if self.missing_tools > 0 {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!("{} expected tools are missing", self.missing_tools),
            });
        }
        diags
    }
}

/// Scrape duration threshold (ms) — warn when any scraper exceeds this.
const SCRAPE_DURATION_WARNING_MS: i64 = 5000;

impl CaptureGitScrapeMetrics {
    /// Diagnostics: warn when git scrape exceeds duration threshold.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        if self.duration_ms > SCRAPE_DURATION_WARNING_MS {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "git scrape took {}ms (threshold: {}ms)",
                    self.duration_ms, SCRAPE_DURATION_WARNING_MS
                ),
            });
        }
        diags
    }
}

impl CaptureBeliefsMetrics {
    /// Diagnostics: warn when beliefs scrape exceeds duration threshold.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        if self.duration_ms > SCRAPE_DURATION_WARNING_MS {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "beliefs scrape took {}ms (threshold: {}ms)",
                    self.duration_ms, SCRAPE_DURATION_WARNING_MS
                ),
            });
        }
        diags
    }
}

impl CaptureStructureMetrics {
    /// Diagnostics: compare current vs previous structural metrics.
    ///
    /// The `previous` parameter comes from the second-most-recent
    /// measure.capture.structure event. Delta thresholds are hardcoded.
    pub fn diagnostics_with_delta(&self, previous: Option<&CaptureStructureMetrics>) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let prev = match previous {
            Some(p) => p,
            None => return diags,
        };

        // Module count: warn if +2
        let module_delta = self.module_count - prev.module_count;
        if module_delta > 2 {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "module count increased by {} ({}→{}), threshold +2",
                    module_delta, prev.module_count, self.module_count
                ),
            });
        }

        // Pub interfaces: warn if +10%
        if prev.pub_interface_count > 0 {
            let pct = (self.pub_interface_count - prev.pub_interface_count) as f64
                / prev.pub_interface_count as f64
                * 100.0;
            if pct > 10.0 {
                diags.push(Diagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "pub interfaces increased by {:.0}% ({}→{}), threshold +10%",
                        pct, prev.pub_interface_count, self.pub_interface_count
                    ),
                });
            }
        }

        // Dependency count: warn if +1
        let dep_delta = self.dependency_count - prev.dependency_count;
        if dep_delta > 1 {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "dependency count increased by {} ({}→{}), threshold +1",
                    dep_delta, prev.dependency_count, self.dependency_count
                ),
            });
        }

        // Max fan-out: warn if +2
        let fanout_delta = self.coupling_max - prev.coupling_max;
        if fanout_delta > 2 {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "max fan-out increased by {} ({}→{}), threshold +2",
                    fanout_delta, prev.coupling_max, self.coupling_max
                ),
            });
        }

        diags
    }
}

impl CaptureLayerMetrics {
    /// Diagnostics: warn when layer scrape exceeds duration threshold.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        if self.duration_ms > SCRAPE_DURATION_WARNING_MS {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "layer scrape took {}ms (threshold: {}ms)",
                    self.duration_ms, SCRAPE_DURATION_WARNING_MS
                ),
            });
        }
        diags
    }
}

/// Collect diagnostics from a verb's sources.
fn collect_source_diagnostics(sources: &[SourceSummary]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for src in sources {
        match &src.latest_metrics {
            VerbMetrics::Believe(m) => diags.extend(m.diagnostics()),
            VerbMetrics::Search(m) => diags.extend(m.diagnostics()),
            VerbMetrics::CaptureHealthCheck(m) => diags.extend(m.diagnostics()),
            VerbMetrics::CaptureGitScrape(m) => diags.extend(m.diagnostics()),
            VerbMetrics::CaptureBeliefs(m) => diags.extend(m.diagnostics()),
            VerbMetrics::CaptureLayer(m) => diags.extend(m.diagnostics()),
            // CaptureStructure diagnostics need delta (previous event) — handled in build_full_report
            _ => {}
        }
    }
    diags
}

/// Collect structure delta diagnostics by comparing current vs previous events.
fn collect_structure_delta_diagnostics(
    conn: &Connection,
    sources: &[SourceSummary],
) -> Vec<Diagnostic> {
    // Find the current structure metrics from sources
    let current = sources.iter().find_map(|s| {
        if let VerbMetrics::CaptureStructure(m) = &s.latest_metrics {
            Some(m)
        } else {
            None
        }
    });

    let current = match current {
        Some(c) => c,
        None => return Vec::new(),
    };

    // Fetch the previous (second-most-recent) structure event from events.db
    let previous = (|| -> Option<CaptureStructureMetrics> {
        let mut stmt = conn.prepare(
            r#"SELECT json_extract(data, '$.metrics') FROM events.eventlog
               WHERE event_type = 'measure.capture'
                 AND json_extract(data, '$.mode') = 'structure'
               ORDER BY seq DESC LIMIT 1 OFFSET 1"#
        ).ok()?;

        let json_str: String = stmt.query_row([], |row| row.get(0)).ok()?;
        serde_json::from_str(&json_str).ok()
    })();

    current.diagnostics_with_delta(previous.as_ref())
}

/// Derive a freshness diagnostic if the verb is aging or stale.
fn freshness_diagnostic(verb: &str, freshness: Freshness, age_hours: f64) -> Option<Diagnostic> {
    match freshness {
        Freshness::Aging => Some(Diagnostic {
            severity: Severity::Warning,
            message: format!("{} verb data is aging ({:.0}h old)", verb, age_hours),
        }),
        Freshness::Stale => Some(Diagnostic {
            severity: Severity::Error,
            message: format!("{} verb data is stale ({:.0}h old)", verb, age_hours),
        }),
        Freshness::Fresh => None,
    }
}

/// Determine verb status factoring in freshness.
/// Degraded = existing status is NeedsAttention AND freshness is Stale.
fn effective_status(base_status: VerbStatus, freshness: Option<Freshness>) -> VerbStatus {
    match (base_status, freshness) {
        (VerbStatus::NeedsAttention, Some(Freshness::Stale)) => VerbStatus::Degraded,
        (status, _) => status,
    }
}

impl FullMeasureReport {
    /// Construct a complete report — health derived from verbs, no placeholder state.
    pub fn new(
        verbs: std::collections::BTreeMap<String, FullVerbSummary>,
        event_counts: EventCounts,
    ) -> Self {
        let health = Self::derive_health(&verbs);
        Self {
            health,
            verbs,
            event_counts,
        }
    }

    /// Derive health summary from verb map. Private — called by `new()`.
    ///
    /// Worst-verb-wins: Degraded > NeedsAttention > Good > NoData.
    /// The ordering on VerbStatus drives `max()`.
    fn derive_health(verbs: &std::collections::BTreeMap<String, FullVerbSummary>) -> HealthSummary {
        let worst_status = verbs
            .values()
            .map(|v| v.status)
            .max()
            .unwrap_or(VerbStatus::NoData);

        let healthy_count = verbs
            .values()
            .filter(|v| v.status == VerbStatus::Good)
            .count();

        let total = verbs.len();

        // Find the worst verb for the summary sentence
        let worst_reason = verbs
            .iter()
            .filter(|(_, v)| v.status == worst_status && worst_status != VerbStatus::Good)
            .map(|(name, v)| {
                if let Some(diag) = v.diagnostics.first() {
                    format!("{}: {}", name, diag.message)
                } else {
                    format!("{}: {}", name, v.status)
                }
            })
            .next()
            .unwrap_or_default();

        let summary = if worst_status == VerbStatus::Good || worst_status == VerbStatus::NoData {
            format!("{}/{} verbs healthy.", healthy_count, total)
        } else {
            format!(
                "{}/{} verbs healthy. {}",
                healthy_count, total, worst_reason
            )
        };

        HealthSummary {
            status: worst_status,
            summary,
            assessed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ============================================================================
// Full Report Builder
// ============================================================================

/// ATTACH events.db to an open patina.db connection.
///
/// Measure direction: patina.db is primary, events.db is attached.
/// Callers query `events.eventlog` for measure.* events.
fn attach_events(conn: &Connection) -> Result<()> {
    eventlog::ensure_events_db()?;
    let events_path = Path::new(eventlog::EVENTS_DB);
    if events_path.exists() {
        conn.execute(
            "ATTACH DATABASE ?1 AS events",
            [events_path.to_str().unwrap_or(eventlog::EVENTS_DB)],
        )?;
    }
    Ok(())
}

/// Build the full measure report — typed health layer over existing infrastructure.
///
/// Reuses `build_*_summary()` verb builders, then wraps each with freshness,
/// diagnostics, and effective status. Pure library code — no CLI or output.
pub fn build_full_report(conn: &Connection) -> Result<FullMeasureReport> {
    let existing_report = build_report(conn)?;

    let mut verbs = std::collections::BTreeMap::new();

    for verb_summary in existing_report.verbs {
        let verb_name = verb_summary.verb.clone();

        // Compute age and freshness from latest timestamp
        let age_hours = verb_summary
            .latest_timestamp
            .as_deref()
            .and_then(compute_age_hours);

        let freshness = age_hours.map(|h| Freshness::for_verb(&verb_name, h));

        // Collect diagnostics from typed metric sources
        let mut diagnostics = collect_source_diagnostics(&verb_summary.sources);

        // Add freshness diagnostic if aging or stale
        if let (Some(f), Some(h)) = (freshness, age_hours) {
            if let Some(d) = freshness_diagnostic(&verb_name, f, h) {
                diagnostics.push(d);
            }
        }

        let status = effective_status(verb_summary.status, freshness);

        verbs.insert(
            verb_name,
            FullVerbSummary {
                status,
                latest_timestamp: verb_summary.latest_timestamp,
                age_hours,
                freshness,
                sources: verb_summary.sources,
                diagnostics,
            },
        );
    }

    // Structure delta diagnostics — need previous event from events.db
    if let Some(capture_verb) = verbs.get_mut("capture") {
        let structure_diags = collect_structure_delta_diagnostics(conn, &capture_verb.sources);
        capture_verb.diagnostics.extend(structure_diags);
    }

    // Event counts from events.db
    let event_counts = build_event_counts(conn)?;

    Ok(FullMeasureReport::new(verbs, event_counts))
}

/// Count events by type from events.db for the EventCounts field.
fn build_event_counts(conn: &Connection) -> Result<EventCounts> {
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM events.eventlog", [], |row| row.get(0))
        .unwrap_or(0);

    let mut by_type = std::collections::BTreeMap::new();

    let stmt_result = conn.prepare(
        "SELECT event_type, COUNT(*) FROM events.eventlog GROUP BY event_type ORDER BY event_type",
    );

    if let Ok(mut stmt) = stmt_result {
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for (event_type, count) in rows.flatten() {
            by_type.insert(event_type, count);
        }
    }

    Ok(EventCounts {
        total_runtime_events: total,
        by_type,
    })
}

// ============================================================================
// Main Entry Point
// ============================================================================

pub fn run(options: MeasureOptions) -> Result<()> {
    let db_path = Path::new(eventlog::PATINA_DB);
    if !db_path.exists() {
        print_empty_state();
        return Ok(());
    }

    let conn = Connection::open(db_path).context("Failed to open patina.db")?;

    // ATTACH events.db for cross-system queries (measure.* events live there)
    attach_events(&conn)?;

    // Check if ANY measurement data exists
    let total_events = count_measurement_events(&conn)?;
    let has_existing = has_existing_measurement_events(&conn)?;

    if total_events == 0 && !has_existing {
        print_empty_state();
        return Ok(());
    }

    if let Some(ref verb) = options.verb {
        return run_verb_drilldown(&conn, verb, &options);
    }

    if options.full {
        let full_report = build_full_report(&conn)?;
        if options.json {
            println!("{}", serde_json::to_string_pretty(&full_report)?);
        } else {
            render_full_user_view(&full_report);
        }
        return Ok(());
    }

    let report = build_report(&conn)?;

    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if options.system {
        render_system_view(&conn, &report)?;
    } else {
        render_user_view(&report);
    }

    Ok(())
}

// ============================================================================
// Data Collection
// ============================================================================

fn count_measurement_events(conn: &Connection) -> Result<i64> {
    // measure.* events are in events.db (attached as 'events')
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events.eventlog WHERE event_type LIKE 'measure.%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(count)
}

fn has_existing_measurement_events(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM eventlog WHERE event_type IN ('belief.surface', 'session.ended')",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn build_report(conn: &Connection) -> Result<MeasureReport> {
    let mut verbs = Vec::new();
    let mut actions = Vec::new();

    // Capture verb
    let capture = build_capture_summary(conn)?;
    if capture.status == VerbStatus::NoData {
        actions.push("Run `patina scrape` to populate capture metrics".to_string());
    }
    verbs.push(capture);

    // Index verb
    let index = build_index_summary(conn)?;
    if index.status == VerbStatus::NoData {
        actions.push("Run `patina oxidize` to populate index metrics".to_string());
    }
    verbs.push(index);

    // Search verb
    let search = build_search_summary(conn)?;
    if search.status == VerbStatus::NoData {
        actions.push("Run `patina eval` to populate search metrics".to_string());
    }
    verbs.push(search);

    // Believe verb
    let believe = build_believe_summary(conn)?;
    if believe.status == VerbStatus::NoData {
        actions.push("Run `patina scrape` to populate belief metrics".to_string());
    }
    verbs.push(believe);

    // Evolve verb
    let evolve = build_evolve_summary(conn)?;
    if evolve.status == VerbStatus::NoData {
        actions
            .push("Run a session (`patina session start`) to populate evolve metrics".to_string());
    }
    verbs.push(evolve);

    let total_measurement_events = count_measurement_events(conn)?;

    Ok(MeasureReport {
        generated: chrono::Utc::now().to_rfc3339(),
        verbs,
        total_measurement_events,
        actions,
    })
}

/// Build capture verb summary from measure.capture + git.commit
fn build_capture_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();

    // measure.capture events
    collect_measure_sources(conn, "capture", &mut sources)?;

    // git.commit as coverage proxy: count distinct files in recent commits
    let git_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM eventlog WHERE event_type = 'git.commit'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if git_count > 0 {
        let latest_ts: String = conn
            .query_row(
                "SELECT MAX(timestamp) FROM eventlog WHERE event_type = 'git.commit'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();

        // Count distinct files touched in recent commits (by timestamp, not insertion order)
        let file_count: i64 = conn
            .query_row(
                r#"SELECT COUNT(DISTINCT json_each.value)
                   FROM eventlog, json_each(json_extract(data, '$.files'))
                   WHERE event_type = 'git.commit'
                     AND timestamp >= (
                       SELECT timestamp FROM eventlog
                       WHERE event_type = 'git.commit'
                       ORDER BY timestamp DESC
                       LIMIT 1 OFFSET 99
                     )"#,
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        sources.push(SourceSummary {
            source_type: SourceType::GitCommit,
            tool: ToolName::Scrape,
            mode: Mode::Git,
            latest_metrics: VerbMetrics::CaptureGit(CaptureGitMetrics {
                files_tracked: file_count,
                total_commits: git_count,
            }),
            timestamp: latest_ts,
            event_count: git_count,
        });
    }

    let status = determine_status(&sources);
    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "capture".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Build index verb summary from measure.index
fn build_index_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();
    collect_measure_sources(conn, "index", &mut sources)?;

    let status = determine_status(&sources);
    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "index".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Build search verb summary from measure.search
fn build_search_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();
    collect_measure_sources(conn, "search", &mut sources)?;

    let status = if sources.is_empty() {
        VerbStatus::NoData
    } else {
        // Check P@5 threshold if available (Option: None means not measured)
        let has_low_precision = sources.iter().any(|src| {
            if let VerbMetrics::Search(m) = &src.latest_metrics {
                matches!(m.p_at_5, Some(p5) if p5 < 0.4)
            } else {
                false
            }
        });
        if has_low_precision {
            VerbStatus::NeedsAttention
        } else {
            VerbStatus::Good
        }
    };

    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "search".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Build believe verb summary from beliefs table + belief.surface events
fn build_believe_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();

    // Read from beliefs table (has current grounding scores, updated after oxidize)
    // Falls back to belief.surface events if beliefs table doesn't exist
    let has_beliefs_table = conn.prepare("SELECT COUNT(*) FROM beliefs LIMIT 1").is_ok();

    if has_beliefs_table {
        let result = conn.query_row(
            r#"SELECT
                COUNT(*) as total_beliefs,
                COALESCE(SUM(CASE WHEN grounding_score = 0 THEN 1 ELSE 0 END), 0) as floating,
                COALESCE(SUM(CASE WHEN contested_by > 0 THEN 1 ELSE 0 END), 0) as contested,
                COALESCE(AVG(evidence_count), 0) as avg_evidence,
                COALESCE(AVG(health_score), 0) as avg_health
            FROM beliefs"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, f64>(4)?,
                ))
            },
        );

        // Get latest timestamp from belief.surface events, falling back to beliefs table
        let latest_ts: String = conn
            .query_row(
                "SELECT MAX(timestamp) FROM eventlog WHERE event_type = 'belief.surface'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap_or(None)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                // Fallback: use beliefs table's last_activity or revised date
                conn.query_row(
                    "SELECT COALESCE(MAX(last_activity), MAX(revised), '') FROM beliefs",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_default()
            });

        let event_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM eventlog WHERE event_type = 'belief.surface'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if let Ok((total, floating, contested, avg_evidence, avg_health)) = result {
            if total > 0 {
                sources.push(SourceSummary {
                    source_type: SourceType::Beliefs,
                    tool: ToolName::Scrape,
                    mode: Mode::Beliefs,
                    latest_metrics: VerbMetrics::Believe(BelieveMetrics {
                        total_beliefs: total,
                        floating_count: floating,
                        grounded_count: total - floating,
                        contested_count: contested,
                        avg_evidence: (avg_evidence * 100.0).round() / 100.0,
                        avg_health: (avg_health * 100.0).round() / 100.0,
                    }),
                    timestamp: latest_ts,
                    event_count,
                });
            }
        }
    }

    // Also check measure.believe if any exist
    collect_measure_sources(conn, "believe", &mut sources)?;

    let status = if sources.is_empty() {
        VerbStatus::NoData
    } else {
        // Check floating threshold
        let floating_pct = sources
            .iter()
            .find_map(|s| {
                if let VerbMetrics::Believe(m) = &s.latest_metrics {
                    if m.total_beliefs > 0 {
                        Some(m.floating_count as f64 / m.total_beliefs as f64)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or(0.0);

        if floating_pct > 0.1 {
            VerbStatus::NeedsAttention
        } else {
            VerbStatus::Good
        }
    };

    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "believe".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Build evolve verb summary from session.ended (existing events)
fn build_evolve_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();

    let session_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM eventlog WHERE event_type = 'session.ended'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if session_count > 0 {
        // Aggregate recent session metrics
        let result = conn.query_row(
            r#"SELECT
                COUNT(*) as total_sessions,
                COALESCE(SUM(json_extract(data, '$.commits_made')), 0) as total_commits,
                COALESCE(SUM(json_extract(data, '$.files_changed')), 0) as total_files,
                COALESCE(SUM(json_extract(data, '$.beliefs_captured')), 0) as total_beliefs,
                COALESCE(SUM(json_extract(data, '$.patterns_modified')), 0) as total_patterns,
                MAX(timestamp) as latest_ts
            FROM eventlog
            WHERE event_type = 'session.ended'"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        );

        if let Ok((sessions, commits, files, beliefs, patterns, latest_ts)) = result {
            sources.push(SourceSummary {
                source_type: SourceType::SessionEnded,
                tool: ToolName::Session,
                mode: Mode::Lifecycle,
                latest_metrics: VerbMetrics::Evolve(EvolveMetrics {
                    total_sessions: sessions,
                    total_commits: commits,
                    total_files_changed: files,
                    total_beliefs_captured: beliefs,
                    total_patterns_modified: patterns,
                }),
                timestamp: latest_ts,
                event_count: session_count,
            });
        }
    }

    // Also check measure.evolve if any
    collect_measure_sources(conn, "evolve", &mut sources)?;

    let status = determine_status(&sources);
    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "evolve".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Collect measure.* sources for a given verb, grouped by tool:mode
fn collect_measure_sources(
    conn: &Connection,
    verb: &str,
    sources: &mut Vec<SourceSummary>,
) -> Result<()> {
    let event_type = format!("measure.{}", verb);

    // Get distinct tool:mode combinations with latest metrics (from events.db)
    let stmt_result = conn.prepare(
        r#"SELECT
            json_extract(data, '$.tool') as tool,
            json_extract(data, '$.mode') as mode,
            json_extract(data, '$.metrics') as metrics,
            timestamp,
            (SELECT COUNT(*) FROM events.eventlog e2
             WHERE e2.event_type = ?1
               AND json_extract(e2.data, '$.tool') = json_extract(events.eventlog.data, '$.tool')
               AND json_extract(e2.data, '$.mode') = json_extract(events.eventlog.data, '$.mode')
            ) as event_count
        FROM events.eventlog
        WHERE event_type = ?1
          AND seq IN (
            SELECT MAX(seq) FROM events.eventlog
            WHERE event_type = ?1
            GROUP BY json_extract(data, '$.tool'), json_extract(data, '$.mode')
          )
        ORDER BY timestamp DESC"#,
    );

    // If events.db isn't attached or has no data, return empty
    let mut stmt = match stmt_result {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let rows = stmt.query_map([&event_type], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;

    let source_type = match SourceType::from_verb(verb) {
        Some(st) => st,
        None => return Ok(()),
    };

    for row in rows {
        let (tool_str, mode_str, metrics_str, timestamp, count) = row?;

        let tool = match ToolName::from_db_str(&tool_str) {
            Some(t) => t,
            None => {
                tracing::warn!(tool = tool_str, "Unknown tool — skipping source");
                continue;
            }
        };
        let mode = match Mode::from_db_str(&mode_str) {
            Some(m) => m,
            None => {
                tracing::warn!(mode = mode_str, "Unknown mode — skipping source");
                continue;
            }
        };

        let metrics = VerbMetrics::from_db(verb, &mode_str, &metrics_str);

        sources.push(SourceSummary {
            source_type: source_type.clone(),
            tool,
            mode,
            latest_metrics: metrics,
            timestamp,
            event_count: count,
        });
    }

    Ok(())
}

fn determine_status(sources: &[SourceSummary]) -> VerbStatus {
    if sources.is_empty() {
        VerbStatus::NoData
    } else {
        VerbStatus::Good
    }
}

// ============================================================================
// User View (default)
// ============================================================================

fn render_full_user_view(report: &FullMeasureReport) {
    println!("\n  Project Health — Full Report\n");

    // Health summary
    let health_icon = match report.health.status {
        VerbStatus::Good => "+",
        VerbStatus::NeedsAttention => "!",
        VerbStatus::Degraded => "X",
        VerbStatus::NoData => "-",
    };
    println!("  [{}] {}", health_icon, report.health.summary);
    println!();

    // Per-verb details
    for (verb_name, verb) in &report.verbs {
        let icon = match verb.status {
            VerbStatus::Good => "+",
            VerbStatus::NeedsAttention => "!",
            VerbStatus::Degraded => "X",
            VerbStatus::NoData => "-",
        };

        let age = verb
            .latest_timestamp
            .as_deref()
            .map(format_age)
            .unwrap_or_else(|| "never".to_string());

        let freshness_label = verb
            .freshness
            .map(|f| format!(" [{}]", f))
            .unwrap_or_default();

        println!(
            "  [{}] {:<10} {:<18} {}{}",
            icon,
            verb_name,
            verb.status,
            if verb.status != VerbStatus::NoData {
                format!("({})", age)
            } else {
                String::new()
            },
            freshness_label
        );

        // Show key metrics per source (same as user view)
        for src in &verb.sources {
            let summary = user_friendly_metrics(src);
            if !summary.is_empty() {
                println!("        {}", summary);
            }
        }

        // Show diagnostics (capped at 3 per verb)
        let max_diags = 3;
        for diag in verb.diagnostics.iter().take(max_diags) {
            let icon = match diag.severity {
                Severity::Warning => "\u{26a0}", // ⚠
                Severity::Error => "\u{2716}",   // ✖
            };
            println!("        {} {}", icon, diag.message);
        }
        if verb.diagnostics.len() > max_diags {
            println!(
                "        ... {} more diagnostics (see --json)",
                verb.diagnostics.len() - max_diags
            );
        }
    }

    // Event counts
    println!(
        "\n  Events: {} total runtime events",
        report.event_counts.total_runtime_events
    );

    println!();
}

fn render_user_view(report: &MeasureReport) {
    println!("\n  Project Health\n");

    let good_count = report
        .verbs
        .iter()
        .filter(|v| v.status == VerbStatus::Good)
        .count();
    let attention_count = report
        .verbs
        .iter()
        .filter(|v| v.status == VerbStatus::NeedsAttention)
        .count();
    let no_data_count = report
        .verbs
        .iter()
        .filter(|v| v.status == VerbStatus::NoData)
        .count();

    // Overall status line
    if no_data_count == 5 {
        println!("  No measurements yet. Run some tools to get started.\n");
        return;
    }

    println!(
        "  {}/{} verbs reporting{}{}",
        good_count + attention_count,
        5,
        if attention_count > 0 {
            format!(", {} needs attention", attention_count)
        } else {
            String::new()
        },
        if no_data_count > 0 {
            format!(", {} missing", no_data_count)
        } else {
            String::new()
        }
    );
    println!();

    // Per-verb summary
    for verb_summary in &report.verbs {
        let icon = match verb_summary.status {
            VerbStatus::Good => "+",
            VerbStatus::NeedsAttention => "!",
            VerbStatus::Degraded => "X",
            VerbStatus::NoData => "-",
        };

        let age = verb_summary
            .latest_timestamp
            .as_ref()
            .map(|ts| format_age(ts))
            .unwrap_or_else(|| "never".to_string());

        println!(
            "  [{}] {:<10} {:<18} {}",
            icon,
            verb_summary.verb,
            verb_summary.status,
            if verb_summary.status != VerbStatus::NoData {
                format!("({})", age)
            } else {
                String::new()
            }
        );

        // Show key metrics in user-friendly language
        for src in &verb_summary.sources {
            let summary = user_friendly_metrics(src);
            if !summary.is_empty() {
                println!("        {}", summary);
            }
        }
    }

    // Action items
    if !report.actions.is_empty() {
        println!("\n  Actions:");
        for action in &report.actions {
            println!("    - {}", action);
        }
    }

    println!();
}

fn user_friendly_metrics(src: &SourceSummary) -> String {
    match &src.latest_metrics {
        VerbMetrics::CaptureCode(m) => {
            format!(
                "{}: {} files, {} functions parsed",
                src.mode, m.files_parsed, m.functions_found
            )
        }
        VerbMetrics::CaptureGit(m) => {
            format!("{} files tracked in git", m.files_tracked)
        }
        VerbMetrics::CaptureBeliefs(m) => {
            format!(
                "{}: {} beliefs processed, {} verified, {}ms",
                src.mode, m.beliefs_processed, m.beliefs_verified, m.duration_ms
            )
        }
        VerbMetrics::CaptureLayer(m) => {
            format!(
                "{}: {} patterns, {} sessions, {}ms",
                src.mode, m.patterns_processed, m.sessions_processed, m.duration_ms
            )
        }
        VerbMetrics::CaptureGitScrape(m) => {
            format!(
                "{}: {} commits, {} files, {}ms",
                src.mode, m.commits_processed, m.tracked_files, m.duration_ms
            )
        }
        VerbMetrics::CaptureHealthCheck(m) => {
            format!(
                "{}: {} beliefs, {} sessions, {} patterns",
                src.mode, m.beliefs, m.sessions, m.layer_patterns
            )
        }
        VerbMetrics::CaptureStructure(m) => {
            format!(
                "{}: {} modules, {} pub interfaces, {} deps, avg fan-out {:.1}",
                src.mode, m.module_count, m.pub_interface_count, m.dependency_count, m.coupling_avg
            )
        }
        VerbMetrics::Index(m) => {
            format!("{} documents embedded", m.documents_embedded)
        }
        VerbMetrics::Search(m) => match (m.p_at_5, m.mrr, m.recall_at_5) {
            (Some(p5), Some(mrr_val), _) => {
                format!("{}: P@5={:.0}%, MRR={:.2}", src.mode, p5 * 100.0, mrr_val)
            }
            (Some(p5), None, _) => {
                format!("{}: P@5={:.0}%", src.mode, p5 * 100.0)
            }
            (None, _, Some(recall)) => {
                format!("{}: Recall@5={:.0}%", src.mode, recall * 100.0)
            }
            _ => format!("{}: n/a", src.mode),
        },
        VerbMetrics::Believe(m) => {
            let contested = if m.contested_count > 0 {
                format!(", {} contested", m.contested_count)
            } else {
                String::new()
            };
            if m.floating_count > 0 {
                format!(
                    "{} beliefs, {} grounded, {} floating{}, avg health {:.2}",
                    m.total_beliefs, m.grounded_count, m.floating_count, contested, m.avg_health
                )
            } else {
                format!(
                    "{} beliefs, all grounded{}, avg health {:.2}",
                    m.total_beliefs, contested, m.avg_health
                )
            }
        }
        VerbMetrics::Evolve(m) => {
            format!(
                "{} {}, {} {}, {} {} captured",
                m.total_sessions,
                if m.total_sessions == 1 {
                    "session"
                } else {
                    "sessions"
                },
                m.total_commits,
                if m.total_commits == 1 {
                    "commit"
                } else {
                    "commits"
                },
                m.total_beliefs_captured,
                if m.total_beliefs_captured == 1 {
                    "belief"
                } else {
                    "beliefs"
                },
            )
        }
        VerbMetrics::BelieveHistory(_) | VerbMetrics::EvolveHistory(_) => String::new(),
        VerbMetrics::Raw(_) => String::new(),
    }
}

// ============================================================================
// System View (--system)
// ============================================================================

fn render_system_view(conn: &Connection, report: &MeasureReport) -> Result<()> {
    println!("\n  Measurement System — Raw Metrics\n");
    println!(
        "  Total measure.* events: {}",
        report.total_measurement_events
    );
    println!();

    for verb_summary in &report.verbs {
        println!(
            "  === {} ({}) ===",
            verb_summary.verb.to_uppercase(),
            verb_summary.status
        );

        if verb_summary.sources.is_empty() {
            println!("    No data\n");
            continue;
        }

        for src in &verb_summary.sources {
            println!(
                "    Source: {} ({}:{}) — {} events",
                src.source_type, src.tool, src.mode, src.event_count
            );
            println!("    Latest: {}", src.timestamp);

            // Print all metrics with human-readable units
            for (key, val) in src.latest_metrics.format_kv_display() {
                println!("      {}: {}", key, val);
            }
            println!();
        }

        // Show recent history for measure.* sources
        let event_type = format!("measure.{}", verb_summary.verb);
        let history = get_recent_history(conn, &event_type, 5)?;
        if !history.is_empty() {
            println!("    Recent history:");
            for entry in &history {
                println!(
                    "      {} {}:{} {}",
                    &entry.timestamp[..19],
                    entry.tool,
                    entry.mode,
                    format_metrics_inline(&entry.metrics)
                );
            }
            println!();
        }
    }

    Ok(())
}

// ============================================================================
// Verb Drill-Down (--verb <name>)
// ============================================================================

fn run_verb_drilldown(conn: &Connection, verb: &str, options: &MeasureOptions) -> Result<()> {
    // Validate verb
    if !patina::measure::VALID_VERBS.contains(&verb) {
        anyhow::bail!(
            "Unknown verb '{}'. Valid verbs: {}",
            verb,
            patina::measure::VALID_VERBS.join(", ")
        );
    }

    let event_type = format!("measure.{}", verb);
    let history = get_recent_history(conn, &event_type, 20)?;

    // Also get existing event data for this verb
    let existing = match verb {
        "believe" => get_believe_history(conn, 10)?,
        "evolve" => get_evolve_history(conn, 10)?,
        _ => Vec::new(),
    };

    if options.json {
        let output = serde_json::json!({
            "verb": verb,
            "measure_events": history,
            "existing_events": existing,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("\n  {} — Drill-Down\n", verb.to_uppercase());

    // Show current state from source tables (not events) for dual-source verbs
    if verb == "believe" {
        render_believe_current_state(conn);
    }

    if history.is_empty() && existing.is_empty() {
        if verb != "believe" {
            println!("  No measurement data for verb '{}'.\n", verb);
        }
        return Ok(());
    }

    if !history.is_empty() {
        println!("  measure.{} events ({} total):\n", verb, history.len());
        println!(
            "    {:<22} {:<12} {:<12} METRICS",
            "TIMESTAMP", "TOOL", "MODE"
        );
        println!(
            "    {:<22} {:<12} {:<12} ───────",
            "─────────", "────", "────"
        );
        for entry in &history {
            println!(
                "    {:<22} {:<12} {:<12} {}",
                &entry.timestamp[..19.min(entry.timestamp.len())],
                entry.tool,
                entry.mode,
                format_metrics_inline(&entry.metrics)
            );
        }
        println!();
    }

    if !existing.is_empty() {
        let source_name = match verb {
            "believe" => "belief creation by date",
            "evolve" => "session.ended",
            _ => "existing",
        };
        println!("  {} ({} shown):\n", source_name, existing.len());
        println!(
            "    {:<22} {:<12} {:<12} METRICS",
            "TIMESTAMP", "TOOL", "MODE"
        );
        println!(
            "    {:<22} {:<12} {:<12} ───────",
            "─────────", "────", "────"
        );
        for entry in &existing {
            println!(
                "    {:<22} {:<12} {:<12} {}",
                &entry.timestamp[..19.min(entry.timestamp.len())],
                entry.tool,
                entry.mode,
                format_metrics_inline(&entry.metrics)
            );
        }
        println!();
    }

    Ok(())
}

// ============================================================================
// History Queries
// ============================================================================

fn get_recent_history(
    conn: &Connection,
    event_type: &str,
    limit: usize,
) -> Result<Vec<HistoryEntry>> {
    // measure.* events are in events.db
    let stmt_result = conn.prepare(
        r#"SELECT
            timestamp,
            json_extract(data, '$.tool') as tool,
            json_extract(data, '$.mode') as mode,
            json_extract(data, '$.metrics') as metrics
        FROM events.eventlog
        WHERE event_type = ?1
        ORDER BY seq DESC
        LIMIT ?2"#,
    );

    let mut stmt = match stmt_result {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };

    // Extract verb from event_type (e.g., "measure.capture" → "capture")
    let verb = event_type.strip_prefix("measure.").unwrap_or(event_type);

    let entries: Vec<HistoryEntry> = stmt
        .query_map(rusqlite::params![event_type, limit as i64], |row| {
            let tool_str: String = row.get(1)?;
            let mode_str: String = row.get(2)?;
            let metrics_str: String = row.get(3)?;
            let metrics = VerbMetrics::from_db(verb, &mode_str, &metrics_str);
            let tool = ToolName::from_db_str(&tool_str).unwrap_or(ToolName::Scrape);
            let mode = Mode::from_db_str(&mode_str).unwrap_or(Mode::Default);
            Ok(HistoryEntry {
                timestamp: row.get(0)?,
                tool,
                mode,
                metrics,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

fn get_believe_history(conn: &Connection, limit: usize) -> Result<Vec<HistoryEntry>> {
    // Get distinct scrape runs for belief.surface (grouped by timestamp prefix)
    let mut stmt = conn.prepare(
        r#"SELECT
            MAX(timestamp) as latest_ts,
            COUNT(*) as belief_count,
            COALESCE(SUM(CASE WHEN json_extract(data, '$.metrics.grounding.score') = 0 THEN 1 ELSE 0 END), 0) as floating,
            COALESCE(AVG(json_extract(data, '$.metrics.truth.evidence_count')), 0) as avg_evidence
        FROM eventlog
        WHERE event_type = 'belief.surface'
        GROUP BY SUBSTR(timestamp, 1, 16)
        ORDER BY latest_ts DESC
        LIMIT ?1"#,
    )?;

    let entries: Vec<HistoryEntry> = stmt
        .query_map([limit as i64], |row| {
            Ok(HistoryEntry {
                timestamp: row.get(0)?,
                tool: ToolName::Scrape,
                mode: Mode::Beliefs,
                metrics: VerbMetrics::BelieveHistory(BelieveHistoryMetrics {
                    beliefs: row.get(1)?,
                    floating: row.get(2)?,
                    avg_evidence: (row.get::<_, f64>(3)? * 100.0).round() / 100.0,
                }),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

fn get_evolve_history(conn: &Connection, limit: usize) -> Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        r#"SELECT
            timestamp,
            COALESCE(json_extract(data, '$.commits_made'), 0) as commits,
            COALESCE(json_extract(data, '$.files_changed'), 0) as files,
            COALESCE(json_extract(data, '$.beliefs_captured'), 0) as beliefs,
            COALESCE(json_extract(data, '$.patterns_modified'), 0) as patterns
        FROM eventlog
        WHERE event_type = 'session.ended'
        ORDER BY seq DESC
        LIMIT ?1"#,
    )?;

    let entries: Vec<HistoryEntry> = stmt
        .query_map([limit as i64], |row| {
            Ok(HistoryEntry {
                timestamp: row.get(0)?,
                tool: ToolName::Session,
                mode: Mode::Lifecycle,
                metrics: VerbMetrics::EvolveHistory(EvolveHistoryMetrics {
                    commits: row.get(1)?,
                    files: row.get(2)?,
                    beliefs: row.get(3)?,
                    patterns: row.get(4)?,
                }),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

// ============================================================================
// Rendering Helpers
// ============================================================================

fn render_believe_current_state(conn: &Connection) {
    let result = conn.query_row(
        r#"SELECT
            COUNT(*) as total,
            COALESCE(SUM(CASE WHEN grounding_score = 0 THEN 1 ELSE 0 END), 0) as floating,
            COALESCE(SUM(CASE WHEN grounding_score > 0 THEN 1 ELSE 0 END), 0) as grounded,
            COALESCE(AVG(health_score), 0) as avg_health
        FROM beliefs"#,
        [],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        },
    );

    if let Ok((total, floating, grounded, avg_health)) = result {
        if total > 0 {
            println!("  Current state (beliefs table):");
            println!(
                "    {} beliefs, {} grounded, {} floating, avg health {:.2}\n",
                total, grounded, floating, avg_health
            );
        }
    }
}

fn print_empty_state() {
    println!("\n  No measurements recorded yet.\n");
    println!("  Getting started:");
    println!("    1. patina scrape    — capture code and belief metrics");
    println!("    2. patina oxidize   — build embeddings (index metrics)");
    println!("    3. patina eval      — evaluate search quality");
    println!("    4. patina measure   — see project health here\n");
}

fn format_age(timestamp: &str) -> String {
    // Try RFC3339 first (full datetime with timezone)
    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(ts);
        return if duration.num_days() > 0 {
            format!("{}d ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{}h ago", duration.num_hours())
        } else {
            format!("{}m ago", duration.num_minutes())
        };
    }

    // Fall back to date-only (YYYY-MM-DD) — used by belief.surface
    if let Ok(date) = chrono::NaiveDate::parse_from_str(timestamp, "%Y-%m-%d") {
        let today = chrono::Utc::now().date_naive();
        let days = (today - date).num_days();
        return if days == 0 {
            "today".to_string()
        } else {
            format!("{}d ago", days)
        };
    }

    "unknown".to_string()
}

fn format_metrics_inline(metrics: &VerbMetrics) -> String {
    metrics
        .format_kv()
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// MCP Support
// ============================================================================

/// Generate typed health report for MCP tool.
///
/// Returns the same `FullMeasureReport` as `--full --json`.
/// MCP and CLI share one code path — no shape divergence.
pub fn mcp_measure() -> Result<FullMeasureReport> {
    let db_path = Path::new(eventlog::PATINA_DB);
    if !db_path.exists() {
        return Ok(FullMeasureReport::new(
            std::collections::BTreeMap::new(),
            EventCounts {
                total_runtime_events: 0,
                by_type: std::collections::BTreeMap::new(),
            },
        ));
    }

    let conn = Connection::open(db_path).context("Failed to open patina.db")?;
    let _ = attach_events(&conn);

    let total_events = count_measurement_events(&conn)?;
    let has_existing = has_existing_measurement_events(&conn)?;

    if total_events == 0 && !has_existing {
        return Ok(FullMeasureReport::new(
            std::collections::BTreeMap::new(),
            EventCounts {
                total_runtime_events: 0,
                by_type: std::collections::BTreeMap::new(),
            },
        ));
    }

    build_full_report(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_db_unknown_payload_returns_raw() {
        let unknown = r#"{"unknown_key": 42, "another": "value"}"#;

        // Verbs with required struct fields fall to Raw when payload doesn't match
        for (verb, mode) in &[
            ("capture", "code"),     // CaptureCodeMetrics has required i64 fields
            ("index", "default"),    // IndexMetrics has required documents_embedded
            ("believe", "audit"),    // BelieveMetrics has required i64/f64 fields
            ("evolve", "lifecycle"), // EvolveMetrics has required i64 fields
        ] {
            let result = VerbMetrics::from_db(verb, mode, unknown);
            assert!(
                matches!(result, VerbMetrics::Raw(_)),
                "Expected Raw for {}:{}, got {:?}",
                verb,
                mode,
                result
            );
        }

        // Typed capture modes with unknown payload also fall to Raw
        for (verb, mode) in &[
            ("capture", "beliefs"),
            ("capture", "layer"),
            ("capture", "git"),
            ("capture", "health-check"),
        ] {
            let result = VerbMetrics::from_db(verb, mode, unknown);
            assert!(
                matches!(result, VerbMetrics::Raw(_)),
                "Expected Raw for {}:{}, got {:?}",
                verb,
                mode,
                result
            );
        }

        // Unknown capture mode falls to Raw
        let result = VerbMetrics::from_db("capture", "unknown-mode", unknown);
        assert!(matches!(result, VerbMetrics::Raw(_)));

        // SearchMetrics accepts unknown JSON (all fields are Option, unknowns ignored)
        let result = VerbMetrics::from_db("search", "eval", unknown);
        assert!(matches!(result, VerbMetrics::Search(_)));

        // Unknown verb falls to Raw
        let result = VerbMetrics::from_db("unknown", "mode", unknown);
        assert!(matches!(result, VerbMetrics::Raw(_)));
    }

    #[test]
    fn from_db_raw_fallback_serializes_without_panic() {
        let unknown = r#"{"unexpected_field": 99}"#;
        // Use a verb with required fields to trigger Raw fallback
        let raw = VerbMetrics::from_db("index", "default", unknown);
        assert!(matches!(raw, VerbMetrics::Raw(_)));

        // Serialization must not panic and preserves all fields
        let json = serde_json::to_string(&raw).expect("Raw should serialize");
        assert!(json.contains("unexpected_field"));
        assert!(json.contains("99"));
    }

    #[test]
    fn from_db_valid_capture_code() {
        let json = r#"{"files_parsed": 248, "functions_found": 2427, "types_found": 989, "fts_symbols": 6164}"#;
        let result = VerbMetrics::from_db("capture", "code", json);
        assert!(matches!(result, VerbMetrics::CaptureCode(_)));

        if let VerbMetrics::CaptureCode(m) = result {
            assert_eq!(m.files_parsed, 248);
            assert_eq!(m.functions_found, 2427);
        }
    }

    #[test]
    fn from_db_valid_believe() {
        let json = r#"{"total_beliefs": 178, "floating_count": 135, "grounded_count": 43, "avg_evidence": 1.72, "avg_health": 0.88}"#;
        let result = VerbMetrics::from_db("believe", "beliefs", json);
        assert!(matches!(result, VerbMetrics::Believe(_)));
    }

    #[test]
    fn from_db_valid_evolve() {
        let json = r#"{"total_sessions": 675, "total_commits": 2543, "total_files_changed": 50221, "total_beliefs_captured": 247, "total_patterns_modified": 4222}"#;
        let result = VerbMetrics::from_db("evolve", "lifecycle", json);
        assert!(matches!(result, VerbMetrics::Evolve(_)));
    }

    #[test]
    fn from_db_capture_beliefs_typed() {
        let json = r#"{"beliefs_processed": 178, "beliefs_verified": 43, "beliefs_skipped": 0, "supports_edges": 96, "attacks_edges": 82, "values_processed": 10, "duration_ms": 1092}"#;
        let result = VerbMetrics::from_db("capture", "beliefs", json);
        assert!(matches!(result, VerbMetrics::CaptureBeliefs(_)));

        if let VerbMetrics::CaptureBeliefs(m) = result {
            assert_eq!(m.beliefs_processed, 178);
            assert_eq!(m.attacks_edges, 82);
            assert_eq!(m.duration_ms, 1092);
        }
    }

    #[test]
    fn from_db_capture_layer_typed() {
        let json = r#"{"patterns_processed": 12, "sessions_processed": 5, "duration_ms": 250}"#;
        let result = VerbMetrics::from_db("capture", "layer", json);
        assert!(matches!(result, VerbMetrics::CaptureLayer(_)));
    }

    #[test]
    fn from_db_capture_git_scrape_typed() {
        let json = r#"{"commits_processed": 100, "tracked_files": 248, "tags_indexed": 50, "co_change_pairs": 1200, "duration_ms": 3000}"#;
        let result = VerbMetrics::from_db("capture", "git", json);
        assert!(matches!(result, VerbMetrics::CaptureGitScrape(_)));
    }

    #[test]
    fn from_db_capture_health_check_typed() {
        let json = r#"{"beliefs": 178, "sessions": 42, "layer_patterns": 12, "missing_tools": 0, "new_tools": 1}"#;
        let result = VerbMetrics::from_db("capture", "health-check", json);
        assert!(matches!(result, VerbMetrics::CaptureHealthCheck(_)));
    }

    #[test]
    fn from_db_search_option_fields() {
        // All fields present
        let json = r#"{"p_at_5": 0.8, "mrr": 0.75, "recall_at_5": 0.9}"#;
        let result = VerbMetrics::from_db("search", "eval", json);
        if let VerbMetrics::Search(m) = result {
            assert_eq!(m.p_at_5, Some(0.8));
            assert_eq!(m.mrr, Some(0.75));
            assert_eq!(m.recall_at_5, Some(0.9));
        } else {
            panic!("Expected Search variant");
        }

        // Missing fields → None (not 0.0)
        let json = r#"{"p_at_5": 0.6}"#;
        let result = VerbMetrics::from_db("search", "eval", json);
        if let VerbMetrics::Search(m) = result {
            assert_eq!(m.p_at_5, Some(0.6));
            assert_eq!(m.mrr, None);
            assert_eq!(m.recall_at_5, None);
        } else {
            panic!("Expected Search variant");
        }
    }

    #[test]
    fn verb_metrics_untagged_serialization() {
        // Typed variant serializes flat (no wrapper)
        let m = VerbMetrics::CaptureGit(CaptureGitMetrics {
            files_tracked: 200,
            total_commits: 2892,
        });
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["files_tracked"], 200);
        assert_eq!(json["total_commits"], 2892);
        // No "CaptureGit" wrapper key
        assert!(json.get("CaptureGit").is_none());
    }

    #[test]
    fn from_db_believe_returns_summary_not_history() {
        // Proves dispatch paths are disjoint: from_db returns Believe (summary),
        // not BelieveHistory (which is direct-construction only).
        let json = r#"{"total_beliefs": 178, "floating_count": 5, "grounded_count": 173, "avg_evidence": 1.72, "avg_health": 0.88}"#;
        let result = VerbMetrics::from_db("believe", "beliefs", json);
        assert!(matches!(result, VerbMetrics::Believe(_)));
    }

    // ================================================================
    // Full report derivation tests
    // ================================================================

    #[test]
    fn freshness_capture_thresholds() {
        assert_eq!(Freshness::for_verb("capture", 12.0), Freshness::Fresh);
        assert_eq!(Freshness::for_verb("capture", 48.0), Freshness::Aging);
        assert_eq!(Freshness::for_verb("capture", 100.0), Freshness::Stale);
    }

    #[test]
    fn freshness_believe_thresholds() {
        assert_eq!(Freshness::for_verb("believe", 24.0), Freshness::Fresh);
        assert_eq!(Freshness::for_verb("believe", 200.0), Freshness::Aging);
        assert_eq!(Freshness::for_verb("believe", 800.0), Freshness::Stale);
    }

    #[test]
    fn freshness_unknown_verb_uses_default() {
        // Unknown verb gets believe/evolve thresholds (168h/720h)
        assert_eq!(Freshness::for_verb("unknown", 100.0), Freshness::Fresh);
        assert_eq!(Freshness::for_verb("unknown", 500.0), Freshness::Aging);
        assert_eq!(Freshness::for_verb("unknown", 800.0), Freshness::Stale);
    }

    #[test]
    fn believe_diagnostics_floating() {
        let m = BelieveMetrics {
            total_beliefs: 178,
            floating_count: 135,
            grounded_count: 43,
            contested_count: 0,
            avg_evidence: 1.72,
            avg_health: 0.88,
        };
        let diags = m.diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(diags[0]
            .message
            .contains("135 beliefs have no code grounding"));
        assert!(diags[0].message.contains("76% floating"));
        assert!(matches!(diags[0].severity, Severity::Warning));
    }

    #[test]
    fn believe_diagnostics_contested() {
        let m = BelieveMetrics {
            total_beliefs: 178,
            floating_count: 0,
            grounded_count: 178,
            contested_count: 5,
            avg_evidence: 2.0,
            avg_health: 0.9,
        };
        let diags = m.diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(diags[0]
            .message
            .contains("5 beliefs have active attacks without resolution"));
        assert!(matches!(diags[0].severity, Severity::Warning));
    }

    #[test]
    fn believe_diagnostics_floating_and_contested() {
        let m = BelieveMetrics {
            total_beliefs: 100,
            floating_count: 20,
            grounded_count: 80,
            contested_count: 3,
            avg_evidence: 2.0,
            avg_health: 0.85,
        };
        let diags = m.diagnostics();
        assert_eq!(diags.len(), 2);
        assert!(diags[0]
            .message
            .contains("20 beliefs have no code grounding"));
        assert!(diags[1]
            .message
            .contains("3 beliefs have active attacks without resolution"));
    }

    #[test]
    fn believe_diagnostics_all_grounded() {
        let m = BelieveMetrics {
            total_beliefs: 50,
            floating_count: 0,
            grounded_count: 50,
            contested_count: 0,
            avg_evidence: 3.0,
            avg_health: 0.95,
        };
        assert!(m.diagnostics().is_empty());
    }

    #[test]
    fn believe_diagnostics_low_health() {
        let m = BelieveMetrics {
            total_beliefs: 10,
            floating_count: 0,
            grounded_count: 10,
            contested_count: 0,
            avg_evidence: 1.0,
            avg_health: 0.3,
        };
        let diags = m.diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("average belief health is low"));
    }

    #[test]
    fn search_diagnostics_low_precision() {
        let m = SearchMetrics {
            p_at_5: Some(0.2),
            mrr: Some(0.3),
            recall_at_5: None,
        };
        let diags = m.diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("P@5=20%"));
    }

    #[test]
    fn search_diagnostics_good_precision() {
        let m = SearchMetrics {
            p_at_5: Some(0.8),
            mrr: Some(0.75),
            recall_at_5: None,
        };
        assert!(m.diagnostics().is_empty());
    }

    #[test]
    fn health_check_diagnostics_missing_tools() {
        let m = CaptureHealthCheckMetrics {
            beliefs: 178,
            sessions: 42,
            layer_patterns: 12,
            missing_tools: 2,
            new_tools: 0,
        };
        let diags = m.diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("2 expected tools are missing"));
    }

    #[test]
    fn effective_status_degraded_when_stale_and_needs_attention() {
        assert_eq!(
            effective_status(VerbStatus::NeedsAttention, Some(Freshness::Stale)),
            VerbStatus::Degraded
        );
    }

    #[test]
    fn effective_status_preserves_good() {
        assert_eq!(
            effective_status(VerbStatus::Good, Some(Freshness::Stale)),
            VerbStatus::Good
        );
    }

    #[test]
    fn effective_status_preserves_needs_attention_when_fresh() {
        assert_eq!(
            effective_status(VerbStatus::NeedsAttention, Some(Freshness::Fresh)),
            VerbStatus::NeedsAttention
        );
    }

    #[test]
    fn health_summary_worst_verb_wins() {
        let mut verbs = std::collections::BTreeMap::new();
        verbs.insert(
            "capture".to_string(),
            FullVerbSummary {
                status: VerbStatus::Good,
                latest_timestamp: None,
                age_hours: None,
                freshness: None,
                sources: vec![],
                diagnostics: vec![],
            },
        );
        verbs.insert(
            "believe".to_string(),
            FullVerbSummary {
                status: VerbStatus::NeedsAttention,
                latest_timestamp: None,
                age_hours: None,
                freshness: None,
                sources: vec![],
                diagnostics: vec![Diagnostic {
                    severity: Severity::Warning,
                    message: "135 beliefs floating".to_string(),
                }],
            },
        );

        let report = FullMeasureReport::new(
            verbs,
            EventCounts {
                total_runtime_events: 0,
                by_type: std::collections::BTreeMap::new(),
            },
        );

        assert_eq!(report.health.status, VerbStatus::NeedsAttention);
        assert!(report.health.summary.contains("1/2 verbs healthy"));
        assert!(report.health.summary.contains("believe"));
    }

    #[test]
    fn health_summary_all_good() {
        let mut verbs = std::collections::BTreeMap::new();
        verbs.insert(
            "capture".to_string(),
            FullVerbSummary {
                status: VerbStatus::Good,
                latest_timestamp: None,
                age_hours: None,
                freshness: None,
                sources: vec![],
                diagnostics: vec![],
            },
        );

        let report = FullMeasureReport::new(
            verbs,
            EventCounts {
                total_runtime_events: 0,
                by_type: std::collections::BTreeMap::new(),
            },
        );

        assert_eq!(report.health.status, VerbStatus::Good);
        assert!(report.health.summary.contains("1/1 verbs healthy"));
    }

    #[test]
    fn compute_age_hours_rfc3339() {
        // 1 hour ago
        let ts = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let age = compute_age_hours(&ts).unwrap();
        assert!(age >= 0.9 && age <= 1.1, "Expected ~1.0h, got {}", age);
    }

    #[test]
    fn compute_age_hours_date_only() {
        let yesterday = (chrono::Utc::now().date_naive() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let age = compute_age_hours(&yesterday).unwrap();
        assert!(age >= 23.0 && age <= 25.0, "Expected ~24h, got {}", age);
    }

    #[test]
    fn compute_age_hours_invalid() {
        assert!(compute_age_hours("not-a-date").is_none());
    }

    #[test]
    fn verb_status_ordering() {
        // Worst-verb-wins: Degraded > NeedsAttention > Good > NoData
        assert!(VerbStatus::Degraded > VerbStatus::NeedsAttention);
        assert!(VerbStatus::NeedsAttention > VerbStatus::Good);
        assert!(VerbStatus::Good > VerbStatus::NoData);

        // max() returns worst status
        let statuses = vec![
            VerbStatus::Good,
            VerbStatus::NoData,
            VerbStatus::NeedsAttention,
        ];
        assert_eq!(statuses.into_iter().max(), Some(VerbStatus::NeedsAttention));
    }

    /// JSON shape contract test for FullMeasureReport.
    ///
    /// Pins the serialized structure that LLMs and MCP consumers depend on.
    /// If this test breaks, the JSON contract changed — verify intentionally.
    #[test]
    fn full_measure_report_json_shape() {
        let mut verbs = std::collections::BTreeMap::new();
        verbs.insert(
            "believe".to_string(),
            FullVerbSummary {
                status: VerbStatus::NeedsAttention,
                latest_timestamp: Some("2026-02-27T09:00:00Z".to_string()),
                age_hours: Some(2.0),
                freshness: Some(Freshness::Fresh),
                sources: vec![SourceSummary {
                    source_type: SourceType::Beliefs,
                    tool: ToolName::Scrape,
                    mode: Mode::Beliefs,
                    latest_metrics: VerbMetrics::Believe(BelieveMetrics {
                        total_beliefs: 178,
                        floating_count: 135,
                        grounded_count: 43,
                        contested_count: 16,
                        avg_evidence: 1.72,
                        avg_health: 0.88,
                    }),
                    timestamp: "2026-02-27T09:00:00Z".to_string(),
                    event_count: 178,
                }],
                diagnostics: vec![Diagnostic {
                    severity: Severity::Warning,
                    message: "135 beliefs have no code grounding (76% floating)".to_string(),
                }],
            },
        );
        verbs.insert(
            "capture".to_string(),
            FullVerbSummary {
                status: VerbStatus::Good,
                latest_timestamp: Some("2026-02-27T10:45:00Z".to_string()),
                age_hours: Some(0.25),
                freshness: Some(Freshness::Fresh),
                sources: vec![],
                diagnostics: vec![],
            },
        );

        let mut by_type = std::collections::BTreeMap::new();
        by_type.insert("measure.capture".to_string(), 12);
        by_type.insert("scry.query".to_string(), 45);

        let report = FullMeasureReport::new(
            verbs,
            EventCounts {
                total_runtime_events: 142,
                by_type,
            },
        );

        let json = serde_json::to_value(&report).unwrap();

        // Top-level structure
        assert!(json.get("health").is_some(), "missing top-level 'health'");
        assert!(json.get("verbs").is_some(), "missing top-level 'verbs'");
        assert!(
            json.get("event_counts").is_some(),
            "missing top-level 'event_counts'"
        );

        // Health shape
        let health = &json["health"];
        assert_eq!(health["status"], "needs_attention");
        assert!(health["summary"]
            .as_str()
            .unwrap()
            .contains("1/2 verbs healthy"));
        assert!(health["assessed_at"].as_str().is_some());

        // Verb keys are alphabetically ordered (BTreeMap)
        let verb_keys: Vec<&str> = json["verbs"]
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        assert_eq!(verb_keys, vec!["believe", "capture"]);

        // Verb shape — believe
        let believe = &json["verbs"]["believe"];
        assert_eq!(believe["status"], "needs_attention");
        assert_eq!(believe["latest_timestamp"], "2026-02-27T09:00:00Z");
        assert_eq!(believe["age_hours"], 2.0);
        assert_eq!(believe["freshness"], "fresh");
        assert!(believe["sources"].as_array().is_some());
        assert!(believe["diagnostics"].as_array().is_some());

        // Source shape
        let src = &believe["sources"][0];
        assert_eq!(src["source_type"], "beliefs");
        assert_eq!(src["tool"], "scrape");
        assert_eq!(src["mode"], "beliefs");
        assert_eq!(src["event_count"], 178);
        let metrics = &src["latest_metrics"];
        assert_eq!(metrics["total_beliefs"], 178);
        assert_eq!(metrics["floating_count"], 135);
        assert_eq!(metrics["grounded_count"], 43);
        assert_eq!(metrics["contested_count"], 16);

        // Diagnostic shape
        let diag = &believe["diagnostics"][0];
        assert_eq!(diag["severity"], "warning");
        assert!(diag["message"].as_str().unwrap().contains("floating"));

        // Verb shape — capture (no data verb with Good)
        let capture = &json["verbs"]["capture"];
        assert_eq!(capture["status"], "good");
        assert_eq!(capture["freshness"], "fresh");

        // Event counts shape
        let ec = &json["event_counts"];
        assert_eq!(ec["total_runtime_events"], 142);
        assert_eq!(ec["by_type"]["measure.capture"], 12);
        assert_eq!(ec["by_type"]["scry.query"], 45);

        // VerbStatus serializes as snake_case
        assert_eq!(
            serde_json::to_value(VerbStatus::NeedsAttention).unwrap(),
            "needs_attention"
        );
        assert_eq!(serde_json::to_value(VerbStatus::NoData).unwrap(), "no_data");
        assert_eq!(
            serde_json::to_value(VerbStatus::Degraded).unwrap(),
            "degraded"
        );
        assert_eq!(serde_json::to_value(VerbStatus::Good).unwrap(), "good");

        // Freshness serializes as lowercase
        assert_eq!(serde_json::to_value(Freshness::Fresh).unwrap(), "fresh");
        assert_eq!(serde_json::to_value(Freshness::Aging).unwrap(), "aging");
        assert_eq!(serde_json::to_value(Freshness::Stale).unwrap(), "stale");

        // Severity serializes as lowercase
        assert_eq!(serde_json::to_value(Severity::Warning).unwrap(), "warning");
        assert_eq!(serde_json::to_value(Severity::Error).unwrap(), "error");
    }

    #[test]
    fn git_scrape_diagnostics_slow() {
        let m = CaptureGitScrapeMetrics {
            commits_processed: 10,
            tracked_files: 248,
            tags_indexed: 50,
            co_change_pairs: 1200,
            duration_ms: 6000,
        };
        let diags = m.diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("git scrape took 6000ms"));
        assert!(matches!(diags[0].severity, Severity::Warning));
    }

    #[test]
    fn git_scrape_diagnostics_fast() {
        let m = CaptureGitScrapeMetrics {
            commits_processed: 10,
            tracked_files: 248,
            tags_indexed: 50,
            co_change_pairs: 1200,
            duration_ms: 2000,
        };
        assert!(m.diagnostics().is_empty());
    }

    #[test]
    fn layer_scrape_diagnostics_slow() {
        let m = CaptureLayerMetrics {
            patterns_processed: 12,
            sessions_processed: 5,
            duration_ms: 7000,
        };
        let diags = m.diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("layer scrape took 7000ms"));
    }

    #[test]
    fn beliefs_scrape_diagnostics_fast() {
        let m = CaptureBeliefsMetrics {
            beliefs_processed: 178,
            beliefs_verified: 43,
            beliefs_skipped: 0,
            supports_edges: 96,
            attacks_edges: 82,
            values_processed: 10,
            duration_ms: 1000,
        };
        assert!(m.diagnostics().is_empty());
    }
}
