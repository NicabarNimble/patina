use anyhow::{Context, Result};
use patina::layer::{Layer, Pattern, PatternType};
use patina::session::SessionManager;
use std::io::{self, Write};

pub fn execute(message: String) -> Result<()> {
    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;
    
    // Get session manager
    let session_manager = SessionManager::new(&project_root);
    
    // Get current session
    let mut session = session_manager.current_session()?
        .context("No active session. Use 'patina add' to start a session.")?;
    
    // Get uncommitted patterns
    let uncommitted = session.uncommitted_patterns();
    
    if uncommitted.is_empty() {
        println!("No uncommitted patterns in session.");
        return Ok(());
    }
    
    println!("📝 Patterns to commit:");
    for pattern in &uncommitted {
        println!("  - {} '{}'", pattern.pattern_type, pattern.name);
    }
    
    // Initialize layer
    let layer_path = project_root.join("layer");
    let layer = Layer::new(&layer_path);
    
    // Get project name from config
    let config_path = project_root.join(".patina").join("config.json");
    let config_content = std::fs::read_to_string(&config_path)
        .context("Failed to read project config")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    let project_name = config.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown");
    
    // For each pattern, prompt for content if not already provided
    let mut committed_names = Vec::new();
    
    for pattern in uncommitted {
        println!("\n🎯 Pattern: {} '{}'", pattern.pattern_type, pattern.name);
        
        // Determine pattern type for layer storage
        let pattern_type = match pattern.pattern_type.as_str() {
            "core" => PatternType::Core,
            "topic" => {
                print!("Topic name (e.g., 'architecture', 'testing'): ");
                io::stdout().flush()?;
                let mut topic = String::new();
                io::stdin().read_line(&mut topic)?;
                PatternType::Topic(topic.trim().to_string())
            }
            "project" | "decision" | "constraint" | "principle" => {
                PatternType::Project(project_name.to_string())
            }
            _ => {
                println!("Unknown pattern type '{}', storing as project pattern", pattern.pattern_type);
                PatternType::Project(project_name.to_string())
            }
        };
        
        // Get pattern content
        let content = if let Some(ref existing_content) = pattern.content {
            existing_content.clone()
        } else {
            println!("Enter pattern content (press Enter twice to finish):");
            let mut lines = Vec::new();
            let mut empty_count = 0;
            
            loop {
                let mut line = String::new();
                io::stdin().read_line(&mut line)?;
                
                if line.trim().is_empty() {
                    empty_count += 1;
                    if empty_count >= 2 {
                        break;
                    }
                } else {
                    empty_count = 0;
                }
                
                lines.push(line);
            }
            
            lines.join("")
        };
        
        // Create pattern with proper markdown formatting
        let formatted_content = format!(
            "# {}\n\n{}\n\n## Metadata\n- Type: {}\n- Added: {}\n- Commit: {}\n",
            pattern.name,
            content.trim(),
            pattern.pattern_type,
            pattern.added_at,
            message
        );
        
        // Store pattern in layer
        let layer_pattern = Pattern {
            name: pattern.name.clone(),
            pattern_type,
            content: formatted_content,
        };
        
        layer.store_pattern(&layer_pattern)?;
        committed_names.push(pattern.name.clone());
        
        println!("✓ Committed '{}' to layer", pattern.name);
    }
    
    // Mark patterns as committed in session
    session.mark_committed(&committed_names);
    session_manager.save_session(&session)?;
    
    println!("\n✨ Committed {} patterns to layer", committed_names.len());
    println!("   Message: {}", message);
    
    Ok(())
}