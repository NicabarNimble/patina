//! Git metrics command - analyze code evolution and patterns

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

use patina::git_metrics::{GitMetrics, survival, comodification, evolution};

#[derive(Debug, Args)]
pub struct MetricsArgs {
    #[command(subcommand)]
    pub command: MetricsCommand,
}

#[derive(Debug, Subcommand)]
pub enum MetricsCommand {
    /// Analyze code survival rates
    Survival {
        /// Output format (text, json, markdown)
        #[arg(short, long, default_value = "text")]
        format: String,
        
        /// Show only top N results
        #[arg(short = 'n', long)]
        top: Option<usize>,
    },
    
    /// Find co-modification patterns
    Comodification {
        /// Minimum confidence threshold (0.0-1.0)
        #[arg(short, long, default_value = "0.3")]
        confidence: f64,
        
        /// Output format (text, json, markdown)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    
    /// Track pattern evolution
    Evolution {
        /// Specific pattern to track
        #[arg(short, long)]
        pattern: Option<String>,
        
        /// Output format (text, json, markdown)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    
    /// Analyze development sessions
    Sessions {
        /// Show detailed session info
        #[arg(short, long)]
        detailed: bool,
        
        /// Output format (text, json, markdown)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    
    /// Generate comprehensive metrics report
    Report {
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Include all metrics
        #[arg(short, long)]
        all: bool,
    },
}

pub fn execute(args: MetricsArgs) -> Result<()> {
    let repo_path = std::env::current_dir()?;
    
    match args.command {
        MetricsCommand::Survival { format, top } => {
            analyze_survival(&repo_path, &format, top)?;
        }
        MetricsCommand::Comodification { confidence, format } => {
            analyze_comodification(&repo_path, confidence, &format)?;
        }
        MetricsCommand::Evolution { pattern, format } => {
            analyze_evolution(&repo_path, pattern, &format)?;
        }
        MetricsCommand::Sessions { detailed, format } => {
            analyze_sessions(&repo_path, detailed, &format)?;
        }
        MetricsCommand::Report { output, all } => {
            generate_report(&repo_path, output, all)?;
        }
    }
    
    Ok(())
}

fn analyze_survival(repo_path: &PathBuf, format: &str, top: Option<usize>) -> Result<()> {
    println!("🔍 Analyzing code survival rates...");
    
    let mut analyses = survival::analyze_survival(repo_path)?;
    
    if let Some(n) = top {
        analyses.truncate(n);
    }
    
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&analyses)?);
        }
        "markdown" => {
            let report = survival::generate_survival_report(&analyses);
            println!("{}", report);
        }
        _ => {
            // Text format
            println!("\n📊 Code Survival Analysis\n");
            println!("{:<50} {:>10} {:>10} {:>10}", "File", "Age (days)", "Status", "Survival");
            println!("{}", "-".repeat(80));
            
            for analysis in &analyses {
                let status = if analysis.death_date.is_none() {
                    "Alive"
                } else {
                    "Dead"
                };
                
                println!("{:<50} {:>10} {:>10} {:>9.1}%",
                    truncate_path(&analysis.file.to_string_lossy(), 50),
                    analysis.lifespan_days,
                    status,
                    analysis.survival_rate * 100.0
                );
            }
        }
    }
    
    Ok(())
}

fn analyze_comodification(repo_path: &PathBuf, min_confidence: f64, format: &str) -> Result<()> {
    println!("🔍 Finding co-modification patterns...");
    
    let mut metrics = GitMetrics::new(repo_path)?;
    let commits = metrics.get_commit_metrics()?;
    let clusters = comodification::find_clusters(&commits)?;
    
    // Filter by confidence
    let filtered: Vec<_> = clusters.into_iter()
        .filter(|c| c.confidence >= min_confidence)
        .collect();
    
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&filtered)?);
        }
        "markdown" => {
            let report = comodification::generate_comodification_report(&filtered);
            println!("{}", report);
        }
        _ => {
            // Text format
            println!("\n🔗 Co-modification Clusters\n");
            
            for (i, cluster) in filtered.iter().enumerate() {
                println!("Cluster {} (Confidence: {:.1}%, Frequency: {})",
                    i + 1,
                    cluster.confidence * 100.0,
                    cluster.frequency
                );
                
                for file in &cluster.files {
                    println!("  - {}", file.display());
                }
                println!();
            }
        }
    }
    
    Ok(())
}

fn analyze_evolution(repo_path: &PathBuf, pattern: Option<String>, format: &str) -> Result<()> {
    println!("🔍 Tracking pattern evolution...");
    
    let evolutions = evolution::track_patterns(repo_path)?;
    
    let filtered = if let Some(p) = pattern {
        evolutions.into_iter()
            .filter(|(name, _)| name.contains(&p))
            .collect()
    } else {
        evolutions
    };
    
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&filtered)?);
        }
        "markdown" => {
            let report = evolution::generate_evolution_report(&filtered);
            println!("{}", report);
        }
        _ => {
            // Text format
            println!("\n📈 Pattern Evolution\n");
            
            for (name, evolution) in &filtered {
                println!("Pattern: {}", name);
                println!("Milestones: {}", evolution.milestones.len());
                
                for milestone in evolution.milestones.iter().take(5) {
                    println!("  {} - {}",
                        milestone.timestamp.format("%Y-%m-%d"),
                        milestone.event
                    );
                }
                println!();
            }
        }
    }
    
    Ok(())
}

fn analyze_sessions(repo_path: &PathBuf, detailed: bool, format: &str) -> Result<()> {
    println!("🔍 Analyzing development sessions...");
    
    let mut metrics = GitMetrics::new(repo_path)?;
    let commits = metrics.get_commit_metrics()?;
    let session_metrics = patina::git_metrics::session::analyze_sessions(&commits)?;
    
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&session_metrics)?);
        }
        _ => {
            // Text format
            println!("\n📊 Session Metrics\n");
            println!("Total sessions: {}", session_metrics.total_sessions);
            println!("Average commits per session: {:.1}", session_metrics.avg_session_commits);
            println!("Average session duration: {:.1} hours", session_metrics.avg_session_duration_hours);
            
            if !session_metrics.most_productive_sessions.is_empty() {
                println!("\nMost productive sessions:");
                for (i, session) in session_metrics.most_productive_sessions.iter().enumerate() {
                    println!("  {}. {}", i + 1, session);
                }
            }
            
            if detailed {
                // TODO: Add more detailed session analysis
                println!("\n(Use --format=markdown for detailed report)");
            }
        }
    }
    
    Ok(())
}

fn generate_report(repo_path: &PathBuf, output: Option<PathBuf>, all: bool) -> Result<()> {
    println!("📊 Generating comprehensive metrics report...");
    
    let mut metrics = GitMetrics::new(repo_path)?;
    metrics.load_cache()?;
    
    let report = if all {
        metrics.collect_all()?
    } else {
        // Quick report from cache
        let commits = metrics.get_commit_metrics()?;
        let file_metrics = metrics.analyze_file_metrics(&commits)?;
        let comod_clusters = metrics.find_comodification_clusters(&commits)?;
        let pattern_evolution = metrics.track_pattern_evolution()?;
        let session_metrics = metrics.analyze_session_metrics(&commits)?;
        
        patina::git_metrics::MetricsReport {
            timestamp: chrono::Utc::now(),
            total_commits: commits.len(),
            file_metrics,
            comodification_clusters: comod_clusters,
            pattern_evolution,
            session_metrics,
        }
    };
    
    metrics.save_cache()?;
    
    // Generate markdown report
    let mut markdown = String::from("# Patina Git Metrics Report\n\n");
    markdown.push_str(&format!("Generated: {}\n", report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
    markdown.push_str(&format!("Total commits: {}\n\n", report.total_commits));
    
    // Add survival section
    let survival_analyses = survival::analyze_survival(repo_path)?;
    markdown.push_str(&survival::generate_survival_report(&survival_analyses));
    markdown.push_str("\n");
    
    // Add co-modification section
    markdown.push_str(&comodification::generate_comodification_report(&report.comodification_clusters));
    markdown.push_str("\n");
    
    // Add evolution section
    markdown.push_str(&evolution::generate_evolution_report(&report.pattern_evolution));
    
    // Write output
    if let Some(path) = output {
        std::fs::write(&path, markdown)?;
        println!("✅ Report written to {}", path.display());
    } else {
        println!("{}", markdown);
    }
    
    Ok(())
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}

