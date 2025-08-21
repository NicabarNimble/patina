use anyhow::Result;
use chrono::Utc;
use clap::Args;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Args, Debug)]
pub struct SessionAnalyzeArgs {
    /// Session ID (defaults to current)
    #[arg(long)]
    session: Option<String>,

    /// Show code-pattern connections
    #[arg(long, default_value = "false")]
    patterns: bool,

    /// Generate context for agents
    #[arg(long, default_value = "false")]
    agent_context: bool,

    /// Verbose output
    #[arg(long, short = 'v', default_value = "false")]
    verbose: bool,
}

#[derive(Debug, Serialize)]
struct SessionAnalysis {
    session_id: String,
    goal: String,
    duration_minutes: i64,

    // What happened
    files_created: Vec<String>,
    files_modified: Vec<String>,
    lines_added: u32,
    lines_removed: u32,

    // Pattern analysis
    patterns_implemented: Vec<String>,
    patterns_discovered: Vec<String>,

    // Quality signals
    survival_rate: f32,
    refactor_count: u32,

    // Key insights
    insights: Vec<String>,
}

pub fn execute(args: SessionAnalyzeArgs) -> Result<()> {
    let session_id = get_session_id(&args.session)?;

    println!("🔍 Analyzing session: {session_id}\n");

    let analysis = analyze_session(&session_id, &args)?;

    if args.agent_context {
        print_agent_context(&analysis);
    } else {
        print_human_summary(&analysis, args.verbose);
    }

    Ok(())
}

fn get_session_id(override_id: &Option<String>) -> Result<String> {
    if let Some(id) = override_id {
        return Ok(id.clone());
    }

    // Get current session from active-session.md
    if let Ok(content) = std::fs::read_to_string(".claude/context/active-session.md") {
        for line in content.lines() {
            if line.contains("**ID**:") {
                return Ok(line.split(':').nth(1).unwrap_or("").trim().to_string());
            }
        }
    }

    // Fall back to most recent session tag
    let output = Command::new("git")
        .args(["tag", "-l", "session-*-start", "--sort=-creatordate"])
        .output()?;

    let tags = String::from_utf8_lossy(&output.stdout);
    if let Some(tag) = tags.lines().next() {
        // Extract ID from session-YYYYMMDD-HHMMSS-start
        let id = tag.replace("session-", "").replace("-start", "");
        return Ok(id);
    }

    anyhow::bail!("No session found")
}

fn analyze_session(session_id: &str, args: &SessionAnalyzeArgs) -> Result<SessionAnalysis> {
    let start_tag = format!("session-{session_id}-start");
    let end_tag = format!("session-{session_id}-end");

    // Get session goal from session file
    let goal = get_session_goal(session_id)?;

    // Get time duration
    let duration = get_session_duration(&start_tag, &end_tag)?;

    // Get file changes
    let (files_created, files_modified) = get_file_changes(&start_tag)?;

    // Get diff stats
    let (lines_added, lines_removed) = get_diff_stats(&start_tag)?;

    // Analyze patterns
    let (patterns_implemented, patterns_discovered) = if args.patterns {
        analyze_patterns(&files_created, &files_modified)?
    } else {
        (vec![], vec![])
    };

    // Calculate quality metrics
    let survival_rate = calculate_survival_rate(&start_tag)?;
    let refactor_count = count_refactors(&start_tag)?;

    // Extract insights
    let insights = extract_insights(
        &files_created,
        &files_modified,
        lines_added,
        refactor_count,
        survival_rate,
    );

    Ok(SessionAnalysis {
        session_id: session_id.to_string(),
        goal,
        duration_minutes: duration,
        files_created,
        files_modified,
        lines_added,
        lines_removed,
        patterns_implemented,
        patterns_discovered,
        survival_rate,
        refactor_count,
        insights,
    })
}

fn get_session_goal(session_id: &str) -> Result<String> {
    // Try session file
    let session_file = format!("layer/sessions/{session_id}.md");
    if Path::new(&session_file).exists() {
        let content = std::fs::read_to_string(&session_file)?;
        for line in content.lines() {
            if line.starts_with("# Session:") {
                return Ok(line.replace("# Session:", "").trim().to_string());
            }
        }
    }

    // Try active session
    if let Ok(content) = std::fs::read_to_string(".claude/context/active-session.md") {
        for line in content.lines() {
            if line.starts_with("# Session:") {
                return Ok(line.replace("# Session:", "").trim().to_string());
            }
        }
    }

    Ok("Unknown goal".to_string())
}

fn get_session_duration(start_tag: &str, end_tag: &str) -> Result<i64> {
    // Get timestamp of start tag
    let start_time = Command::new("git")
        .args(["log", "-1", "--format=%at", start_tag])
        .output()?;

    let start_ts = String::from_utf8_lossy(&start_time.stdout)
        .trim()
        .parse::<i64>()
        .unwrap_or(0);

    // Get timestamp of end tag or current time
    let end_ts = if git_tag_exists(end_tag) {
        let end_time = Command::new("git")
            .args(["log", "-1", "--format=%at", end_tag])
            .output()?;

        String::from_utf8_lossy(&end_time.stdout)
            .trim()
            .parse::<i64>()
            .unwrap_or(0)
    } else {
        Utc::now().timestamp()
    };

    Ok((end_ts - start_ts) / 60) // Return minutes
}

fn git_tag_exists(tag: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", tag])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn get_file_changes(start_tag: &str) -> Result<(Vec<String>, Vec<String>)> {
    let output = Command::new("git")
        .args(["diff", "--name-status", &format!("{start_tag}..HEAD")])
        .output()?;

    let mut created = Vec::new();
    let mut modified = Vec::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            let status = parts[0];
            let file = parts[1].to_string();

            // Skip non-code files
            if !file.ends_with(".rs") && !file.ends_with(".md") && !file.ends_with(".toml") {
                continue;
            }

            match status {
                "A" => created.push(file),
                "M" => modified.push(file),
                _ => {}
            }
        }
    }

    Ok((created, modified))
}

fn get_diff_stats(start_tag: &str) -> Result<(u32, u32)> {
    let output = Command::new("git")
        .args(["diff", "--shortstat", &format!("{start_tag}..HEAD")])
        .output()?;

    let stats = String::from_utf8_lossy(&output.stdout);

    // Parse: "X files changed, Y insertions(+), Z deletions(-)"
    let mut added = 0u32;
    let mut removed = 0u32;

    if let Some(pos) = stats.find("insertion") {
        let before = &stats[..pos];
        if let Some(num_str) = before.split_whitespace().last() {
            added = num_str.parse().unwrap_or(0);
        }
    }

    if let Some(pos) = stats.find("deletion") {
        let before = &stats[..pos];
        if let Some(num_str) = before.split_whitespace().last() {
            removed = num_str.parse().unwrap_or(0);
        }
    }

    Ok((added, removed))
}

fn analyze_patterns(created: &[String], modified: &[String]) -> Result<(Vec<String>, Vec<String>)> {
    let mut implemented = Vec::new();
    let mut discovered = Vec::new();

    // Check for DEPENDABLE_RUST pattern
    for file in created.iter().chain(modified.iter()) {
        if (file.contains("/internal/") || file.ends_with("/internal.rs"))
            && !implemented.contains(&"dependable-rust".to_string())
        {
            implemented.push("dependable-rust".to_string());
        }

        // Check for organize pattern discovery
        if file.contains("organize") {
            discovered.push("git-based-value-assessment".to_string());
        }
    }

    // Check commit messages for pattern mentions
    let output = Command::new("git")
        .args(["log", "--oneline", "--grep=pattern", "-10"])
        .output()?;

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.to_lowercase().contains("pattern") {
            // Extract pattern name if possible
            if line.contains("implement") {
                discovered.push("pattern-from-commit".to_string());
            }
        }
    }

    Ok((implemented, discovered))
}

fn calculate_survival_rate(start_tag: &str) -> Result<f32> {
    // Get all commits since start
    let output = Command::new("git")
        .args(["rev-list", &format!("{start_tag}..HEAD")])
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<&str> = output_str.lines().collect();

    if commits.is_empty() {
        return Ok(100.0);
    }

    // Count how many commits modified files that still exist
    let mut survived = 0;
    let total = commits.len();

    for commit in &commits {
        // Check if changes from this commit still exist
        let diff = Command::new("git")
            .args(["diff", "--name-only", &format!("{commit}^..{commit}")])
            .output()?;

        let files_changed = String::from_utf8_lossy(&diff.stdout).lines().count();

        if files_changed > 0 {
            survived += 1;
        }
    }

    Ok((survived as f32 / total as f32) * 100.0)
}

fn count_refactors(start_tag: &str) -> Result<u32> {
    // Count commits with refactor-related messages
    let output = Command::new("git")
        .args([
            "log",
            "--oneline",
            &format!("{start_tag}..HEAD"),
            "--grep=refactor",
            "-i",
        ])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).lines().count() as u32)
}

fn extract_insights(
    created: &[String],
    _modified: &[String],
    lines_added: u32,
    refactor_count: u32,
    survival_rate: f32,
) -> Vec<String> {
    let mut insights = Vec::new();

    // Insight: Exploration pattern
    if created.iter().any(|f| f.contains("_v2")) {
        insights.push("Explored multiple implementations (v1 → v2)".to_string());
    }

    // Insight: High refactoring
    if refactor_count > 2 {
        insights.push(format!(
            "High iteration ({refactor_count} refactors) - exploring solution space"
        ));
    }

    // Insight: New subsystem
    if created.len() > 5 {
        insights.push(format!(
            "Created new subsystem ({} new files)",
            created.len()
        ));
    }

    // Insight: Quality code
    if survival_rate > 90.0 && lines_added > 100 {
        insights.push("High-quality implementation (90%+ survival)".to_string());
    }

    // Insight: Pattern extraction opportunity
    if lines_added > 500 {
        insights.push("Large implementation - consider extracting patterns".to_string());
    }

    insights
}

fn print_human_summary(analysis: &SessionAnalysis, verbose: bool) {
    println!("📊 Session Analysis: {}", analysis.goal);
    println!("═══════════════════════════════════════");

    println!("\n⏱️  Duration: {} minutes", analysis.duration_minutes);

    println!("\n📝 Changes:");
    println!("  Files created: {}", analysis.files_created.len());
    println!("  Files modified: {}", analysis.files_modified.len());
    println!(
        "  Lines: +{} -{}",
        analysis.lines_added, analysis.lines_removed
    );

    if verbose {
        if !analysis.files_created.is_empty() {
            println!("\n  Created:");
            let max_files = 5.min(analysis.files_created.len());
            for file in &analysis.files_created[..max_files] {
                println!("    - {file}");
            }
        }

        if !analysis.files_modified.is_empty() {
            println!("\n  Modified:");
            let max_files = 5.min(analysis.files_modified.len());
            for file in &analysis.files_modified[..max_files] {
                println!("    - {file}");
            }
        }
    }

    println!("\n📈 Quality Metrics:");
    println!("  Survival rate: {:.0}%", analysis.survival_rate);
    println!("  Refactors: {}", analysis.refactor_count);

    if !analysis.patterns_implemented.is_empty() {
        println!("\n🎯 Patterns Implemented:");
        for pattern in &analysis.patterns_implemented {
            println!("  - {pattern}");
        }
    }

    if !analysis.patterns_discovered.is_empty() {
        println!("\n💡 Patterns Discovered:");
        for pattern in &analysis.patterns_discovered {
            println!("  - {pattern}");
        }
    }

    if !analysis.insights.is_empty() {
        println!("\n🔍 Key Insights:");
        for insight in &analysis.insights {
            println!("  • {insight}");
        }
    }
}

fn print_agent_context(analysis: &SessionAnalysis) {
    // Minimal context for agents
    println!("CONTEXT FOR SESSION {}", analysis.session_id);
    println!("Goal: {}", analysis.goal);
    println!("Duration: {}m", analysis.duration_minutes);
    println!("Quality: {:.0}% survival", analysis.survival_rate);

    if !analysis.insights.is_empty() {
        println!("\nKey points:");
        for insight in &analysis.insights {
            println!("- {insight}");
        }
    }

    println!("\nFocus areas:");
    for file in analysis.files_created.iter().take(3) {
        println!("- NEW: {file}");
    }

    if !analysis.patterns_discovered.is_empty() {
        println!("\nPattern: {}", analysis.patterns_discovered[0]);
    }
}
