use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use super::patterns::{analyze_relationships, PatternDetector};
use super::{extract_symbols, SemanticSymbol};

/// Analyzes a codebase for semantic patterns
pub struct SemanticAnalyzer {
    symbols: Vec<SemanticSymbol>,
    detector: PatternDetector,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            detector: PatternDetector::new(),
        }
    }

    /// Analyze a directory of Rust code
    pub fn analyze_directory(&mut self, path: &Path) -> Result<()> {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    self.analyze_file(path)?;
                }
            }
        }

        Ok(())
    }

    /// Analyze a single Rust file
    pub fn analyze_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path).context("Failed to read file")?;

        let file_path = path.to_str().unwrap_or("");
        let symbols = extract_symbols(&content, file_path)?;

        self.symbols.extend(symbols);
        Ok(())
    }

    /// Get all extracted symbols
    pub fn symbols(&self) -> &[SemanticSymbol] {
        &self.symbols
    }

    /// Detect patterns in the analyzed code
    pub fn detect_patterns(&self) -> Vec<(String, super::PatternMatch)> {
        self.detector.detect_patterns(&self.symbols)
    }

    /// Find relationships between symbols
    pub fn find_relationships(&self) -> Vec<(String, String, String)> {
        analyze_relationships(&self.symbols)
    }

    /// Generate SQL for storing in DuckDB
    pub fn generate_sql(&self) -> String {
        let mut sql = String::new();
        sql.push_str("BEGIN TRANSACTION;\n");

        // Clear existing semantic data
        sql.push_str("DELETE FROM semantic_symbols;\n");
        sql.push_str("DELETE FROM semantic_patterns;\n");
        sql.push_str("DELETE FROM semantic_relationships;\n");

        // Insert symbols
        for symbol in &self.symbols {
            sql.push_str(&format!(
                "INSERT INTO semantic_symbols (file, name, kind, line, complexity) VALUES ('{}', '{}', '{}', {}, {});\n",
                symbol.file,
                symbol.name,
                format!("{:?}", symbol.kind).to_lowercase(),
                symbol.line,
                symbol.complexity
            ));

            // Insert pattern matches for this symbol
            for pattern in &symbol.patterns {
                sql.push_str(&format!(
                    "INSERT INTO semantic_patterns (file, symbol, pattern, confidence, evidence) VALUES ('{}', '{}', '{}', {}, '{}');\n",
                    symbol.file,
                    symbol.name,
                    pattern.pattern,
                    pattern.confidence,
                    pattern.evidence
                ));
            }
        }

        // Insert detected patterns
        for (pattern_name, pattern_match) in self.detect_patterns() {
            sql.push_str(&format!(
                "INSERT INTO semantic_patterns (file, symbol, pattern, confidence, evidence) VALUES ('global', 'global', '{}', {}, '{}');\n",
                pattern_name,
                pattern_match.confidence,
                pattern_match.evidence
            ));
        }

        // Insert relationships
        for (from, to, rel_type) in self.find_relationships() {
            sql.push_str(&format!(
                "INSERT INTO semantic_relationships (from_symbol, to_symbol, relationship_type) VALUES ('{}', '{}', '{}');\n",
                from, to, rel_type
            ));
        }

        sql.push_str("COMMIT;\n");
        sql
    }

    /// Get statistics about the analyzed code
    pub fn stats(&self) -> SemanticStats {
        let total_functions = self
            .symbols
            .iter()
            .filter(|s| s.kind == super::SymbolKind::Function)
            .count();

        let avg_complexity = if total_functions > 0 {
            self.symbols
                .iter()
                .filter(|s| s.kind == super::SymbolKind::Function)
                .map(|s| s.complexity)
                .sum::<usize>()
                / total_functions
        } else {
            0
        };

        let patterns_found = self.symbols.iter().flat_map(|s| &s.patterns).count();

        SemanticStats {
            total_symbols: self.symbols.len(),
            total_functions,
            total_structs: self
                .symbols
                .iter()
                .filter(|s| s.kind == super::SymbolKind::Struct)
                .count(),
            total_traits: self
                .symbols
                .iter()
                .filter(|s| s.kind == super::SymbolKind::Trait)
                .count(),
            avg_complexity,
            patterns_found,
        }
    }
}

#[derive(Debug)]
pub struct SemanticStats {
    pub total_symbols: usize,
    pub total_functions: usize,
    pub total_structs: usize,
    pub total_traits: usize,
    pub avg_complexity: usize,
    pub patterns_found: usize,
}
