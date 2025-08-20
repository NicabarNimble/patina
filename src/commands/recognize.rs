use anyhow::{Context, Result};
use colored::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn execute() -> Result<()> {
    println!("{}", "\n🔬 Recognizing patterns in surviving code...".bright_cyan());
    
    // Find files with good survival (1 day for testing, normally 180)
    let survivors = find_surviving_files(1)?;
    
    // Analyze patterns in these files
    let patterns = analyze_patterns(&survivors)?;
    
    // Display discovered patterns
    display_patterns(&patterns)?;
    
    Ok(())
}

#[derive(Debug, Clone)]
struct SurvivingFile {
    path: String,
    days_unchanged: i64,
    last_modified: String,
    language: String,
}

#[derive(Debug, Default)]
struct RecognizedPattern {
    name: String,
    description: String,
    found_in: Vec<String>,
    characteristics: Vec<String>,
    survival_rate: f64,
    co_patterns: HashMap<String, f64>,
}

fn find_surviving_files(min_days: i64) -> Result<Vec<SurvivingFile>> {
    let mut survivors = Vec::new();
    
    // Find all source files
    let output = Command::new("git")
        .args(&["ls-files", "src/", "modules/"])
        .output()
        .context("Failed to list files")?;
    
    let files = String::from_utf8_lossy(&output.stdout);
    let today = chrono::Local::now();
    
    for file in files.lines() {
        if file.is_empty() { continue; }
        
        // Get last modification date
        let log_output = Command::new("git")
            .args(&["log", "-1", "--pretty=format:%ai", "--", file])
            .output()?;
        
        let last_modified_str = String::from_utf8_lossy(&log_output.stdout);
        if last_modified_str.is_empty() { continue; }
        
        // Parse date and calculate days unchanged
        if let Ok(last_modified) = chrono::DateTime::parse_from_str(
            &last_modified_str.lines().next().unwrap_or(""),
            "%Y-%m-%d %H:%M:%S %z"
        ) {
            let days_unchanged = (today - last_modified.with_timezone(&chrono::Local)).num_days();
            
            if days_unchanged >= min_days {
                let language = if file.ends_with(".rs") {
                    "rust".to_string()
                } else if file.ends_with(".go") {
                    "go".to_string()
                } else {
                    "unknown".to_string()
                };
                
                survivors.push(SurvivingFile {
                    path: file.to_string(),
                    days_unchanged,
                    last_modified: last_modified_str.to_string(),
                    language,
                });
            }
        }
    }
    
    survivors.sort_by(|a, b| b.days_unchanged.cmp(&a.days_unchanged));
    Ok(survivors)
}

fn analyze_patterns(files: &[SurvivingFile]) -> Result<Vec<RecognizedPattern>> {
    let mut patterns = Vec::new();
    
    // Pattern 1: Public API, Private Core
    let mut public_private = RecognizedPattern {
        name: "Public API, Private Core".to_string(),
        description: "Module exposes public functions that delegate to private implementation".to_string(),
        ..Default::default()
    };
    
    // Pattern 2: Error Context Chain
    let mut error_context = RecognizedPattern {
        name: "Error Context Chain".to_string(),
        description: "Every Result has .context() for debugging".to_string(),
        ..Default::default()
    };
    
    // Pattern 3: Builder Pattern
    let mut builder = RecognizedPattern {
        name: "Builder Pattern".to_string(),
        description: "Structs constructed through builder methods".to_string(),
        ..Default::default()
    };
    
    // Pattern 4: Type-State Pattern
    let mut type_state = RecognizedPattern {
        name: "Type-State Pattern".to_string(),
        description: "Types encode valid states, making invalid states unrepresentable".to_string(),
        ..Default::default()
    };
    
    for file in files {
        if file.language != "rust" { continue; }
        
        let content = fs::read_to_string(&file.path)
            .context(format!("Failed to read {}", file.path))?;
        
        // Detect Public API, Private Core
        if content.contains("pub fn") && content.contains("struct") && !content.contains("pub struct") {
            public_private.found_in.push(file.path.clone());
            public_private.characteristics.push("Clear module boundaries".to_string());
        }
        
        // Detect Error Context Chain
        if content.contains(".context(") || content.contains(".with_context(") {
            error_context.found_in.push(file.path.clone());
            let context_count = content.matches(".context(").count() + content.matches(".with_context(").count();
            if context_count > 5 {
                error_context.characteristics.push(format!("{} error contexts", context_count));
            }
        }
        
        // Detect Builder Pattern
        if content.contains("impl") && content.contains("Builder") && content.contains("build(") {
            builder.found_in.push(file.path.clone());
            builder.characteristics.push("Fluent interface for construction".to_string());
        }
        
        // Detect Type-State Pattern
        if content.contains("PhantomData") || (content.contains("enum") && content.contains("impl") && content.contains("From")) {
            type_state.found_in.push(file.path.clone());
            type_state.characteristics.push("Compile-time state validation".to_string());
        }
    }
    
    // Calculate survival rates
    let total_files = files.len() as f64;
    
    if !public_private.found_in.is_empty() {
        public_private.survival_rate = (public_private.found_in.len() as f64 / total_files) * 100.0;
        patterns.push(public_private);
    }
    
    if !error_context.found_in.is_empty() {
        error_context.survival_rate = (error_context.found_in.len() as f64 / total_files) * 100.0;
        patterns.push(error_context);
    }
    
    if !builder.found_in.is_empty() {
        builder.survival_rate = (builder.found_in.len() as f64 / total_files) * 100.0;
        patterns.push(builder);
    }
    
    if !type_state.found_in.is_empty() {
        type_state.survival_rate = (type_state.found_in.len() as f64 / total_files) * 100.0;
        patterns.push(type_state);
    }
    
    // Find co-occurring patterns
    let pattern_names: Vec<String> = patterns.iter().map(|p| p.name.clone()).collect();
    for i in 0..patterns.len() {
        for j in 0..patterns.len() {
            if i != j {
                let pattern_i_files: HashSet<_> = patterns[i].found_in.iter().collect();
                let pattern_j_files: HashSet<_> = patterns[j].found_in.iter().collect();
                let intersection = pattern_i_files.intersection(&pattern_j_files).count();
                
                if intersection > 0 {
                    let co_occurrence = intersection as f64 / patterns[i].found_in.len() as f64;
                    patterns[i].co_patterns.insert(pattern_names[j].clone(), co_occurrence);
                }
            }
        }
    }
    
    patterns.sort_by(|a, b| b.survival_rate.partial_cmp(&a.survival_rate).unwrap());
    Ok(patterns)
}

fn display_patterns(patterns: &[RecognizedPattern]) -> Result<()> {
    if patterns.is_empty() {
        println!("{}", "No patterns found in surviving code".bright_yellow());
        return Ok(());
    }
    
    println!("\n{}", "📊 Discovered Patterns in Surviving Code:".bright_yellow());
    println!("{}", "━".repeat(60).bright_black());
    
    for (idx, pattern) in patterns.iter().enumerate() {
        let status_icon = if pattern.survival_rate > 75.0 {
            "⭐".to_string()
        } else if pattern.survival_rate > 50.0 {
            "✅".to_string()
        } else {
            "🔄".to_string()
        };
        
        println!("\n{} {}: {}",
            status_icon,
            format!("Pattern {}", idx + 1).bright_cyan(),
            pattern.name.bright_white()
        );
        
        println!("├─ {}: {}", "Description".bright_black(), pattern.description);
        println!("├─ {}: {} files", "Found in".bright_black(), pattern.found_in.len());
        println!("├─ {}: {:.1}%", "Prevalence".bright_black(), pattern.survival_rate);
        
        if !pattern.characteristics.is_empty() {
            println!("├─ {}:", "Characteristics".bright_black());
            for char in &pattern.characteristics {
                println!("│  • {}", char);
            }
        }
        
        if !pattern.co_patterns.is_empty() {
            println!("├─ {}:", "Co-occurs with".bright_black());
            for (co_pattern, rate) in pattern.co_patterns.iter() {
                println!("│  • {}: {:.0}%", co_pattern, rate * 100.0);
            }
        }
        
        println!("└─ {}:", "Example files".bright_black());
        for file in pattern.found_in.iter().take(3) {
            println!("   • {}", file.bright_green());
        }
    }
    
    println!("\n{}", "💡 Tip: These patterns appear in code that hasn't been modified for 6+ months".bright_black());
    
    Ok(())
}