use anyhow::{Context, Result};
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn execute() -> Result<()> {
    println!(
        "{}",
        "\n🔗 Connecting ideas to implementations...".bright_cyan()
    );

    // Find all ideas in docs
    let ideas = find_ideas()?;

    // Find implementations in code
    let implementations = find_implementations(&ideas)?;

    // Analyze connections
    let connections = analyze_connections(&ideas, &implementations)?;

    // Display connections
    display_connections(&connections)?;

    Ok(())
}

#[derive(Debug, Clone)]
struct Idea {
    name: String,
    file: String,
    _content_snippet: String,
    last_updated: String,
}

#[derive(Debug, Clone)]
struct Implementation {
    idea_name: String,
    files: Vec<String>,
    survival_days: i64,
    pattern_detected: Option<String>,
}

#[derive(Debug)]
struct Connection {
    idea: Idea,
    implementation: Option<Implementation>,
    status: ConnectionStatus,
    survival_rate: f64,
}

#[derive(Debug)]
enum ConnectionStatus {
    Thriving,
    Stable,
    Evolving,
    NotImplemented,
    Abandoned,
}

fn find_ideas() -> Result<Vec<Idea>> {
    let mut ideas = Vec::new();

    // Search for markdown files in layer/
    let output = Command::new("find")
        .args(["layer", "-name", "*.md", "-type", "f"])
        .output()
        .context("Failed to find markdown files")?;

    let files = String::from_utf8_lossy(&output.stdout);

    for file in files.lines() {
        if file.is_empty() || file.contains("/sessions/") {
            continue;
        }

        let content = fs::read_to_string(file).context(format!("Failed to read {file}"))?;

        // Extract idea name from frontmatter or heading
        let idea_name = if let Some(id_line) = content.lines().find(|l| l.starts_with("id:")) {
            id_line.replace("id:", "").trim().to_string()
        } else if let Some(heading) = content.lines().find(|l| l.starts_with("# ")) {
            heading.replace("# ", "").trim().to_string()
        } else {
            Path::new(file)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };

        // Get snippet of content
        let snippet = content
            .lines()
            .find(|l| !l.starts_with("#") && !l.starts_with("---") && !l.trim().is_empty())
            .unwrap_or("")
            .to_string();

        // Get last update time
        let log_output = Command::new("git")
            .args(["log", "-1", "--pretty=format:%ai", "--", file])
            .output()?;

        let last_updated = String::from_utf8_lossy(&log_output.stdout).to_string();

        ideas.push(Idea {
            name: idea_name,
            file: file.to_string(),
            _content_snippet: snippet,
            last_updated,
        });
    }

    Ok(ideas)
}

fn find_implementations(ideas: &[Idea]) -> Result<Vec<Implementation>> {
    let mut implementations = Vec::new();
    let today = chrono::Local::now();

    for idea in ideas {
        // Search for idea name in code
        let underscore_variant = idea.name.replace("-", "_");
        let dash_variant = idea.name.replace("_", "-");
        let search_terms = vec![&idea.name, &underscore_variant, &dash_variant];

        let mut impl_files = Vec::new();
        let mut total_survival_days = 0i64;
        let mut file_count = 0;

        for term in search_terms {
            let output = Command::new("grep")
                .args(["-r", "-l", term, "src/", "modules/"])
                .output()
                .context("Failed to search code")?;

            let files = String::from_utf8_lossy(&output.stdout);

            for file in files.lines() {
                if file.is_empty() || impl_files.contains(&file.to_string()) {
                    continue;
                }

                impl_files.push(file.to_string());

                // Calculate survival time
                let log_output = Command::new("git")
                    .args(["log", "-1", "--pretty=format:%ai", "--", file])
                    .output()?;

                let last_modified_str = String::from_utf8_lossy(&log_output.stdout);
                if let Ok(last_modified) = chrono::DateTime::parse_from_str(
                    last_modified_str.lines().next().unwrap_or(""),
                    "%Y-%m-%d %H:%M:%S %z",
                ) {
                    let days = (today - last_modified.with_timezone(&chrono::Local)).num_days();
                    total_survival_days += days;
                    file_count += 1;
                }
            }
        }

        if !impl_files.is_empty() {
            let avg_survival = if file_count > 0 {
                total_survival_days / file_count as i64
            } else {
                0
            };

            // Try to detect pattern from file content
            let pattern = detect_pattern_in_files(&impl_files)?;

            implementations.push(Implementation {
                idea_name: idea.name.clone(),
                files: impl_files,
                survival_days: avg_survival,
                pattern_detected: pattern,
            });
        }
    }

    Ok(implementations)
}

fn detect_pattern_in_files(files: &[String]) -> Result<Option<String>> {
    for file in files {
        if !file.ends_with(".rs") {
            continue;
        }

        let content = fs::read_to_string(file).ok().unwrap_or_default();

        // Simple pattern detection
        if content.contains("pub fn") && !content.contains("pub struct") {
            return Ok(Some("Public API, Private Core".to_string()));
        }
        if content.contains(".context(") {
            return Ok(Some("Error Context Chain".to_string()));
        }
        if content.contains("Builder") && content.contains("build(") {
            return Ok(Some("Builder Pattern".to_string()));
        }
    }

    Ok(None)
}

fn analyze_connections(
    ideas: &[Idea],
    implementations: &[Implementation],
) -> Result<Vec<Connection>> {
    let mut connections = Vec::new();

    for idea in ideas {
        let implementation = implementations
            .iter()
            .find(|i| i.idea_name == idea.name)
            .cloned();

        let (status, survival_rate) = match &implementation {
            Some(impl_) if impl_.survival_days > 180 => (ConnectionStatus::Thriving, 95.0),
            Some(impl_) if impl_.survival_days > 90 => (ConnectionStatus::Stable, 75.0),
            Some(impl_) if impl_.survival_days > 30 => (ConnectionStatus::Evolving, 50.0),
            Some(_) => (ConnectionStatus::Evolving, 25.0),
            None => {
                // Check if idea is old but not implemented
                if idea.last_updated.len() > 10 {
                    let idea_date = &idea.last_updated[..10];
                    if idea_date < "2025-06-01" {
                        (ConnectionStatus::Abandoned, 0.0)
                    } else {
                        (ConnectionStatus::NotImplemented, 0.0)
                    }
                } else {
                    (ConnectionStatus::NotImplemented, 0.0)
                }
            }
        };

        connections.push(Connection {
            idea: idea.clone(),
            implementation,
            status,
            survival_rate,
        });
    }

    // Sort by survival rate
    connections.sort_by(|a, b| b.survival_rate.partial_cmp(&a.survival_rate).unwrap());

    Ok(connections)
}

fn display_connections(connections: &[Connection]) -> Result<()> {
    println!(
        "\n{}",
        "📊 Ideas → Implementation → Pattern Connections:".bright_yellow()
    );
    println!("{}", "━".repeat(70).bright_black());

    // Group by status
    let mut by_status: HashMap<String, Vec<&Connection>> = HashMap::new();

    for conn in connections {
        let status_key = format!("{:?}", conn.status);
        by_status.entry(status_key).or_default().push(conn);
    }

    // Display Thriving connections first
    if let Some(thriving) = by_status.get("Thriving") {
        println!(
            "\n⭐ {} ({})",
            "Thriving Patterns".bright_green(),
            thriving.len()
        );
        for conn in thriving {
            display_connection(conn)?;
        }
    }

    // Display Stable connections
    if let Some(stable) = by_status.get("Stable") {
        println!(
            "\n✅ {} ({})",
            "Stable Patterns".bright_yellow(),
            stable.len()
        );
        for conn in stable {
            display_connection(conn)?;
        }
    }

    // Display Evolving connections
    if let Some(evolving) = by_status.get("Evolving") {
        println!(
            "\n🔄 {} ({})",
            "Evolving Patterns".bright_cyan(),
            evolving.len()
        );
        for conn in evolving.iter().take(3) {
            display_connection(conn)?;
        }
    }

    // Display Not Implemented
    if let Some(not_impl) = by_status.get("NotImplemented") {
        println!(
            "\n💭 {} ({})",
            "Ideas Not Yet Implemented".bright_black(),
            not_impl.len()
        );
        for conn in not_impl.iter().take(3) {
            println!(
                "  • {} ({})",
                conn.idea.name.bright_white(),
                conn.idea.file.bright_black()
            );
        }
    }

    // Summary statistics
    let total = connections.len();
    let implemented = connections
        .iter()
        .filter(|c| c.implementation.is_some())
        .count();
    let success_rate = if total > 0 {
        (implemented as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    println!("\n{}", "📈 Summary:".bright_yellow());
    println!("  Total Ideas: {total}");
    println!("  Implemented: {implemented} ({success_rate:.0}%)");
    println!(
        "  Average Survival: {} days",
        connections
            .iter()
            .filter_map(|c| c.implementation.as_ref())
            .map(|i| i.survival_days)
            .sum::<i64>()
            / implemented.max(1) as i64
    );

    Ok(())
}

fn display_connection(conn: &Connection) -> Result<()> {
    println!(
        "\n{} {} (doc)",
        "📝".bright_blue(),
        conn.idea.name.bright_white()
    );
    println!("  └─ {}", conn.idea.file.bright_black());

    if let Some(impl_) = &conn.implementation {
        println!(
            "  {} {} (implementation)",
            "→".bright_green(),
            format!("{} files", impl_.files.len()).bright_green()
        );

        for file in impl_.files.iter().take(2) {
            println!("     • {}", file.bright_green());
        }

        if let Some(pattern) = &impl_.pattern_detected {
            println!(
                "  {} {} (emergent pattern)",
                "→".bright_cyan(),
                pattern.bright_cyan()
            );
        }

        println!(
            "  {} {}% survival rate ({} days avg)",
            "→".bright_yellow(),
            conn.survival_rate as i32,
            impl_.survival_days
        );
    }

    Ok(())
}
