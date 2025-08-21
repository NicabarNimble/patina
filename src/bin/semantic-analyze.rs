use anyhow::Result;
use patina::semantic::analyzer::SemanticAnalyzer;
use std::path::Path;

fn main() -> Result<()> {
    println!("🧠 Semantic Analysis Tool\n");

    let mut analyzer = SemanticAnalyzer::new();

    println!("Analyzing src/ directory...");
    analyzer.analyze_directory(Path::new("src"))?;

    let stats = analyzer.stats();
    println!("\n📊 Analysis Results:");
    println!("  Total symbols: {}", stats.total_symbols);
    println!("  Functions: {}", stats.total_functions);
    println!("  Structs: {}", stats.total_structs);
    println!("  Traits: {}", stats.total_traits);
    println!("  Average complexity: {}", stats.avg_complexity);
    println!("  Pattern instances: {}", stats.patterns_found);

    println!("\n🔍 Detected Patterns:");
    for (pattern, match_info) in analyzer.detect_patterns().iter().take(10) {
        println!("  {} (confidence: {:.1})", pattern, match_info.confidence);
    }

    println!("\n🔗 Symbol Relationships:");
    for (from, to, rel_type) in analyzer.find_relationships().iter().take(10) {
        println!("  {} {} {}", from, rel_type, to);
    }

    Ok(())
}
