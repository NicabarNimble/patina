use anyhow::Result;
use patina::semantic::deep_analyzer::DeepAnalyzer;
use std::fs;
use walkdir::WalkDir;

fn main() -> Result<()> {
    println!("🧠 Deep Semantic Analysis of Patina\n");

    let mut analyzer = DeepAnalyzer::new()?;
    let mut all_functions = Vec::new();
    let mut file_count = 0;

    // Analyze all Rust files in src/
    for entry in WalkDir::new("src")
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let content = fs::read_to_string(path)?;
                let file_path = path.to_str().unwrap_or("");

                if let Ok(functions) = analyzer.analyze_function(&content, file_path) {
                    all_functions.extend(functions);
                    file_count += 1;
                }
            }
        }
    }

    println!(
        "📊 Analyzed {} files, {} functions\n",
        file_count,
        all_functions.len()
    );

    // Generate insights
    let insights = analyzer.generate_insights(&all_functions);

    println!("🔍 Key Findings:\n");

    println!("📏 Function Sizes (Real, not file size!):");
    let avg_size: usize = all_functions
        .iter()
        .map(|f| f.actual_line_count)
        .sum::<usize>()
        / all_functions.len().max(1);
    println!("  Average function size: {} lines", avg_size);
    println!("  Large functions (>50 lines):");
    for (name, size) in insights.large_functions.iter().take(5) {
        println!("    - {}: {} lines", name, size);
    }

    println!("\n🧩 Complexity Analysis:");
    let avg_complexity: usize = all_functions
        .iter()
        .map(|f| f.cognitive_complexity)
        .sum::<usize>()
        / all_functions.len().max(1);
    println!("  Average cognitive complexity: {}", avg_complexity);
    println!("  Most complex functions:");
    for (name, complexity) in insights.complex_functions.iter().take(5) {
        println!("    - {}: complexity {}", name, complexity);
    }

    println!("\n🚪 API Boundaries:");
    println!(
        "  {} public functions that return Result",
        insights.api_boundaries.len()
    );
    for boundary in insights.api_boundaries.iter().take(5) {
        println!("    - {}", boundary);
    }

    println!("\n⚠️  Error Handling:");
    println!(
        "  {} functions add error context",
        insights.error_handlers.len()
    );
    let no_context = all_functions
        .iter()
        .filter(|f| {
            f.patterns.iter().any(|p| {
                matches!(
                    p,
                    patina::semantic::queries::SemanticPattern::ErrorPropagation {
                        adds_context: false,
                        propagates: true,
                        ..
                    }
                )
            })
        })
        .count();
    println!(
        "  {} functions propagate errors WITHOUT context",
        no_context
    );

    println!("\n🧪 Test Coverage:");
    println!(
        "  {:.1}% of functions are tests",
        insights.test_coverage_ratio * 100.0
    );
    let tested_functions = all_functions
        .iter()
        .filter(|f| !f.called_by.is_empty())
        .count();
    println!(
        "  {} functions are called by other functions",
        tested_functions
    );

    println!("\n📞 Call Patterns:");
    let functions_with_calls = all_functions.iter().filter(|f| !f.calls.is_empty()).count();
    println!(
        "  {} functions make calls to other functions",
        functions_with_calls
    );

    let most_called: Vec<_> = all_functions
        .iter()
        .filter(|f| f.calls.len() > 10)
        .map(|f| (f.name.clone(), f.calls.len()))
        .collect();
    if !most_called.is_empty() {
        println!("  Functions making many calls:");
        for (name, count) in most_called.iter().take(5) {
            println!("    - {}: {} calls", name, count);
        }
    }

    println!("\n🎯 Pattern Detection:");
    let mut pattern_counts = std::collections::HashMap::new();
    for func in &all_functions {
        for pattern in &func.patterns {
            let pattern_name = match pattern {
                patina::semantic::queries::SemanticPattern::ErrorPropagation { .. } => {
                    "ErrorPropagation"
                }
                patina::semantic::queries::SemanticPattern::ApiBoundary { .. } => "ApiBoundary",
                patina::semantic::queries::SemanticPattern::StateMachine { .. } => "StateMachine",
                patina::semantic::queries::SemanticPattern::DependencyInjection { .. } => {
                    "DependencyInjection"
                }
                patina::semantic::queries::SemanticPattern::ResourceManagement { .. } => {
                    "ResourceManagement"
                }
            };
            *pattern_counts.entry(pattern_name).or_insert(0) += 1;
        }
    }

    for (pattern, count) in pattern_counts {
        println!("  {}: {} instances", pattern, count);
    }

    Ok(())
}
