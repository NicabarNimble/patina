//! Belief audit command — show computed use/truth metrics for all beliefs
//!
//! Reads from the `beliefs` table (computed by `patina scrape`) and displays
//! real metrics instead of fabricated confidence scores.
//!
//! E4.6a: --grounding flag computes semantic grounding from usearch embeddings.

use anyhow::{Context, Result};
use clap::Subcommand;
use rusqlite::Connection;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};
use walkdir::WalkDir;

use super::scrape::database;
use super::scry::internal::enrichment::{enrich_results, SearchResults};

#[derive(Subcommand, Debug)]
pub enum BeliefCommands {
    /// Show all beliefs ranked by use/truth metrics (default)
    Audit {
        /// Sort by: "use" (default), "truth", "weak", "health", "grounding"
        #[arg(long, default_value = "use")]
        sort: String,

        /// Show only beliefs with warnings
        #[arg(long)]
        warnings_only: bool,

        /// Show semantic grounding — nearest code/commits/sessions for each belief (E4.6a)
        #[arg(long)]
        grounding: bool,

        /// Show only stale beliefs (last_activity > stale_days threshold)
        #[arg(long)]
        stale: bool,
    },

    /// Import a belief from another project
    ///
    /// Copies a belief from a source project into the current project's
    /// layer/surface/epistemic/beliefs/ directory, resetting entrenchment
    /// to 'low' and adding provenance metadata.
    Import {
        /// Belief ID to import
        belief_id: String,

        /// Source project name (as registered in mother's graph)
        #[arg(long)]
        from: String,

        /// Overwrite if belief already exists locally
        #[arg(long)]
        force: bool,
    },

    /// Rename a belief ID and its file path
    Rename {
        /// Existing belief ID (kebab-case)
        belief_id: String,

        /// New belief ID (kebab-case)
        new_id: String,

        /// Rewrite belief wikilinks in layer/surface/epistemic/beliefs/
        #[arg(long)]
        rewrite_links: bool,

        /// Show planned changes without writing files
        #[arg(long)]
        dry_run: bool,

        /// Skip git commit (still writes files)
        #[arg(long)]
        no_commit: bool,
    },
}

pub fn execute(command: Option<BeliefCommands>) -> Result<()> {
    let cmd = command.unwrap_or(BeliefCommands::Audit {
        sort: "use".to_string(),
        warnings_only: false,
        grounding: false,
        stale: false,
    });

    match cmd {
        BeliefCommands::Audit {
            sort,
            warnings_only,
            grounding,
            stale,
        } => run_audit(&sort, warnings_only, grounding, stale),
        BeliefCommands::Import {
            belief_id,
            from,
            force,
        } => run_import(&belief_id, &from, force),
        BeliefCommands::Rename {
            belief_id,
            new_id,
            rewrite_links,
            dry_run,
            no_commit,
        } => run_rename(&belief_id, &new_id, rewrite_links, dry_run, no_commit),
    }
}

fn run_rename(
    belief_id: &str,
    new_id: &str,
    rewrite_links: bool,
    dry_run: bool,
    no_commit: bool,
) -> Result<()> {
    validate_belief_id(belief_id)?;
    validate_belief_id(new_id)?;

    if belief_id == new_id {
        anyhow::bail!("new belief id must differ from current id");
    }

    let beliefs_dir = Path::new("layer/surface/epistemic/beliefs");
    let old_path = beliefs_dir.join(format!("{}.md", belief_id));
    let new_path = beliefs_dir.join(format!("{}.md", new_id));

    if !old_path.exists() {
        anyhow::bail!("belief '{}' not found at {}", belief_id, old_path.display());
    }
    if new_path.exists() {
        anyhow::bail!(
            "target belief '{}' already exists at {}",
            new_id,
            new_path.display()
        );
    }

    let original = std::fs::read_to_string(&old_path)
        .with_context(|| format!("reading {}", old_path.display()))?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let renamed = rewrite_belief_frontmatter_for_rename(&original, belief_id, new_id, &today)?;

    let mut rewritten_links: Vec<String> = Vec::new();
    if rewrite_links {
        rewritten_links = collect_link_rewrite_targets(beliefs_dir, belief_id, new_id)?;
    }

    if dry_run {
        println!("Dry run: belief rename");
        println!("  {} -> {}", old_path.display(), new_path.display());
        println!("  frontmatter: id '{}' -> '{}'", belief_id, new_id);
        if rewrite_links {
            println!("  rewrite-links: {} file(s)", rewritten_links.len());
            for p in &rewritten_links {
                println!("    {}", p);
            }
        }
        if no_commit {
            println!("  no-commit: enabled");
        }
        return Ok(());
    }

    std::fs::write(&old_path, renamed)
        .with_context(|| format!("writing {}", old_path.display()))?;
    std::fs::rename(&old_path, &new_path)
        .with_context(|| format!("renaming {} -> {}", old_path.display(), new_path.display()))?;

    if rewrite_links {
        apply_link_rewrites(&rewritten_links, belief_id, new_id)?;
    }

    if !no_commit {
        stage_and_commit_belief_rename(&old_path, &new_path, &rewritten_links, belief_id, new_id)?;
    }

    println!("Renamed belief '{}' -> '{}'", belief_id, new_id);
    println!("  file: {}", new_path.display());
    if rewrite_links {
        println!("  rewritten links: {}", rewritten_links.len());
    }
    println!("Run `patina scrape` to refresh belief grounding/metrics.");

    Ok(())
}

fn validate_belief_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        anyhow::bail!("invalid belief id '{}': use kebab-case", id);
    }
    Ok(())
}

fn rewrite_belief_frontmatter_for_rename(
    content: &str,
    old_id: &str,
    new_id: &str,
    today: &str,
) -> Result<String> {
    if !content.starts_with("---") {
        anyhow::bail!("belief is missing YAML frontmatter");
    }

    let mut sections = content.splitn(3, "---");
    let prefix = sections.next().unwrap_or_default();
    let frontmatter = sections
        .next()
        .ok_or_else(|| anyhow::anyhow!("belief frontmatter missing opening delimiter"))?;
    let body = sections
        .next()
        .ok_or_else(|| anyhow::anyhow!("belief frontmatter missing closing delimiter"))?;

    if !prefix.trim().is_empty() {
        anyhow::bail!("belief frontmatter must start at file beginning");
    }

    let mut saw_id = false;
    let mut saw_revised = false;
    let mut out_frontmatter: Vec<String> = Vec::new();

    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(existing_id) = trimmed.strip_prefix("id:") {
            let existing_id = existing_id.trim();
            if existing_id != old_id {
                anyhow::bail!(
                    "belief frontmatter id mismatch: expected '{}', found '{}'",
                    old_id,
                    existing_id
                );
            }
            out_frontmatter.push(format!("id: {}", new_id));
            saw_id = true;
            continue;
        }

        if trimmed.starts_with("revised:") {
            out_frontmatter.push(format!("revised: {}", today));
            saw_revised = true;
            continue;
        }

        out_frontmatter.push(line.to_string());
    }

    if !saw_id {
        anyhow::bail!("belief frontmatter is missing required id field");
    }
    if !saw_revised {
        out_frontmatter.push(format!("revised: {}", today));
    }

    let mut output = String::new();
    output.push_str("---\n");
    for line in out_frontmatter {
        output.push_str(&line);
        output.push('\n');
    }
    output.push_str("---");
    output.push_str(body);

    let old_h1 = format!("\n# {}\n", old_id);
    let new_h1 = format!("\n# {}\n", new_id);
    if output.contains(&old_h1) {
        output = output.replacen(&old_h1, &new_h1, 1);
    }

    Ok(output)
}

fn collect_link_rewrite_targets(
    beliefs_dir: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<Vec<String>> {
    let old_link = format!("[[{}]]", old_id);
    let new_link = format!("[[{}]]", new_id);
    let mut targets: BTreeSet<String> = BTreeSet::new();

    for entry in WalkDir::new(beliefs_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let p = entry.path();
        if !p.is_file() || p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let content =
            std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
        if content.contains(&old_link) {
            targets.insert(p.to_string_lossy().to_string());
        }

        // Guard against accidental no-op rewrite where old==new in content checks.
        if old_link == new_link {
            break;
        }
    }

    Ok(targets.into_iter().collect())
}

fn apply_link_rewrites(paths: &[String], old_id: &str, new_id: &str) -> Result<()> {
    let old_link = format!("[[{}]]", old_id);
    let new_link = format!("[[{}]]", new_id);

    for p in paths {
        let content = std::fs::read_to_string(p).with_context(|| format!("reading {}", p))?;
        let updated = content.replace(&old_link, &new_link);
        if updated != content {
            std::fs::write(p, updated).with_context(|| format!("writing {}", p))?;
        }
    }

    Ok(())
}

fn stage_and_commit_belief_rename(
    old_path: &Path,
    new_path: &Path,
    rewritten_links: &[String],
    old_id: &str,
    new_id: &str,
) -> Result<()> {
    let mut args = vec!["add".to_string(), "-A".to_string()];
    args.push(old_path.to_string_lossy().to_string());
    args.push(new_path.to_string_lossy().to_string());
    for p in rewritten_links {
        args.push(p.clone());
    }

    let output = Command::new("git")
        .args(args)
        .output()
        .context("failed to stage belief rename")?;
    if !output.status.success() {
        anyhow::bail!(
            "failed to stage belief rename: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if patina::git::has_staged_changes()? {
        patina::git::commit(&format!("belief: rename {} to {}", old_id, new_id))?;
    }

    Ok(())
}

/// Import a belief from another project
fn run_import(belief_id: &str, from: &str, force: bool) -> Result<()> {
    use patina::mother::Graph;

    let beliefs_dir = Path::new("layer/surface/epistemic/beliefs");
    let local_path = beliefs_dir.join(format!("{}.md", belief_id));

    // Guard: refuse if belief already exists locally (unless --force)
    if local_path.exists() && !force {
        anyhow::bail!(
            "Belief '{}' already exists locally at {}.\nUse --force to overwrite.",
            belief_id,
            local_path.display()
        );
    }

    // Open graph and look up the belief
    let graph = Graph::open()?;

    let (entry, source_path) = graph.get_belief(belief_id, from)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Belief '{}' not found in project '{}' in graph.db.\n\
                 Check: patina mother graph query projects {}\n\
                 Or sync: patina mother graph sync",
            belief_id,
            from,
            belief_id,
        )
    })?;

    // Resolve source file on disk
    let source_file = source_path
        .join("layer/surface/epistemic/beliefs")
        .join(format!("{}.md", belief_id));

    if !source_file.exists() {
        anyhow::bail!(
            "Source belief file not found: {}\n\
             The project path in graph.db may be stale.\n\
             Try: patina repo register {} {}",
            source_file.display(),
            from,
            source_path.display(),
        );
    }

    // Read the source file
    let source_content = std::fs::read_to_string(&source_file)
        .with_context(|| format!("reading {}", source_file.display()))?;

    // Rewrite the frontmatter: reset entrenchment to low, add imported_from
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let new_content = rewrite_imported_belief(&source_content, &entry.entrenchment, from, &today);

    // Ensure target directory exists
    if !beliefs_dir.exists() {
        std::fs::create_dir_all(beliefs_dir)
            .with_context(|| format!("creating {}", beliefs_dir.display()))?;
    }

    // Write the imported belief
    std::fs::write(&local_path, &new_content)
        .with_context(|| format!("writing {}", local_path.display()))?;

    println!("✅ Imported belief '{}' from project '{}'", belief_id, from);
    println!(
        "   Entrenchment reset to 'low' (was '{}')",
        entry.entrenchment
    );
    println!("   Written to: {}", local_path.display());
    println!();
    println!("Run 'patina scrape' to index the imported belief.");

    Ok(())
}

/// Rewrite a belief's frontmatter for import:
/// - Reset entrenchment to 'low'
/// - Add imported_from and import_date
/// - Append ## Origin section
fn rewrite_imported_belief(
    content: &str,
    original_entrenchment: &str,
    from: &str,
    today: &str,
) -> String {
    let mut output = String::new();

    // Parse and rewrite frontmatter
    if let Some(after_start) = content.strip_prefix("---") {
        if let Some(end) = after_start.find("---") {
            let frontmatter = &after_start[..end];
            let body = &after_start[end + 3..];

            output.push_str("---\n");

            // Rewrite frontmatter lines
            let mut wrote_entrenchment = false;
            let mut wrote_imported_from = false;
            for line in frontmatter.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("entrenchment:") {
                    output.push_str("entrenchment: low\n");
                    wrote_entrenchment = true;
                } else if trimmed.starts_with("imported_from:") {
                    // Replace existing
                    output.push_str(&format!("imported_from: {}\n", from));
                    wrote_imported_from = true;
                } else if trimmed.starts_with("import_date:") {
                    // Skip — will be re-added with imported_from
                } else if !trimmed.is_empty() {
                    output.push_str(line);
                    output.push('\n');
                }
            }

            // Add fields if not already present
            if !wrote_entrenchment {
                output.push_str("entrenchment: low\n");
            }
            if !wrote_imported_from {
                output.push_str(&format!("imported_from: {}\n", from));
            }
            output.push_str(&format!("import_date: {}\n", today));

            output.push_str("---");
            output.push_str(body);
        } else {
            // Malformed frontmatter — pass through
            output.push_str(content);
        }
    } else {
        // No frontmatter — add one
        output.push_str(&format!(
            "---\nentrenchment: low\nimported_from: {}\nimport_date: {}\n---\n",
            from, today
        ));
        output.push_str(content);
    }

    // Append ## Origin section if not already present
    if !output.contains("## Origin") {
        output.push_str("\n## Origin\n\n");
        output.push_str(&format!("- Imported from: {}\n", from));
        output.push_str(&format!(
            "- Original entrenchment: {}\n",
            original_entrenchment
        ));
        output.push_str(&format!("- Import date: {}\n", today));

        // Append session backlink if active
        if let Some(session_id) = crate::commands::scry::internal::logging::get_active_session_id()
        {
            output.push_str(&format!("- Import session: [[session-{}]]\n", session_id));
        }
    }

    output
}

struct BeliefRow {
    id: String,
    entrenchment: String,
    cited_by_beliefs: i32,
    cited_by_sessions: i32,
    applied_in: i32,
    evidence_count: i32,
    evidence_verified: i32,
    defeated_attacks: i32,
    verification_total: i32,
    verification_passed: i32,
    verification_failed: i32,
    verification_errored: i32,
    // E4.6a: Semantic grounding
    grounding_score: f32,
    grounding_code_count: i32,
    grounding_commit_count: i32,
    grounding_session_count: i32,
    // Belief truthfulness
    health_score: f64,
    last_activity: Option<String>,
    verification_drifted: bool,
    contested_by: String,
}

impl BeliefRow {
    fn total_use(&self) -> i32 {
        self.cited_by_beliefs + self.cited_by_sessions
    }

    fn v_ok_display(&self) -> String {
        if self.verification_total == 0 {
            "\u{2014}".to_string() // em dash
        } else {
            format!("{}/{}", self.verification_passed, self.verification_total)
        }
    }

    fn grounding_total(&self) -> i32 {
        self.grounding_code_count + self.grounding_commit_count + self.grounding_session_count
    }

    fn grounding_display(&self) -> String {
        if self.grounding_total() == 0 {
            "\u{2014}".to_string() // em dash
        } else {
            format!(
                "{}c{}m{}s",
                self.grounding_code_count,
                self.grounding_commit_count,
                self.grounding_session_count
            )
        }
    }

    fn health_warnings(&self) -> Vec<String> {
        let mut warnings: Vec<String> = Vec::new();
        if self.evidence_count == 0 {
            warnings.push("no-evidence".to_string());
        }
        if self.evidence_verified == 0 && self.evidence_count > 0 {
            warnings.push("unverified".to_string());
        }
        if self.total_use() == 0 {
            warnings.push("unused".to_string());
        }
        if self.applied_in == 0 {
            warnings.push("no-applications".to_string());
        }
        if self.verification_failed > 0 {
            warnings.push("verify-contested".to_string());
        }
        if self.verification_errored > 0 {
            warnings.push("verify-error".to_string());
        }
        if self.verification_drifted {
            warnings.push("verify-drifted".to_string());
        }
        if self.grounding_total() == 0 && self.grounding_score == 0.0 {
            warnings.push("floating".to_string());
        }
        if self.health_score < 0.4 {
            warnings.push("low-health".to_string());
        }
        // Phase C: contested-by warnings
        if !self.contested_by.is_empty() {
            for id in self.contested_by.split(',').filter(|s| !s.is_empty()) {
                warnings.push(format!("contested-by:{}", id));
            }
        }
        warnings
    }
}

fn run_audit(sort_by: &str, warnings_only: bool, show_grounding: bool, stale: bool) -> Result<()> {
    let db_path = database::patina_db_path()?;
    if !db_path.exists() {
        anyhow::bail!("No database found. Run `patina scrape` first.");
    }

    // Load config for stale_days (each command loads independently per spec)
    let config = patina::project::load(Path::new(".")).unwrap_or_default();
    let stale_days = config.beliefs.stale_days;

    let conn = Connection::open(&db_path)?;

    // Check if metric columns exist
    let has_metrics = conn
        .prepare("SELECT cited_by_beliefs FROM beliefs LIMIT 1")
        .is_ok();

    if !has_metrics {
        anyhow::bail!(
            "Belief metrics not computed yet. Run `patina scrape --rebuild` to compute use/truth metrics."
        );
    }

    let order_clause = match sort_by {
        "truth" => "evidence_count DESC, evidence_verified DESC",
        "weak" => "(cited_by_beliefs + cited_by_sessions) ASC, evidence_count ASC",
        "health" => "health_score ASC", // ascending: worst health first
        "grounding" => "grounding_score DESC, (grounding_code_count + grounding_commit_count + grounding_session_count) DESC",
        _ => "(cited_by_beliefs + cited_by_sessions) DESC, evidence_count DESC", // "use" default
    };

    // Check if verification columns exist (migration may not have run yet)
    let has_verification = conn
        .prepare("SELECT verification_total FROM beliefs LIMIT 1")
        .is_ok();

    // Check if grounding columns exist
    let has_grounding = conn
        .prepare("SELECT grounding_score FROM beliefs LIMIT 1")
        .is_ok();

    // Check if truthfulness columns exist
    let has_truthfulness = conn
        .prepare("SELECT health_score FROM beliefs LIMIT 1")
        .is_ok();

    let sql = format!(
        "SELECT id, entrenchment, cited_by_beliefs, cited_by_sessions, applied_in,
                evidence_count, evidence_verified, defeated_attacks{}{}{}
         FROM beliefs
         ORDER BY {}",
        if has_verification {
            ", verification_total, verification_passed, verification_failed, verification_errored"
        } else {
            ""
        },
        if has_grounding {
            ", grounding_score, grounding_code_count, grounding_commit_count, grounding_session_count"
        } else {
            ""
        },
        if has_truthfulness {
            ", health_score, last_activity, verification_drifted, contested_by"
        } else {
            ""
        },
        order_clause
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<BeliefRow> = stmt
        .query_map([], |row| {
            let base_idx = 8; // 0-7 are always present
            let v_offset = base_idx;
            let g_offset = if has_verification {
                v_offset + 4
            } else {
                v_offset
            };
            let t_offset = if has_grounding {
                g_offset + 4
            } else {
                g_offset
            };

            Ok(BeliefRow {
                id: row.get(0)?,
                entrenchment: row.get(1)?,
                cited_by_beliefs: row.get(2)?,
                cited_by_sessions: row.get(3)?,
                applied_in: row.get(4)?,
                evidence_count: row.get(5)?,
                evidence_verified: row.get(6)?,
                defeated_attacks: row.get(7)?,
                verification_total: if has_verification {
                    row.get(v_offset)?
                } else {
                    0
                },
                verification_passed: if has_verification {
                    row.get(v_offset + 1)?
                } else {
                    0
                },
                verification_failed: if has_verification {
                    row.get(v_offset + 2)?
                } else {
                    0
                },
                verification_errored: if has_verification {
                    row.get(v_offset + 3)?
                } else {
                    0
                },
                grounding_score: if has_grounding {
                    row.get(g_offset)?
                } else {
                    0.0
                },
                grounding_code_count: if has_grounding {
                    row.get(g_offset + 1)?
                } else {
                    0
                },
                grounding_commit_count: if has_grounding {
                    row.get(g_offset + 2)?
                } else {
                    0
                },
                grounding_session_count: if has_grounding {
                    row.get(g_offset + 3)?
                } else {
                    0
                },
                health_score: if has_truthfulness {
                    row.get(t_offset)?
                } else {
                    0.0
                },
                last_activity: if has_truthfulness {
                    row.get(t_offset + 1)?
                } else {
                    None
                },
                verification_drifted: if has_truthfulness {
                    row.get::<_, i32>(t_offset + 2).unwrap_or(0) == 1
                } else {
                    false
                },
                contested_by: if has_truthfulness {
                    row.get::<_, String>(t_offset + 3).unwrap_or_default()
                } else {
                    String::new()
                },
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        println!("No beliefs found. Create beliefs in layer/surface/epistemic/beliefs/");
        return Ok(());
    }

    // Compute stale threshold date for --stale filter
    let stale_threshold = {
        let today = chrono::Utc::now().date_naive();
        today - chrono::Duration::days(stale_days as i64)
    };

    // Apply filters: --stale AND --warnings-only compose via AND
    let display_rows: Vec<&BeliefRow> = rows
        .iter()
        .filter(|r| {
            if stale {
                // Stale = last_activity exceeds stale_days OR last_activity is NULL
                match &r.last_activity {
                    Some(date_str) => {
                        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                            date <= stale_threshold
                        } else {
                            true // unparseable → treat as stale
                        }
                    }
                    None => true, // NULL → stale
                }
            } else {
                true
            }
        })
        .filter(|r| {
            if warnings_only {
                !r.health_warnings().is_empty()
            } else {
                true
            }
        })
        .collect();

    // Print header
    println!(
        "\n  Belief Audit — {} beliefs (sorted by {}{})\n",
        rows.len(),
        sort_by,
        if stale {
            format!(", stale >{}d", stale_days)
        } else {
            String::new()
        }
    );
    println!(
        "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>5} {:>9} {:>7} WARNINGS",
        "BELIEF",
        "B-USE",
        "S-USE",
        "EVID",
        "VERI",
        "DEFT",
        "APPL",
        "V-OK",
        "HLTH",
        "ENTRENCH",
        "GROUND"
    );
    println!(
        "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>5} {:>9} {:>7} ────────",
        "──────",
        "─────",
        "─────",
        "────",
        "────",
        "────",
        "────",
        "─────",
        "─────",
        "─────────",
        "───────"
    );

    let mut warning_count = 0;
    for row in &display_rows {
        let warnings = row.health_warnings();
        if !warnings.is_empty() {
            warning_count += 1;
        }
        let warning_str = if warnings.is_empty() {
            String::new()
        } else {
            warnings.join(", ")
        };

        // Truncate ID for display
        let display_id = if row.id.len() > 35 {
            format!("{}…", &row.id[..34])
        } else {
            row.id.clone()
        };

        let health_display = format!("{:.2}", row.health_score);

        println!(
            "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>5} {:>9} {:>7} {}",
            display_id,
            row.cited_by_beliefs,
            row.cited_by_sessions,
            row.evidence_count,
            row.evidence_verified,
            row.defeated_attacks,
            row.applied_in,
            row.v_ok_display(),
            health_display,
            row.entrenchment,
            row.grounding_display(),
            warning_str,
        );
    }

    // Summary
    let total_use: i32 = rows.iter().map(|r| r.total_use()).sum();
    let total_evidence: i32 = rows.iter().map(|r| r.evidence_count).sum();
    let total_verified: i32 = rows.iter().map(|r| r.evidence_verified).sum();
    let with_no_evidence: usize = rows.iter().filter(|r| r.evidence_count == 0).count();
    let with_unverified: usize = rows
        .iter()
        .filter(|r| r.evidence_verified == 0 && r.evidence_count > 0)
        .count();
    let unused: usize = rows.iter().filter(|r| r.total_use() == 0).count();

    // Verification stats
    let beliefs_with_queries: usize = rows.iter().filter(|r| r.verification_total > 0).count();
    let total_queries: i32 = rows.iter().map(|r| r.verification_total).sum();
    let total_passed: i32 = rows.iter().map(|r| r.verification_passed).sum();
    let total_failed: i32 = rows.iter().map(|r| r.verification_failed).sum();
    let total_errored: i32 = rows.iter().map(|r| r.verification_errored).sum();

    // Grounding stats
    let grounded: usize = rows.iter().filter(|r| r.grounding_total() > 0).count();
    let floating: usize = rows.len() - grounded;

    // Freshness stats
    let stale_count = rows
        .iter()
        .filter(|r| match &r.last_activity {
            Some(date_str) => chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map(|d| d <= stale_threshold)
                .unwrap_or(true),
            None => true,
        })
        .count();

    // Median activity age (non-NULL last_activity only)
    let today_naive = chrono::Utc::now().date_naive();
    let mut activity_ages: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.last_activity.as_ref())
        .filter_map(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map(|d| (today_naive - d).num_days())
        .collect();
    activity_ages.sort();
    let median_age = if activity_ages.is_empty() {
        None
    } else {
        Some(activity_ages[activity_ages.len() / 2])
    };

    println!("\n  ── Summary ──");
    println!("  Total beliefs: {}", rows.len());
    println!(
        "  Total citations: {} ({} by beliefs, {} by sessions)",
        total_use,
        rows.iter().map(|r| r.cited_by_beliefs).sum::<i32>(),
        rows.iter().map(|r| r.cited_by_sessions).sum::<i32>()
    );
    println!(
        "  Evidence: {} total, {} verified ({:.0}%)",
        total_evidence,
        total_verified,
        if total_evidence > 0 {
            total_verified as f64 / total_evidence as f64 * 100.0
        } else {
            0.0
        }
    );
    if total_queries > 0 {
        println!(
            "  Verification: {} queries across {} beliefs ({} passed, {} contested, {} errors)",
            total_queries, beliefs_with_queries, total_passed, total_failed, total_errored
        );
    }
    if grounded > 0 || floating > 0 {
        println!("  Grounding: {} grounded, {} floating", grounded, floating);
    }
    // Freshness summary line
    if let Some(median) = median_age {
        println!(
            "  Freshness: {}/{} beliefs stale (>{}d), median activity age {}d",
            stale_count,
            rows.len(),
            stale_days,
            median
        );
    } else {
        println!(
            "  Freshness: {}/{} beliefs stale (>{}d)",
            stale_count,
            rows.len(),
            stale_days
        );
    }
    if warning_count > 0 {
        println!("\n  Warnings: {}", warning_count);
        if with_no_evidence > 0 {
            println!("    {} beliefs with no evidence", with_no_evidence);
        }
        if with_unverified > 0 {
            println!("    {} beliefs with unverified evidence", with_unverified);
        }
        if unused > 0 {
            println!("    {} beliefs with no citations", unused);
        }
        if floating > 0 {
            println!(
                "    {} beliefs floating (no code/commit/session grounding)",
                floating
            );
        }
        if total_failed > 0 {
            println!("    {} beliefs with contested verification", total_failed);
        }
        if total_errored > 0 {
            println!("    {} beliefs with verification errors", total_errored);
        }
        let drifted_count = rows.iter().filter(|r| r.verification_drifted).count();
        if drifted_count > 0 {
            println!("    {} beliefs with verification drift", drifted_count);
        }
        let low_health_count = rows.iter().filter(|r| r.health_score < 0.4).count();
        if low_health_count > 0 {
            println!("    {} beliefs with low health (<0.4)", low_health_count);
        }
        let contested_count = rows.iter().filter(|r| !r.contested_by.is_empty()).count();
        if contested_count > 0 {
            println!("    {} beliefs with active contradictions", contested_count);
        }
    }
    println!();

    // E4.6a: Semantic grounding report
    if show_grounding {
        run_grounding_report(&conn, &rows)?;
    }

    // Emit measurement: belief audit metrics
    patina::measure::emit_or_warn(
        "believe",
        "belief",
        "audit",
        &serde_json::json!({
            "total_beliefs": rows.len(),
            "warnings": warning_count,
            "grounded": grounded,
            "floating": floating,
            "total_citations": total_use,
            "total_evidence": total_evidence,
            "stale": stale_count,
        }),
    );

    Ok(())
}

/// Compute and display semantic grounding for each belief (E4.6a)
///
/// Uses the usearch semantic index to find each belief's nearest neighbors
/// across all content types. Shows what code, commits, and sessions each
/// belief is semantically connected to.
fn run_grounding_report(conn: &Connection, rows: &[BeliefRow]) -> Result<()> {
    // Get embeddings path
    let project_root = std::env::current_dir()?;
    let model = crate::commands::scry::internal::search::get_embedding_model();
    let index_path = patina::paths::project::model_projections_dir(&project_root, &model)
        .join("semantic.usearch");

    if !index_path.exists() {
        println!("  Grounding: semantic index not found. Run `patina oxidize` first.\n");
        return Ok(());
    }

    // Load usearch index
    let index_options = IndexOptions {
        dimensions: 256,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&index_options).context("Failed to create index")?;
    index
        .load(index_path.to_string_lossy().as_ref())
        .context("Failed to load semantic index")?;
    let index_dimensions = index.dimensions() as usize;

    use patina::embeddings::offsets::*;
    const GROUNDING_LIMIT: usize = 20; // Search this many neighbors
    const DISPLAY_LIMIT: usize = 3; // Show top 3 per type

    println!("  ── Semantic Grounding (E4.6a) ──\n");

    let mut grounded_count = 0;
    let mut floating_count = 0;

    for row in rows {
        // Look up belief's rowid
        let rowid: Result<i64, _> =
            conn.query_row("SELECT rowid FROM beliefs WHERE id = ?", [&row.id], |r| {
                r.get(0)
            });

        let rowid = match rowid {
            Ok(r) => r,
            Err(_) => continue,
        };

        let belief_key = (BELIEF_ID_OFFSET + rowid) as u64;

        // Get belief's vector
        let mut vector = vec![0.0_f32; index_dimensions];
        if index.get(belief_key, &mut vector).is_err() {
            continue;
        }

        // Check for zero vector (not in index)
        let magnitude: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if magnitude < 0.001 {
            continue;
        }

        // Search for neighbors
        let matches = match index.search(&vector, GROUNDING_LIMIT + 2) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let results = SearchResults {
            keys: matches.keys,
            distances: matches.distances,
        };

        let enriched = match enrich_results(conn, &results, "semantic", 0.0) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Filter out self entries and categorize
        let mut code_results = Vec::new();
        let mut commit_results = Vec::new();
        let mut session_results = Vec::new();

        for r in &enriched {
            // Skip self — belief appears as both belief.surface and pattern.surface
            // Pattern source_id is now file_path, so check contains for pattern match
            if r.event_type == "belief.surface" && r.source_id == row.id {
                continue;
            }
            if r.event_type.starts_with("pattern.") && r.source_id.contains(&row.id) {
                continue;
            }

            let key = r.id;
            if (CODE_ID_OFFSET..PATTERN_ID_OFFSET).contains(&key) {
                code_results.push(r);
            } else if (COMMIT_ID_OFFSET..BELIEF_ID_OFFSET).contains(&key) {
                commit_results.push(r);
            } else if key < CODE_ID_OFFSET {
                session_results.push(r);
            }
        }

        let has_grounding =
            !code_results.is_empty() || !commit_results.is_empty() || !session_results.is_empty();

        if has_grounding {
            grounded_count += 1;
        } else {
            floating_count += 1;
        }

        // Display
        let display_id = if row.id.len() > 35 {
            format!("{}…", &row.id[..34])
        } else {
            row.id.clone()
        };

        println!(
            "  {} ({}c {}m {}s)",
            display_id,
            code_results.len(),
            commit_results.len(),
            session_results.len()
        );

        // Show top code neighbors
        for r in code_results.iter().take(DISPLAY_LIMIT) {
            println!("    code  {:.3}  {}", r.score, truncate(&r.source_id, 60));
        }
        for r in commit_results.iter().take(DISPLAY_LIMIT) {
            println!("    commit {:.3}  {}", r.score, truncate(&r.content, 60));
        }
        for r in session_results.iter().take(DISPLAY_LIMIT) {
            println!("    session {:.3} {}", r.score, truncate(&r.content, 55));
        }

        if has_grounding {
            println!();
        } else {
            println!("    (floating — no code/commit/session neighbors)\n");
        }
    }

    println!(
        "  ── Grounding Summary: {} grounded, {} floating ──\n",
        grounded_count, floating_count
    );

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max - 1).collect();
    format!("{}…", truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounding_vector_uses_runtime_index_dimensions() {
        let options = IndexOptions {
            dimensions: 384,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            ..Default::default()
        };
        let index = Index::new(&options).expect("create test index");
        let vector = vec![0.0_f32; index.dimensions() as usize];
        assert_eq!(vector.len(), 384);
    }
}
