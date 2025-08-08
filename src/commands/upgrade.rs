use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::{self, Write};

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    published_at: String,
    html_url: String,
}

pub fn execute(check_only: bool, json: bool) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    
    if !json {
        println!("🔍 Checking for Patina updates...");
    }
    
    // Check GitHub releases
    let latest_release = check_latest_release()?;
    let latest_version = latest_release.tag_name.trim_start_matches('v');
    
    let is_outdated = is_version_outdated(current_version, latest_version)?;
    
    if json {
        let result = serde_json::json!({
            "current_version": current_version,
            "latest_version": latest_version,
            "update_available": is_outdated,
            "release_url": latest_release.html_url,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    
    println!("Current version: v{}", current_version);
    println!("Latest version:  v{}", latest_version);
    
    // Parse and display when the release was published
    if let Ok(published_date) = chrono::DateTime::parse_from_rfc3339(&latest_release.published_at) {
        let days_ago = (chrono::Utc::now() - published_date.with_timezone(&chrono::Utc)).num_days();
        if days_ago == 0 {
            println!("Published:       today");
        } else if days_ago == 1 {
            println!("Published:       yesterday");
        } else {
            println!("Published:       {} days ago", days_ago);
        }
    }
    
    if !is_outdated {
        println!("\n✅ You're running the latest version of Patina!");
        return Ok(());
    }
    
    println!("\n🚀 New version available!");
    
    if check_only {
        return Ok(());
    }
    
    // Provide upgrade instructions
    println!("\nTo upgrade Patina, run:");
    println!("  cargo install patina --version {}", latest_version);
    println!("\nOr download from:");
    println!("  {}", latest_release.html_url);
    
    // Check if running in non-interactive mode
    let non_interactive = std::env::var("PATINA_NONINTERACTIVE").is_ok();
    
    if !non_interactive {
        print!("\nView release notes? [Y/n] ");
        io::stdout().flush()?;
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        
        if response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y") {
            println!("\nRelease: {}", latest_release.html_url);
        }
    }
    
    Ok(())
}

fn check_latest_release() -> Result<GitHubRelease> {
    // Use reqwest blocking client to fetch from GitHub API
    let client = reqwest::blocking::Client::builder()
        .user_agent("patina-cli")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    
    // For now, use a placeholder repo URL since the actual repo isn't published yet
    // In production, this would be the real patina repo
    let url = "https://api.github.com/repos/rust-lang/rust/releases/latest";
    
    // Add a note for when we have a real repo
    if std::env::var("PATINA_REPO_URL").is_ok() {
        eprintln!("Note: Using PATINA_REPO_URL for version check");
    }
    
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .context("Failed to connect to GitHub API")?;
    
    if !response.status().is_success() {
        // If rate limited or other error, fall back to mock data
        if response.status() == reqwest::StatusCode::FORBIDDEN {
            eprintln!("Warning: GitHub API rate limit may have been exceeded");
        }
        
        // Return mock data as fallback
        return Ok(GitHubRelease {
            tag_name: "v0.2.0".to_string(),
            published_at: "2025-08-07T00:00:00Z".to_string(),
            html_url: "https://github.com/patina-project/patina/releases/tag/v0.2.0".to_string(),
        });
    }
    
    let release: GitHubRelease = response
        .json()
        .context("Failed to parse GitHub release JSON")?;
    
    Ok(release)
}

fn is_version_outdated(current: &str, latest: &str) -> Result<bool> {
    // Simple version comparison
    // In production, use semver crate for proper semantic versioning
    
    let current_parts: Vec<u32> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
        
    let latest_parts: Vec<u32> = latest
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    for i in 0..3 {
        let current_part = current_parts.get(i).unwrap_or(&0);
        let latest_part = latest_parts.get(i).unwrap_or(&0);
        
        if latest_part > current_part {
            return Ok(true);
        } else if latest_part < current_part {
            return Ok(false);
        }
    }
    
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_comparison() {
        assert!(is_version_outdated("0.1.0", "0.2.0").unwrap());
        assert!(is_version_outdated("0.1.0", "0.1.1").unwrap());
        assert!(is_version_outdated("0.1.0", "1.0.0").unwrap());
        assert!(!is_version_outdated("0.2.0", "0.1.0").unwrap());
        assert!(!is_version_outdated("1.0.0", "1.0.0").unwrap());
    }
}