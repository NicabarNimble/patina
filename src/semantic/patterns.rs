use super::{PatternMatch, SemanticSymbol};
use std::collections::HashMap;

/// Detect high-level patterns from semantic symbols
pub struct PatternDetector {
    patterns: HashMap<String, Box<dyn PatternMatcher>>,
}

trait PatternMatcher: Send + Sync {
    fn matches(&self, symbol: &SemanticSymbol, context: &[SemanticSymbol]) -> Option<PatternMatch>;
}

impl PatternDetector {
    pub fn new() -> Self {
        let mut patterns: HashMap<String, Box<dyn PatternMatcher>> = HashMap::new();

        // Register pattern matchers
        patterns.insert(
            "dependable-rust".to_string(),
            Box::new(DependableRustMatcher),
        );
        patterns.insert(
            "error-propagation".to_string(),
            Box::new(ErrorPropagationMatcher),
        );
        patterns.insert(
            "modular-interface".to_string(),
            Box::new(ModularInterfaceMatcher),
        );

        Self { patterns }
    }

    pub fn detect_patterns(&self, symbols: &[SemanticSymbol]) -> Vec<(String, PatternMatch)> {
        let mut results = Vec::new();

        for symbol in symbols {
            for (pattern_name, matcher) in &self.patterns {
                if let Some(pattern_match) = matcher.matches(symbol, symbols) {
                    results.push((pattern_name.clone(), pattern_match));
                }
            }
        }

        results
    }
}

/// Detects dependable-rust pattern (small, focused functions)
struct DependableRustMatcher;

impl PatternMatcher for DependableRustMatcher {
    fn matches(
        &self,
        symbol: &SemanticSymbol,
        _context: &[SemanticSymbol],
    ) -> Option<PatternMatch> {
        if symbol.kind != super::SymbolKind::Function {
            return None;
        }

        // Check if function follows dependable-rust principles
        if symbol.complexity <= 10 {
            // Low complexity
            Some(PatternMatch {
                pattern: "dependable-rust".to_string(),
                confidence: if symbol.complexity <= 5 { 1.0 } else { 0.7 },
                evidence: format!("Low complexity: {}", symbol.complexity),
            })
        } else {
            None
        }
    }
}

/// Detects error propagation patterns
struct ErrorPropagationMatcher;

impl PatternMatcher for ErrorPropagationMatcher {
    fn matches(
        &self,
        symbol: &SemanticSymbol,
        _context: &[SemanticSymbol],
    ) -> Option<PatternMatch> {
        // Check if symbol has error handling patterns
        let has_result = symbol.patterns.iter().any(|p| p.pattern == "result-return");
        let has_context = symbol.patterns.iter().any(|p| p.pattern == "error-context");

        if has_result && has_context {
            Some(PatternMatch {
                pattern: "error-propagation".to_string(),
                confidence: 0.9,
                evidence: "Uses Result with context".to_string(),
            })
        } else if has_result {
            Some(PatternMatch {
                pattern: "error-propagation".to_string(),
                confidence: 0.6,
                evidence: "Uses Result type".to_string(),
            })
        } else {
            None
        }
    }
}

/// Detects modular interface patterns
struct ModularInterfaceMatcher;

impl PatternMatcher for ModularInterfaceMatcher {
    fn matches(&self, symbol: &SemanticSymbol, context: &[SemanticSymbol]) -> Option<PatternMatch> {
        if symbol.kind != super::SymbolKind::Trait {
            return None;
        }

        // Check if there are implementations of this trait
        let impl_count = context
            .iter()
            .filter(|s| s.kind == super::SymbolKind::Impl && s.name.contains(&symbol.name))
            .count();

        if impl_count > 0 {
            Some(PatternMatch {
                pattern: "modular-interface".to_string(),
                confidence: if impl_count > 2 { 0.9 } else { 0.6 },
                evidence: format!("{} implementations found", impl_count),
            })
        } else {
            None
        }
    }
}

/// Analyze relationships between symbols
pub fn analyze_relationships(symbols: &[SemanticSymbol]) -> Vec<(String, String, String)> {
    let mut relationships = Vec::new();

    // Find trait implementations
    for symbol in symbols {
        if symbol.kind == super::SymbolKind::Impl {
            // This is a simple heuristic - real implementation would parse the impl block
            if symbol.name != "impl" {
                relationships.push((
                    symbol.file.clone(),
                    symbol.name.clone(),
                    "implements".to_string(),
                ));
            }
        }
    }

    // Find module relationships
    for symbol in symbols {
        if symbol.kind == super::SymbolKind::Module {
            // Track module dependencies
            for dep in &symbol.dependencies {
                relationships.push((symbol.name.clone(), dep.clone(), "depends_on".to_string()));
            }
        }
    }

    relationships
}
