use anyhow::{Context, Result};
use clap::Args;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct PatternLinkArgs {
    /// Analyze pattern-code relationships
    #[arg(long, default_value = "false")]
    analyze: bool,
    
    /// Check if code follows patterns
    #[arg(long, default_value = "false")]
    validate: bool,
    
    /// Show detailed output
    #[arg(long, short = 'v', default_value = "false")]
    verbose: bool,
}

/// Link patterns to code and validate implementation
pub fn execute(args: PatternLinkArgs) -> Result<()> {
    if args.analyze {
        analyze_pattern_usage()?;
    }
    
    if args.validate {
        validate_pattern_compliance()?;
    }
    
    if !args.analyze && !args.validate {
        // Default: show current linkage
        show_pattern_code_links()?;
    }
    
    Ok(())
}

fn analyze_pattern_usage() -> Result<()> {
    println!("🔍 Analyzing pattern-code relationships...\n");
    
    // Find all patterns that define code structure
    let structural_patterns = find_structural_patterns()?;
    
    // Find code that claims to implement patterns
    let pattern_implementations = find_pattern_implementations()?;
    
    // Find code that SHOULD follow patterns (based on structure)
    let implicit_patterns = detect_implicit_patterns()?;
    
    println!("📊 Pattern Usage Analysis:");
    println!("─────────────────────────");
    
    println!("\n🎯 Patterns with implementations:");
    for (pattern, files) in &pattern_implementations {
        println!("  {} → {} files", pattern, files.len());
        if files.len() <= 3 {
            for file in files {
                println!("    - {}", file.display());
            }
        }
    }
    
    println!("\n❓ Code that might follow patterns (detected):");
    for (pattern, files) in &implicit_patterns {
        println!("  {} → {} potential files", pattern, files.len());
    }
    
    println!("\n⚠️  Patterns with no implementations:");
    for pattern in &structural_patterns {
        if !pattern_implementations.contains_key(pattern) {
            println!("  - {}", pattern);
        }
    }
    
    Ok(())
}

fn find_structural_patterns() -> Result<Vec<String>> {
    let mut patterns = Vec::new();
    
    // Look for patterns that define code structure
    for layer in ["core", "surface"] {
        let layer_path = Path::new("layer").join(layer);
        if !layer_path.exists() { continue; }
        
        for entry in fs::read_dir(&layer_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let content = fs::read_to_string(&path)?;
                
                // Pattern defines structure if it mentions:
                // - File organization (mod.rs, internal.rs)
                // - Code structure patterns
                // - Implementation rules
                if content.contains("mod.rs") || 
                   content.contains("impl ") ||
                   content.contains("pub struct") ||
                   content.contains("structure:") {
                    
                    let pattern_name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    patterns.push(pattern_name);
                }
            }
        }
    }
    
    Ok(patterns)
}

fn find_pattern_implementations() -> Result<HashMap<String, Vec<PathBuf>>> {
    let mut implementations = HashMap::new();
    
    // Look for code that explicitly references patterns
    for entry in walkdir::WalkDir::new("src") {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content = fs::read_to_string(path)?;
            
            // Check for pattern references in comments
            for line in content.lines() {
                if line.contains("// Pattern:") || 
                   line.contains("// Implements:") ||
                   line.contains("// Following:") {
                    
                    // Extract pattern name
                    if let Some(pattern) = extract_pattern_name(line) {
                        implementations.entry(pattern)
                            .or_insert_with(Vec::new)
                            .push(path.to_path_buf());
                    }
                }
            }
            
            // Check for DEPENDABLE_RUST pattern specifically
            if path.components().any(|c| c.as_os_str() == "internal") {
                if let Some(parent) = path.parent() {
                    if parent.join("mod.rs").exists() {
                        implementations.entry("dependable-rust".to_string())
                            .or_insert_with(Vec::new)
                            .push(parent.to_path_buf());
                    }
                }
            }
        }
    }
    
    Ok(implementations)
}

fn detect_implicit_patterns() -> Result<HashMap<String, Vec<PathBuf>>> {
    let mut detected = HashMap::new();
    
    // Detect DEPENDABLE_RUST pattern (mod.rs + internal/)
    for entry in walkdir::WalkDir::new("src") {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            let mod_file = path.join("mod.rs");
            let internal_dir = path.join("internal");
            let internal_file = path.join("internal.rs");
            
            if mod_file.exists() && (internal_dir.exists() || internal_file.exists()) {
                detected.entry("dependable-rust".to_string())
                    .or_insert_with(Vec::new)
                    .push(path.to_path_buf());
            }
        }
    }
    
    // Detect adapter pattern
    if Path::new("src/adapters").exists() {
        for entry in fs::read_dir("src/adapters")? {
            let entry = entry?;
            if entry.path().is_dir() {
                detected.entry("adapter-pattern".to_string())
                    .or_insert_with(Vec::new)
                    .push(entry.path());
            }
        }
    }
    
    Ok(detected)
}

fn validate_pattern_compliance() -> Result<()> {
    println!("✓ Validating pattern compliance...\n");
    
    let detected = detect_implicit_patterns()?;
    
    for (pattern, paths) in detected {
        println!("Checking {} pattern:", pattern);
        
        for path in paths {
            let valid = match pattern.as_str() {
                "dependable-rust" => validate_dependable_rust(&path)?,
                "adapter-pattern" => validate_adapter_pattern(&path)?,
                _ => true,
            };
            
            if valid {
                println!("  ✓ {} follows pattern", path.display());
            } else {
                println!("  ✗ {} violates pattern", path.display());
            }
        }
    }
    
    Ok(())
}

fn validate_dependable_rust(path: &Path) -> Result<bool> {
    let mod_file = path.join("mod.rs");
    
    if !mod_file.exists() {
        return Ok(false);
    }
    
    let content = fs::read_to_string(&mod_file)?;
    let line_count = content.lines().count();
    
    // DEPENDABLE_RUST rule: mod.rs should be ≤150 lines
    if line_count > 150 {
        println!("    mod.rs has {} lines (should be ≤150)", line_count);
        return Ok(false);
    }
    
    // Check for proper documentation
    if !content.contains("//!") && !content.contains("///") {
        println!("    mod.rs lacks documentation");
        return Ok(false);
    }
    
    Ok(true)
}

fn validate_adapter_pattern(path: &Path) -> Result<bool> {
    // Adapter should have a consistent interface
    let mod_file = path.join("mod.rs");
    
    if !mod_file.exists() {
        return Ok(false);
    }
    
    let content = fs::read_to_string(&mod_file)?;
    
    // Should implement standard adapter interface
    let has_interface = content.contains("impl Adapter") || 
                        content.contains("impl LLMAdapter");
    
    Ok(has_interface)
}

fn show_pattern_code_links() -> Result<()> {
    println!("🔗 Pattern-Code Linkage\n");
    
    // Show which modules follow which patterns
    let detected = detect_implicit_patterns()?;
    
    println!("Detected pattern usage:");
    for (pattern, paths) in detected {
        println!("\n{}:", pattern);
        for path in paths {
            // Check if explicitly documented
            let mod_file = path.join("mod.rs");
            let documented = if mod_file.exists() {
                let content = fs::read_to_string(&mod_file)?;
                content.contains(&format!("Pattern: {}", pattern)) ||
                content.contains(&format!("Following: {}", pattern))
            } else {
                false
            };
            
            let status = if documented { "✓ documented" } else { "⚠️  implicit" };
            println!("  {} - {}", path.display(), status);
        }
    }
    
    Ok(())
}

fn extract_pattern_name(line: &str) -> Option<String> {
    // Extract pattern name from comments like:
    // Pattern: dependable-rust
    // Implements: adapter-pattern
    
    if let Some(idx) = line.find("Pattern:") {
        let name = line[idx + 8..].trim();
        return Some(name.to_string());
    }
    
    if let Some(idx) = line.find("Implements:") {
        let name = line[idx + 11..].trim();
        return Some(name.to_string());
    }
    
    if let Some(idx) = line.find("Following:") {
        let name = line[idx + 10..].trim();
        return Some(name.to_string());
    }
    
    None
}

// Need walkdir crate for recursive directory walking
use walkdir::WalkDir;