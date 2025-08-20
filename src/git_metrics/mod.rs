//! Git metrics collection system for tracking code evolution and pattern survival
//! 
//! This module provides comprehensive Git analytics including:
//! - Code survival rates (how long code lives)
//! - Co-modification patterns (what changes together)
//! - Author collaboration networks
//! - Pattern evolution tracking
//! - Session impact metrics

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod survival;
pub mod comodification;
pub mod evolution;
pub mod session;

/// Core metrics collector that orchestrates all Git analytics
pub struct GitMetrics {
    repo_path: PathBuf,
    cache: MetricsCache,
}

/// Cached metrics to avoid expensive Git operations
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MetricsCache {
    pub last_updated: Option<DateTime<Utc>>,
    pub commit_count: usize,
    pub file_metrics: HashMap<PathBuf, FileMetrics>,
    pub pattern_metrics: HashMap<String, PatternMetrics>,
}

/// Metrics for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetrics {
    pub path: PathBuf,
    pub first_seen: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub commit_count: usize,
    pub author_count: usize,
    pub survival_score: f64,  // 0.0 (deleted quickly) to 1.0 (eternal)
    pub comodified_with: Vec<PathBuf>,
    pub pattern_references: Vec<String>,
}

/// Metrics for a pattern (from layer/)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMetrics {
    pub name: String,
    pub category: PatternCategory,
    pub first_documented: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub reference_count: usize,
    pub survival_rate: f64,  // How often pattern survives refactors
    pub adoption_curve: Vec<(DateTime<Utc>, usize)>,  // Usage over time
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternCategory {
    Core,     // Eternal patterns
    Surface,  // Active development
    Dust,     // Historical
    Session,  // Session-specific
}

/// Git commit with metrics-relevant data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMetrics {
    pub sha: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub files_changed: Vec<PathBuf>,
    pub insertions: usize,
    pub deletions: usize,
    pub session_tag: Option<String>,  // If part of a session
}

/// Co-modification cluster (files that change together)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComodificationCluster {
    pub files: HashSet<PathBuf>,
    pub frequency: usize,
    pub confidence: f64,  // 0.0 to 1.0
    pub last_seen: DateTime<Utc>,
}

impl GitMetrics {
    /// Create a new metrics collector for the repository
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        
        // Verify it's a Git repository
        if !repo_path.join(".git").exists() {
            anyhow::bail!("Not a Git repository: {}", repo_path.display());
        }
        
        Ok(Self {
            repo_path,
            cache: MetricsCache::default(),
        })
    }
    
    /// Load cached metrics from disk
    pub fn load_cache(&mut self) -> Result<()> {
        let cache_path = self.repo_path.join(".patina/metrics_cache.json");
        if cache_path.exists() {
            let content = std::fs::read_to_string(&cache_path)
                .context("Failed to read metrics cache")?;
            self.cache = serde_json::from_str(&content)
                .context("Failed to parse metrics cache")?;
        }
        Ok(())
    }
    
    /// Save metrics cache to disk
    pub fn save_cache(&self) -> Result<()> {
        let cache_dir = self.repo_path.join(".patina");
        std::fs::create_dir_all(&cache_dir)?;
        
        let cache_path = cache_dir.join("metrics_cache.json");
        let content = serde_json::to_string_pretty(&self.cache)?;
        std::fs::write(&cache_path, content)
            .context("Failed to write metrics cache")?;
        
        Ok(())
    }
    
    /// Collect all metrics (expensive operation)
    pub fn collect_all(&mut self) -> Result<MetricsReport> {
        println!("🔍 Collecting Git metrics...");
        
        // Collect various metrics
        let commits = self.get_commit_metrics()?;
        let file_metrics = self.analyze_file_metrics(&commits)?;
        let comod_clusters = self.find_comodification_clusters(&commits)?;
        let pattern_evolution = self.track_pattern_evolution()?;
        let session_metrics = self.analyze_session_metrics(&commits)?;
        
        // Update cache
        self.cache.last_updated = Some(Utc::now());
        self.cache.commit_count = commits.len();
        self.cache.file_metrics = file_metrics.clone();
        
        // Save cache
        self.save_cache()?;
        
        Ok(MetricsReport {
            timestamp: Utc::now(),
            total_commits: commits.len(),
            file_metrics,
            comodification_clusters: comod_clusters,
            pattern_evolution,
            session_metrics,
        })
    }
    
    /// Get commit metrics from Git log
    fn get_commit_metrics(&self) -> Result<Vec<CommitMetrics>> {
        let output = Command::new("git")
            .arg("log")
            .arg("--format=%H|%an|%aI|%s")
            .arg("--numstat")
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to run git log")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commits = Vec::new();
        let mut current_commit: Option<CommitMetrics> = None;
        
        for line in stdout.lines() {
            if line.contains('|') && !line.starts_with('\t') {
                // Commit header line
                if let Some(commit) = current_commit.take() {
                    commits.push(commit);
                }
                
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() == 4 {
                    current_commit = Some(CommitMetrics {
                        sha: parts[0].to_string(),
                        author: parts[1].to_string(),
                        timestamp: DateTime::parse_from_rfc3339(parts[2])
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        message: parts[3].to_string(),
                        files_changed: Vec::new(),
                        insertions: 0,
                        deletions: 0,
                        session_tag: Self::extract_session_tag(parts[3]),
                    });
                }
            } else if line.starts_with('\t') {
                // Numstat line
                if let Some(ref mut commit) = current_commit {
                    let parts: Vec<&str> = line.trim().split('\t').collect();
                    if parts.len() == 3 {
                        let insertions = parts[0].parse::<usize>().unwrap_or(0);
                        let deletions = parts[1].parse::<usize>().unwrap_or(0);
                        commit.insertions += insertions;
                        commit.deletions += deletions;
                        commit.files_changed.push(PathBuf::from(parts[2]));
                    }
                }
            }
        }
        
        if let Some(commit) = current_commit {
            commits.push(commit);
        }
        
        Ok(commits)
    }
    
    /// Extract session tag from commit message if present
    fn extract_session_tag(message: &str) -> Option<String> {
        if message.contains("session-") {
            // Look for session-YYYYMMDD-HHMMSS pattern
            let re = regex::Regex::new(r"session-\d{8}-\d{6}").ok()?;
            re.find(message).map(|m| m.as_str().to_string())
        } else {
            None
        }
    }
    
    /// Analyze file-level metrics
    fn analyze_file_metrics(&self, commits: &[CommitMetrics]) -> Result<HashMap<PathBuf, FileMetrics>> {
        let mut file_metrics = HashMap::new();
        
        for commit in commits {
            for file in &commit.files_changed {
                let entry = file_metrics.entry(file.clone()).or_insert_with(|| {
                    FileMetrics {
                        path: file.clone(),
                        first_seen: commit.timestamp,
                        last_modified: commit.timestamp,
                        commit_count: 0,
                        author_count: 0,
                        survival_score: 0.0,
                        comodified_with: Vec::new(),
                        pattern_references: Vec::new(),
                    }
                });
                
                entry.commit_count += 1;
                entry.last_modified = entry.last_modified.max(commit.timestamp);
                entry.first_seen = entry.first_seen.min(commit.timestamp);
            }
        }
        
        // Calculate survival scores
        for metrics in file_metrics.values_mut() {
            let age_days = (Utc::now() - metrics.first_seen).num_days() as f64;
            let modification_rate = metrics.commit_count as f64 / age_days.max(1.0);
            
            // Higher survival score for older files with fewer modifications
            metrics.survival_score = (age_days / 365.0).min(1.0) * (1.0 / (1.0 + modification_rate));
        }
        
        Ok(file_metrics)
    }
    
    /// Find files that frequently change together
    fn find_comodification_clusters(&self, commits: &[CommitMetrics]) -> Result<Vec<ComodificationCluster>> {
        comodification::find_clusters(commits)
    }
    
    /// Track pattern evolution over time
    fn track_pattern_evolution(&self) -> Result<HashMap<String, PatternEvolution>> {
        evolution::track_patterns(&self.repo_path)
    }
    
    /// Analyze session-specific metrics
    fn analyze_session_metrics(&self, commits: &[CommitMetrics]) -> Result<SessionMetrics> {
        session::analyze_sessions(commits)
    }
}

/// Complete metrics report
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsReport {
    pub timestamp: DateTime<Utc>,
    pub total_commits: usize,
    pub file_metrics: HashMap<PathBuf, FileMetrics>,
    pub comodification_clusters: Vec<ComodificationCluster>,
    pub pattern_evolution: HashMap<String, PatternEvolution>,
    pub session_metrics: SessionMetrics,
}

/// Pattern evolution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvolution {
    pub name: String,
    pub milestones: Vec<EvolutionMilestone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionMilestone {
    pub timestamp: DateTime<Utc>,
    pub event: String,
    pub impact: ImpactLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Session-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_sessions: usize,
    pub avg_session_commits: f64,
    pub avg_session_duration_hours: f64,
    pub most_productive_sessions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_creation() {
        // This will fail if not in a git repo, which is expected
        let result = GitMetrics::new("/tmp/not-a-repo");
        assert!(result.is_err());
    }
}