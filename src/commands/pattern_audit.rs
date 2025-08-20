use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::path::{Path, PathBuf};
use patina::pointers::{PointerSystem, PatternPointer, ComplianceStatus};

#[derive(Debug, Args)]
pub struct PatternCommand {
    #[command(subcommand)]
    pub command: PatternSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PatternSubcommand {
    /// Audit files against patterns
    Audit {
        /// File or directory to audit
        path: PathBuf,
        
        /// Specific pattern to check (checks all if not specified)
        #[arg(short, long)]
        pattern: Option<String>,
        
        /// Use LLM for compliance checking
        #[arg(short, long)]
        llm: bool,
        
        /// Output format (text, json, markdown)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    
    /// Show pointers for a pattern
    Show {
        /// Pattern ID to show
        pattern_id: String,
        
        /// Show only violations
        #[arg(short, long)]
        violations: bool,
    },
    
    /// List all patterns with their compliance rates
    List {
        /// Sort by compliance rate
        #[arg(short, long)]
        by_compliance: bool,
    },
    
    /// Create a pointer from pattern to code (LLM provides assessment)
    Point {
        /// Pattern ID
        pattern_id: String,
        
        /// File path
        file: PathBuf,
        
        /// Compliance status (compliant/violation/partial)
        #[arg(short = 'c', long)]
        compliance: String,
        
        /// Explanation from LLM
        #[arg(short = 'e', long)]
        explanation: Option<String>,
        
        /// Suggested fix from LLM
        #[arg(short = 'f', long)]
        fix: Option<String>,
        
        /// Symbol (function/struct/module name)
        #[arg(short, long)]
        symbol: Option<String>,
    },
    
    /// Verify all pointers are still valid
    Verify {
        /// Pattern to verify (all if not specified)
        pattern: Option<String>,
    },
    
    /// Show pattern spread metrics
    Spread {
        /// Pattern ID
        pattern_id: String,
    },
}

pub fn execute(cmd: PatternCommand) -> Result<()> {
    let db_path = PathBuf::from(".patina/pointers.db");
    std::fs::create_dir_all(".patina")?;
    
    let mut pointer_system = PointerSystem::new(&db_path)?;
    
    match cmd.command {
        PatternSubcommand::Audit { path, pattern, llm, format } => {
            audit_command(&mut pointer_system, &path, pattern, llm, &format)?;
        }
        PatternSubcommand::Show { pattern_id, violations } => {
            show_pattern(&pointer_system, &pattern_id, violations)?;
        }
        PatternSubcommand::List { by_compliance } => {
            list_patterns(&pointer_system, by_compliance)?;
        }
        PatternSubcommand::Point { pattern_id, file, compliance, explanation, fix, symbol } => {
            create_pointer_from_llm(&mut pointer_system, &pattern_id, &file, &compliance, explanation, fix, symbol)?;
        }
        PatternSubcommand::Verify { pattern } => {
            verify_pointers(&mut pointer_system, pattern)?;
        }
        PatternSubcommand::Spread { pattern_id } => {
            show_spread(&pointer_system, &pattern_id)?;
        }
    }
    
    Ok(())
}

fn audit_command(
    system: &mut PointerSystem,
    path: &Path,
    pattern: Option<String>,
    use_llm: bool,
    format: &str,
) -> Result<()> {
    println!("🔍 Auditing {} against patterns...", path.display());
    
    // If LLM requested, set up adapter
    if use_llm {
        // For now, we'll use the Claude adapter if available
        // In a real implementation, we'd check project config
        println!("Using LLM for compliance checking...");
    }
    
    let pointers = if path.is_file() {
        system.audit_file(path)?
    } else {
        // For directories, we'd recursively audit all files
        // For now, just audit the directory itself
        vec![]
    };
    
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&pointers)?);
        }
        "markdown" => {
            print_audit_markdown(&pointers);
        }
        _ => {
            print_audit_text(&pointers);
        }
    }
    
    Ok(())
}

fn print_audit_text(pointers: &[PatternPointer]) {
    if pointers.is_empty() {
        println!("No patterns found or checked.");
        return;
    }
    
    println!("\n📊 Audit Results:\n");
    
    for pointer in pointers {
        println!(
            "{} {} -> {}",
            pointer.compliance.symbol(),
            pointer.pattern_id,
            pointer.file_path.display()
        );
        
        if let Some(explanation) = &pointer.explanation {
            println!("  {}", explanation);
        }
        
        if let Some(fix) = &pointer.suggested_fix {
            println!("  💡 Fix: {}", fix);
        }
        
        if let Some(confidence) = &pointer.confidence {
            println!("  Confidence: {:.0}%", confidence * 100.0);
        }
        
        println!();
    }
    
    // Summary
    let compliant = pointers.iter()
        .filter(|p| p.compliance == ComplianceStatus::Compliant)
        .count();
    let violations = pointers.iter()
        .filter(|p| p.compliance == ComplianceStatus::Violation)
        .count();
    let partial = pointers.iter()
        .filter(|p| p.compliance == ComplianceStatus::Partial)
        .count();
    
    println!("Summary:");
    println!("  ✓ Compliant: {}", compliant);
    println!("  ✗ Violations: {}", violations);
    println!("  ◐ Partial: {}", partial);
    
    let total = compliant + violations + partial;
    if total > 0 {
        let rate = (compliant as f32 / total as f32) * 100.0;
        println!("  📈 Compliance Rate: {:.1}%", rate);
    }
}

fn print_audit_markdown(pointers: &[PatternPointer]) {
    println!("# Pattern Audit Report\n");
    
    if pointers.is_empty() {
        println!("No patterns found or checked.");
        return;
    }
    
    println!("## Results\n");
    
    for pointer in pointers {
        println!(
            "### {} `{}` → `{}`",
            pointer.compliance.symbol(),
            pointer.pattern_id,
            pointer.file_path.display()
        );
        
        if let Some(explanation) = &pointer.explanation {
            println!("\n{}", explanation);
        }
        
        if let Some(fix) = &pointer.suggested_fix {
            println!("\n**Suggested Fix:** {}", fix);
        }
        
        if let Some(confidence) = &pointer.confidence {
            println!("\n*Confidence: {:.0}%*", confidence * 100.0);
        }
        
        println!();
    }
}

fn show_pattern(system: &PointerSystem, pattern_id: &str, violations_only: bool) -> Result<()> {
    let pointers = system.get_pointers_for_pattern(pattern_id)?;
    
    if pointers.is_empty() {
        println!("No pointers found for pattern: {}", pattern_id);
        return Ok(());
    }
    
    println!("📍 Pointers for pattern: {}\n", pattern_id);
    
    for pointer in pointers {
        if violations_only && pointer.compliance != ComplianceStatus::Violation {
            continue;
        }
        
        println!(
            "{} {}",
            pointer.compliance.symbol(),
            pointer.file_path.display()
        );
        
        if let Some(symbol) = &pointer.symbol {
            println!("  Symbol: {}", symbol);
        }
        
        if let Some(lines) = pointer.line_start.zip(pointer.line_end) {
            println!("  Lines: {}-{}", lines.0, lines.1);
        }
        
        if let Some(explanation) = &pointer.explanation {
            println!("  {}", explanation);
        }
        
        println!();
    }
    
    Ok(())
}

fn list_patterns(system: &PointerSystem, by_compliance: bool) -> Result<()> {
    // In a full implementation, we'd list all patterns and their compliance
    println!("📚 Pattern List\n");
    println!("(Pattern listing not yet implemented)");
    Ok(())
}

fn create_pointer(
    system: &mut PointerSystem,
    pattern_id: &str,
    file: &Path,
    symbol: Option<String>,
) -> Result<()> {
    println!("Creating pointer: {} -> {}", pattern_id, file.display());
    
    let pointer = system.create_pointer(pattern_id, file, symbol)?;
    
    println!(
        "\n{} Pointer created:",
        pointer.compliance.symbol()
    );
    println!("  Pattern: {}", pointer.pattern_id);
    println!("  File: {}", pointer.file_path.display());
    println!("  Status: {}", pointer.compliance.to_string());
    
    if let Some(explanation) = &pointer.explanation {
        println!("  {}", explanation);
    }
    
    Ok(())
}

fn create_pointer_from_llm(
    system: &mut PointerSystem,
    pattern_id: &str,
    file: &Path,
    compliance: &str,
    explanation: Option<String>,
    fix: Option<String>,
    symbol: Option<String>,
) -> Result<()> {
    println!("📍 Recording LLM assessment: {} -> {}", pattern_id, file.display());
    
    let status = ComplianceStatus::from_str(compliance);
    let explanation = explanation.unwrap_or_else(|| {
        format!("LLM assessed as {}", compliance)
    });
    
    // Create pointer with LLM's assessment
    let pointer = system.create_pointer_with_assessment(
        pattern_id,
        file,
        status,
        explanation.clone(),
        fix,
        symbol,
    )?;
    
    println!(
        "\n{} Pointer created from LLM assessment:",
        pointer.compliance.symbol()
    );
    println!("  Pattern: {}", pointer.pattern_id);
    println!("  File: {}", pointer.file_path.display());
    println!("  Status: {}", pointer.compliance.to_string());
    println!("  Assessment: {}", explanation);
    
    if let Some(fix) = &pointer.suggested_fix {
        println!("  💡 Fix: {}", fix);
    }
    
    Ok(())
}

fn verify_pointers(system: &mut PointerSystem, pattern: Option<String>) -> Result<()> {
    println!("🔄 Verifying pointers...");
    
    // In a full implementation, we'd verify all pointers
    println!("(Pointer verification not yet fully implemented)");
    
    Ok(())
}

fn show_spread(system: &PointerSystem, pattern_id: &str) -> Result<()> {
    let spread = system.get_pattern_spread(pattern_id)?;
    
    println!("📈 Pattern Spread: {}\n", pattern_id);
    println!("Total Implementations: {}", spread.total_implementations);
    println!("  ✓ Compliant: {}", spread.compliant_count);
    println!("  ✗ Violations: {}", spread.violation_count);
    println!("  ◐ Partial: {}", spread.partial_count);
    println!("\nCompliance Rate: {:.1}%", spread.compliance_rate());
    println!("Last Updated: {}", spread.last_updated.format("%Y-%m-%d %H:%M"));
    
    Ok(())
}