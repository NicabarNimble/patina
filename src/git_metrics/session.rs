//! Session-specific metrics for tracking development sessions

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

use crate::git_metrics::{CommitMetrics, SessionMetrics};

/// Analyze session-level metrics
pub fn analyze_sessions(commits: &[CommitMetrics]) -> Result<SessionMetrics> {
    let mut sessions: HashMap<String, SessionData> = HashMap::new();
    
    // Group commits by session
    for commit in commits {
        if let Some(ref session_tag) = commit.session_tag {
            let session = sessions.entry(session_tag.clone()).or_insert_with(|| {
                SessionData {
                    tag: session_tag.clone(),
                    commits: Vec::new(),
                    start_time: commit.timestamp,
                    end_time: commit.timestamp,
                    files_changed: 0,
                    lines_added: 0,
                    lines_deleted: 0,
                }
            });
            
            session.commits.push(commit.clone());
            session.start_time = session.start_time.min(commit.timestamp);
            session.end_time = session.end_time.max(commit.timestamp);
            session.files_changed += commit.files_changed.len();
            session.lines_added += commit.insertions;
            session.lines_deleted += commit.deletions;
        }
    }
    
    // Calculate statistics
    let total_sessions = sessions.len();
    let avg_session_commits = if total_sessions > 0 {
        sessions.values().map(|s| s.commits.len()).sum::<usize>() as f64 / total_sessions as f64
    } else {
        0.0
    };
    
    let avg_session_duration_hours = if total_sessions > 0 {
        sessions.values()
            .map(|s| (s.end_time - s.start_time).num_hours() as f64)
            .sum::<f64>() / total_sessions as f64
    } else {
        0.0
    };
    
    // Find most productive sessions (by impact score)
    let mut productive_sessions: Vec<(String, f64)> = sessions.iter()
        .map(|(tag, data)| {
            let impact_score = calculate_impact_score(data);
            (tag.clone(), impact_score)
        })
        .collect();
    
    productive_sessions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    let most_productive_sessions = productive_sessions.iter()
        .take(5)
        .map(|(tag, _)| tag.clone())
        .collect();
    
    Ok(SessionMetrics {
        total_sessions,
        avg_session_commits,
        avg_session_duration_hours,
        most_productive_sessions,
    })
}

#[derive(Debug, Clone)]
struct SessionData {
    tag: String,
    commits: Vec<CommitMetrics>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    files_changed: usize,
    lines_added: usize,
    lines_deleted: usize,
}

/// Calculate impact score for a session
fn calculate_impact_score(session: &SessionData) -> f64 {
    let commit_score = session.commits.len() as f64 * 10.0;
    let code_score = (session.lines_added + session.lines_deleted) as f64 * 0.1;
    let file_score = session.files_changed as f64 * 5.0;
    
    // Bonus for focused sessions (shorter duration with high output)
    let duration_hours = (session.end_time - session.start_time).num_hours() as f64;
    let focus_bonus = if duration_hours > 0.0 && duration_hours < 4.0 {
        100.0 / duration_hours
    } else {
        0.0
    };
    
    commit_score + code_score + file_score + focus_bonus
}

/// Generate session metrics report
pub fn generate_session_report(
    metrics: &SessionMetrics,
    sessions: &HashMap<String, SessionData>,
) -> String {
    let mut report = String::from("# Session Metrics Report\n\n");
    
    report.push_str(&format!("## Overview\n"));
    report.push_str(&format!("- Total sessions: {}\n", metrics.total_sessions));
    report.push_str(&format!("- Average commits per session: {:.1}\n", metrics.avg_session_commits));
    report.push_str(&format!("- Average session duration: {:.1} hours\n\n", 
        metrics.avg_session_duration_hours));
    
    report.push_str("## Most Productive Sessions\n");
    for (i, session_tag) in metrics.most_productive_sessions.iter().enumerate() {
        if let Some(session) = sessions.get(session_tag) {
            let duration = (session.end_time - session.start_time).num_hours();
            report.push_str(&format!("{}. {} ({} commits, {} hours, {} files)\n",
                i + 1,
                session_tag,
                session.commits.len(),
                duration,
                session.files_changed
            ));
            
            // Show session highlights
            if let Some(first_commit) = session.commits.first() {
                report.push_str(&format!("   Started: {}\n", 
                    first_commit.timestamp.format("%Y-%m-%d %H:%M")));
            }
            report.push_str(&format!("   Impact: +{} -{} lines\n",
                session.lines_added,
                session.lines_deleted
            ));
        }
    }
    
    report.push_str("\n## Session Patterns\n");
    
    // Analyze session patterns
    let morning_sessions = sessions.values()
        .filter(|s| s.start_time.hour() >= 6 && s.start_time.hour() < 12)
        .count();
    let afternoon_sessions = sessions.values()
        .filter(|s| s.start_time.hour() >= 12 && s.start_time.hour() < 18)
        .count();
    let evening_sessions = sessions.values()
        .filter(|s| s.start_time.hour() >= 18 || s.start_time.hour() < 6)
        .count();
    
    report.push_str(&format!("- Morning sessions (6am-12pm): {}\n", morning_sessions));
    report.push_str(&format!("- Afternoon sessions (12pm-6pm): {}\n", afternoon_sessions));
    report.push_str(&format!("- Evening sessions (6pm-6am): {}\n", evening_sessions));
    
    // Find longest session
    if let Some(longest) = sessions.values()
        .max_by_key(|s| (s.end_time - s.start_time).num_seconds()) {
        let duration = (longest.end_time - longest.start_time).num_hours();
        report.push_str(&format!("\nLongest session: {} ({} hours)\n", 
            longest.tag, duration));
    }
    
    // Find most commits in a session
    if let Some(most_commits) = sessions.values()
        .max_by_key(|s| s.commits.len()) {
        report.push_str(&format!("Most commits in a session: {} ({} commits)\n",
            most_commits.tag, most_commits.commits.len()));
    }
    
    report
}