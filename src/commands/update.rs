use anyhow::{Context, Result};
use patina::session::SessionManager;
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct UpdateResult {
    adapter: String,
    current_version: Option<String>,
    available_version: Option<String>,
    update_available: bool,
    updated: bool,
    error: Option<String>,
}

pub fn execute(check_only: bool, auto_yes: bool, auto_no: bool, json_output: bool) -> Result<i32> {
    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;
    
    // Check for non-interactive mode via environment variable
    let non_interactive = auto_yes || auto_no || json_output ||
        std::env::var("PATINA_NONINTERACTIVE").is_ok();
    
    if !json_output {
        println!("🔍 Checking for adapter updates...");
    }
    
    // Read project config
    let config_path = project_root.join(".patina").join("config.json");
    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read project config")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    let llm = config.get("llm")
        .and_then(|l| l.as_str())
        .unwrap_or("claude");
    
    // Check for updates using adapter
    let adapter = patina::adapters::get_adapter(llm);
    
    let mut result = UpdateResult {
        adapter: llm.to_string(),
        current_version: None,
        available_version: None,
        update_available: false,
        updated: false,
        error: None,
    };
    
    match adapter.check_for_updates(&project_root)? {
        Some((current, available)) => {
            result.current_version = Some(current.clone());
            result.available_version = Some(available.clone());
            result.update_available = true;
            
            if !json_output {
                println!("📦 {} adapter update available: {} → {}", llm, current, available);
                
                // Show what's new if we're Claude adapter
                if llm == "claude" {
                    println!("\nWhat's new:");
                    // This is a simplified changelog display - could be enhanced
                    println!("  - Fixed: Scripts now properly stored in .claude/bin/ directory");
                    println!("  - Added: Adapter versioning and update mechanism");
                    println!("  - Improved: Session commands use correct paths");
                }
            }
            
            if check_only || auto_no {
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                return Ok(if check_only { 2 } else { 0 });
            }
            
            // Determine whether to update
            let should_update = if non_interactive {
                auto_yes || std::env::var("PATINA_AUTO_APPROVE").is_ok()
            } else {
                // Interactive prompt
                print!("\nUpdate adapter files? [Y/n] ");
                use std::io::{self, Write};
                io::stdout().flush()?;
                let mut response = String::new();
                io::stdin().read_line(&mut response)?;
                response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y")
            };
            
            if should_update {
                adapter.update_adapter_files(&project_root)?;
                result.updated = true;
                
                if !json_output {
                    println!("\n✨ Adapter files updated successfully!");
                    println!("\nNote: Use 'patina push' to regenerate {}", 
                        adapter.get_context_file_path(&project_root).file_name().unwrap().to_string_lossy());
                }
            } else if !json_output {
                println!("Update cancelled.");
            }
        }
        None => {
            result.update_available = false;
            if !json_output {
                println!("✓ {} adapter is up to date (no updates available)", llm);
            }
        }
    }
    
    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    
    Ok(0)
}

