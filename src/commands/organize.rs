use crate::config::Config;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Args;
use patina::indexer::PatternIndexer;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct OrganizeArgs {
    /// Analyze patterns without making changes
    #[arg(long, default_value = "false")]
    dry_run: bool,

    /// Show detailed analysis
    #[arg(long, short = 'v', default_value = "false")]
    verbose: bool,

    /// Focus on specific layer (core, surface, or all)
    #[arg(long, default_value = "all")]
    layer: String,

    /// Clean stale database entries older than N days
    #[arg(long)]
    clean_db_older_than: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PatternMetrics {
    path: PathBuf,
    layer: String,
    usage_count: u32,
    last_accessed: Option<DateTime<Utc>>,
    references_from: Vec<String>,
    references_to: Vec<String>,
    quality_score: f32,
    issues: Vec<String>,
    recommendation: PatternAction,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
enum PatternAction {
    PromoteToCore,
    KeepInSurface,
    DemoteToDust,
    Archive,
    Consolidate(String), // Merge with another pattern
    UpdateMetadata,
    NoAction,
}

pub fn execute(config: &Config, args: OrganizeArgs) -> Result<()> {
    println!("🧹 Organizing Patina patterns...\n");

    // Initialize indexer for pattern analysis
    let indexer = PatternIndexer::new()?;

    // Analyze patterns
    let mut analyzer = PatternAnalyzer::new(&indexer, &args)?;
    let metrics = analyzer.analyze_patterns()?;

    // Display analysis
    display_analysis(&metrics, args.verbose);

    if !args.dry_run {
        println!("\n📋 Applying recommendations...");
        apply_recommendations(&metrics, config)?;

        if let Some(days) = args.clean_db_older_than {
            clean_database(config, days)?;
        }
    } else {
        println!("\n🔍 Dry run mode - no changes made");
        println!("Run without --dry-run to apply recommendations");
    }

    Ok(())
}

struct PatternAnalyzer<'a> {
    indexer: &'a PatternIndexer,
    args: &'a OrganizeArgs,
    usage_data: HashMap<String, UsageInfo>,
}

#[derive(Debug, Default)]
struct UsageInfo {
    access_count: u32,
    last_accessed: Option<DateTime<Utc>>,
    references: Vec<String>,
}

impl<'a> PatternAnalyzer<'a> {
    fn new(indexer: &'a PatternIndexer, args: &'a OrganizeArgs) -> Result<Self> {
        let usage_data = Self::load_usage_data(indexer)?;
        Ok(Self {
            indexer,
            args,
            usage_data,
        })
    }

    fn load_usage_data(_indexer: &PatternIndexer) -> Result<HashMap<String, UsageInfo>> {
        let mut usage_map = HashMap::new();

        // Load from pattern_usage table if it exists
        let db_path = std::path::Path::new(".patina/cache/patina.db");
        if db_path.exists() {
            if let Ok(conn) = Connection::open(db_path) {
                let mut stmt = conn
                    .prepare(
                        "SELECT pattern_id, COUNT(*) as count, MAX(accessed_at) as last_access 
                 FROM pattern_usage 
                 GROUP BY pattern_id",
                    )
                    .ok();

                if let Some(ref mut stmt) = stmt {
                    let _ = stmt
                        .query_map([], |row| {
                            let pattern_id: String = row.get(0)?;
                            let count: u32 = row.get(1)?;
                            let last_access: Option<String> = row.get(2)?;

                            Ok((pattern_id, count, last_access))
                        })
                        .map(|mapped| {
                            for result in mapped {
                                if let Ok((id, count, last)) = result {
                                    usage_map.insert(
                                        id,
                                        UsageInfo {
                                            access_count: count,
                                            last_accessed: last
                                                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                                                .map(|dt| dt.with_timezone(&Utc)),
                                            references: vec![],
                                        },
                                    );
                                }
                            }
                        });
                }
            }
        }

        Ok(usage_map)
    }

    fn analyze_patterns(&mut self) -> Result<Vec<PatternMetrics>> {
        let mut all_metrics = Vec::new();

        // Analyze each layer based on args
        let layers = match self.args.layer.as_str() {
            "core" => vec!["core"],
            "surface" => vec!["surface"],
            _ => vec!["core", "surface"],
        };

        for layer in layers {
            let layer_path = Path::new("layer").join(layer);
            if layer_path.exists() {
                self.analyze_layer(&layer_path, layer, &mut all_metrics)?;
            }
        }

        // Cross-reference analysis
        self.analyze_references(&mut all_metrics)?;

        // Generate recommendations
        for metric in &mut all_metrics {
            metric.recommendation = self.recommend_action(metric);
        }

        Ok(all_metrics)
    }

    fn analyze_layer(
        &self,
        path: &Path,
        layer: &str,
        metrics: &mut Vec<PatternMetrics>,
    ) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let pattern_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let usage = self.usage_data.get(&pattern_id);
                let mut issues = Vec::new();

                // Check for quality issues
                let content = fs::read_to_string(&path)?;
                let quality_score = self.calculate_quality_score(&content, &mut issues);

                metrics.push(PatternMetrics {
                    path: path.clone(),
                    layer: layer.to_string(),
                    usage_count: usage.map(|u| u.access_count).unwrap_or(0),
                    last_accessed: usage.and_then(|u| u.last_accessed),
                    references_from: vec![],
                    references_to: self.extract_references(&content),
                    quality_score,
                    issues,
                    recommendation: PatternAction::NoAction,
                });
            }
        }

        Ok(())
    }

    fn calculate_quality_score(&self, content: &str, issues: &mut Vec<String>) -> f32 {
        let mut score = 100.0;

        // Check for metadata header
        if !content.starts_with("---") {
            issues.push("Missing metadata header".to_string());
            score -= 20.0;
        }

        // Check for proper sections
        let has_sections = content.contains("## ") || content.contains("# ");
        if !has_sections {
            issues.push("No clear section structure".to_string());
            score -= 15.0;
        }

        // Check for examples
        if !content.contains("```") && !content.contains("Example:") {
            issues.push("No code examples provided".to_string());
            score -= 10.0;
        }

        // Check content length
        let word_count = content.split_whitespace().count();
        if word_count < 100 {
            issues.push("Pattern description too brief".to_string());
            score -= 15.0;
        } else if word_count > 3000 {
            issues.push("Pattern might be too complex - consider splitting".to_string());
            score -= 5.0;
        }

        // Check for TODO/FIXME markers
        if content.contains("TODO") || content.contains("FIXME") {
            issues.push("Contains unfinished work markers".to_string());
            score -= 10.0;
        }

        f32::max(score, 0.0)
    }

    fn extract_references(&self, content: &str) -> Vec<String> {
        let mut refs = Vec::new();

        // Look for markdown links to other patterns
        for line in content.lines() {
            if let Some(start) = line.find("](") {
                if let Some(end) = line[start + 2..].find(')') {
                    let link = &line[start + 2..start + 2 + end];
                    if link.contains("layer/") || link.ends_with(".md") {
                        refs.push(link.to_string());
                    }
                }
            }
        }

        // Look for explicit references in metadata
        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let metadata = &content[3..end + 3];
                if let Some(refs_line) = metadata.lines().find(|l| l.starts_with("references:")) {
                    // Parse YAML array of references
                    if let Some(array_start) = refs_line.find('[') {
                        if let Some(array_end) = refs_line.find(']') {
                            let refs_str = &refs_line[array_start + 1..array_end];
                            for r in refs_str.split(',') {
                                refs.push(
                                    r.trim().trim_matches('"').trim_matches('\'').to_string(),
                                );
                            }
                        }
                    }
                }
            }
        }

        refs
    }

    fn analyze_references(&self, metrics: &mut Vec<PatternMetrics>) -> Result<()> {
        // Build reference map
        let mut reference_map: HashMap<String, Vec<String>> = HashMap::new();

        for metric in metrics.iter() {
            let from = metric
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            for to_ref in &metric.references_to {
                reference_map
                    .entry(to_ref.clone())
                    .or_default()
                    .push(from.clone());
            }
        }

        // Update metrics with incoming references
        for metric in metrics.iter_mut() {
            let pattern_name = metric
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            if let Some(refs) = reference_map.get(&pattern_name) {
                metric.references_from = refs.clone();
            }
        }

        Ok(())
    }

    fn recommend_action(&self, metric: &PatternMetrics) -> PatternAction {
        // Core patterns need high quality and stability
        if metric.layer == "core" {
            // Only demote if quality is really bad
            if metric.quality_score < 60.0 {
                return PatternAction::DemoteToDust;
            }
            // Don't archive core patterns unless they're broken
            if metric.quality_score < 40.0 {
                return PatternAction::Archive;
            }
        }

        // Surface patterns can be promoted or demoted
        if metric.layer == "surface" {
            // High quality, referenced patterns can be promoted
            // (Don't require usage data yet since we just started tracking)
            if metric.quality_score >= 90.0 && !metric.references_from.is_empty() {
                return PatternAction::PromoteToCore;
            }

            // Only demote really low quality patterns
            if metric.quality_score < 40.0 {
                return PatternAction::DemoteToDust;
            }

            // Patterns with only metadata issues just need updating
            if metric.quality_score >= 70.0 && metric.issues.iter().any(|i| i.contains("metadata"))
            {
                return PatternAction::UpdateMetadata;
            }
        }

        PatternAction::NoAction
    }
}

fn display_analysis(metrics: &[PatternMetrics], verbose: bool) {
    // Group by recommendation
    let mut by_action: HashMap<String, Vec<&PatternMetrics>> = HashMap::new();
    for metric in metrics {
        let action_key = format!("{:?}", metric.recommendation);
        by_action.entry(action_key).or_default().push(metric);
    }

    // Summary
    println!("📊 Pattern Analysis Summary");
    println!("─────────────────────────────");
    println!("Total patterns analyzed: {}", metrics.len());

    let avg_quality = metrics.iter().map(|m| m.quality_score).sum::<f32>() / metrics.len() as f32;
    println!("Average quality score: {avg_quality:.1}%");

    // Recommendations summary
    println!("\n🎯 Recommended Actions:");
    for (action_str, patterns) in &by_action {
        if patterns.is_empty() {
            continue;
        }

        let action_display = match action_str.as_str() {
            "PromoteToCore" => "→ Promote to Core",
            "DemoteToDust" => "↓ Demote to Dust",
            "Archive" => "🗄 Archive",
            "UpdateMetadata" => "✏️ Update Metadata",
            s if s.starts_with("Consolidate") => "🔀 Consolidate",
            "KeepInSurface" => "✓ Keep in Surface",
            "NoAction" => "◯ No Action",
            _ => "Unknown",
        };

        println!("  {} ({})", action_display, patterns.len());

        if verbose {
            for pattern in patterns {
                let name = pattern
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                println!(
                    "    - {} (score: {:.0}%, usage: {})",
                    name, pattern.quality_score, pattern.usage_count
                );

                if !pattern.issues.is_empty() {
                    for issue in &pattern.issues {
                        println!("      ⚠️  {issue}");
                    }
                }
            }
        }
    }

    // Problem patterns
    let problem_patterns: Vec<_> = metrics.iter().filter(|m| m.quality_score < 50.0).collect();

    if !problem_patterns.is_empty() {
        println!("\n⚠️  Patterns Needing Attention:");
        for pattern in problem_patterns.iter().take(5) {
            let name = pattern
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            println!(
                "  - {} ({}: {:.0}% quality)",
                name, pattern.layer, pattern.quality_score
            );
        }
    }
}

fn apply_recommendations(metrics: &[PatternMetrics], _config: &Config) -> Result<()> {
    let mut moved_count = 0;
    let mut updated_count = 0;

    for metric in metrics {
        match &metric.recommendation {
            PatternAction::PromoteToCore => {
                promote_pattern(&metric.path, "core")?;
                moved_count += 1;
            }
            PatternAction::DemoteToDust => {
                promote_pattern(&metric.path, "dust")?;
                moved_count += 1;
            }
            PatternAction::Archive => {
                archive_pattern(&metric.path)?;
                moved_count += 1;
            }
            PatternAction::UpdateMetadata => {
                update_pattern_metadata(&metric.path)?;
                updated_count += 1;
            }
            _ => {}
        }
    }

    if moved_count > 0 {
        println!("✓ Moved {moved_count} patterns");
    }
    if updated_count > 0 {
        println!("✓ Updated {updated_count} pattern metadata");
    }

    Ok(())
}

fn promote_pattern(from: &Path, to_layer: &str) -> Result<()> {
    let file_name = from
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    let to_path = Path::new("layer").join(to_layer).join(file_name);

    // Ensure destination directory exists
    if let Some(parent) = to_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Move the file
    fs::rename(from, &to_path)
        .with_context(|| format!("Failed to move {from:?} to {to_path:?}"))?;

    println!("  Moved {} to {}", from.display(), to_layer);
    Ok(())
}

fn archive_pattern(path: &Path) -> Result<()> {
    let archive_dir = Path::new("layer/dust/archived");
    fs::create_dir_all(archive_dir)?;

    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    let archive_path = archive_dir.join(file_name);

    fs::rename(path, &archive_path)?;
    println!("  Archived {}", path.display());
    Ok(())
}

fn update_pattern_metadata(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;

    // Add metadata if missing
    if !content.starts_with("---") {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let updated = format!(
            "---\nid: {}\nstatus: active\ncreated: {}\ntags: []\n---\n\n{}",
            name,
            Utc::now().format("%Y-%m-%d"),
            content
        );

        fs::write(path, updated)?;
        println!("  Added metadata to {}", path.display());
    }

    Ok(())
}

fn clean_database(config: &Config, days_old: u32) -> Result<()> {
    println!(
        "\n🗑️  Cleaning database entries older than {days_old} days..."
    );

    let db_path = config.cache_dir.join("patina.db");
    if !db_path.exists() {
        println!("  No database found");
        return Ok(());
    }

    let conn = Connection::open(&db_path)?;

    // Clean old state transitions
    let cutoff = Utc::now() - chrono::Duration::days(days_old as i64);
    let deleted = conn.execute(
        "DELETE FROM state_transitions WHERE timestamp < ?1",
        [cutoff.to_rfc3339()],
    )?;

    if deleted > 0 {
        println!("  Removed {deleted} old state transitions");
    }

    // Clean orphaned documents
    let orphaned = conn.execute(
        "DELETE FROM documents WHERE id NOT IN (
            SELECT DISTINCT document_id FROM concepts
        ) AND id NOT IN (
            SELECT DISTINCT document_id FROM git_states WHERE document_id IS NOT NULL
        )",
        [],
    )?;

    if orphaned > 0 {
        println!("  Removed {orphaned} orphaned documents");
    }

    // Vacuum to reclaim space
    conn.execute("VACUUM", [])?;
    println!("  Database optimized");

    Ok(())
}
