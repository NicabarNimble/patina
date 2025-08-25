use crate::semantic::languages::Language;
use std::collections::HashSet;

/// Extracted documentation with metadata
#[derive(Debug, Clone)]
pub struct Documentation {
    pub raw: String,
    pub clean: String,
    pub keywords: Vec<String>,
    pub summary: String,
    pub has_examples: bool,
    pub has_params: bool,
    pub doc_length: usize,
}

/// Extract documentation comment for a node
pub fn extract(
    node: tree_sitter::Node,
    source: &[u8],
    language: Language,
) -> Option<Documentation> {
    // Look for doc comment in previous sibling
    if let Some(prev) = node.prev_sibling() {
        let is_doc = match language {
            Language::Rust => prev.kind() == "line_comment" && {
                prev.utf8_text(source)
                    .unwrap_or("")
                    .trim_start()
                    .starts_with("///")
            },
            Language::Go => prev.kind() == "comment" && {
                prev.utf8_text(source)
                    .unwrap_or("")
                    .trim_start()
                    .starts_with("//")
            },
            Language::Python => {
                // Python docstrings are the first string in the function body
                if node.kind() == "function_definition" || node.kind() == "class_definition" {
                    if let Some(body) = node.child_by_field_name("body") {
                        if let Some(first_stmt) = body.children(&mut body.walk()).nth(1) {
                            if first_stmt.kind() == "expression_statement" {
                                if let Some(string) = first_stmt.child(0) {
                                    return if string.kind() == "string" {
                                        let raw = string.utf8_text(source).unwrap_or("");
                                        let clean = clean_text(raw, language);
                                        let keywords = extract_keywords(&clean);
                                        let summary = extract_summary(&clean);
                                        Some(Documentation {
                                            has_examples: clean.contains("```") || clean.contains(">>>"),
                                            has_params: clean.contains("Args:") || clean.contains("Parameters:") || clean.contains("@param"),
                                            doc_length: clean.len(),
                                            raw: raw.to_string(),
                                            clean,
                                            keywords,
                                            summary,
                                        })
                                    } else {
                                        None
                                    }
                                }
                            }
                        }
                    }
                }
                false
            },
            Language::JavaScript | Language::JavaScriptJSX | 
            Language::TypeScript | Language::TypeScriptTSX => {
                prev.kind() == "comment" && {
                    let text = prev.utf8_text(source).unwrap_or("");
                    text.starts_with("/**") || text.starts_with("//")
                }
            },
            Language::Solidity => prev.kind() == "comment" && {
                let text = prev.utf8_text(source).unwrap_or("");
                text.starts_with("///") || text.starts_with("/**")
            },
            _ => false,
        };
        
        if is_doc {
            let raw = prev.utf8_text(source).unwrap_or("").to_string();
            let clean = clean_text(&raw, language);
            let keywords = extract_keywords(&clean);
            let summary = extract_summary(&clean);
            return Some(Documentation {
                has_examples: clean.contains("```") || clean.contains("Example:"),
                has_params: clean.contains("Parameters:") || clean.contains("@param") || clean.contains("Args:"),
                doc_length: clean.len(),
                raw,
                clean,
                keywords,
                summary,
            });
        }
    }
    
    // For languages other than Python, also check block comments above
    if language != Language::Python {
        // Walk up to find doc comments that might be separated by whitespace
        let mut cursor = node.walk();
        if let Some(parent) = node.parent() {
            for child in parent.children(&mut cursor) {
                if child.end_byte() > node.start_byte() {
                    break;
                }
                if child.kind() == "comment" || child.kind() == "line_comment" || child.kind() == "block_comment" {
                    let text = child.utf8_text(source).unwrap_or("");
                    let is_doc = match language {
                        Language::Rust => text.starts_with("///") || text.starts_with("//!"),
                        _ => text.starts_with("/**") || text.starts_with("///"),
                    };
                    if is_doc {
                        let raw = text.to_string();
                        let clean = clean_text(&raw, language);
                        let keywords = extract_keywords(&clean);
                        let summary = extract_summary(&clean);
                        return Some(Documentation {
                            has_examples: clean.contains("```") || clean.contains("Example:"),
                            has_params: clean.contains("Parameters:") || clean.contains("@param"),
                            doc_length: clean.len(),
                            raw,
                            clean,
                            keywords,
                            summary,
                        });
                    }
                }
            }
        }
    }
    
    None
}

/// Clean doc text by removing comment markers
pub fn clean_text(raw: &str, language: Language) -> String {
    match language {
        Language::Rust => {
            raw.lines()
                .map(|line| {
                    line.trim_start()
                        .strip_prefix("///")
                        .or_else(|| line.strip_prefix("//!"))
                        .unwrap_or(line)
                        .trim()
                })
                .collect::<Vec<_>>()
                .join(" ")
        },
        Language::Go | Language::Solidity => {
            raw.lines()
                .map(|line| {
                    line.trim_start()
                        .strip_prefix("//")
                        .unwrap_or(line)
                        .trim()
                })
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        },
        Language::Python => {
            // Remove triple quotes and clean
            raw.trim()
                .trim_start_matches("\"\"\"")
                .trim_start_matches("'''")
                .trim_end_matches("\"\"\"")
                .trim_end_matches("'''")
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        },
        Language::JavaScript | Language::JavaScriptJSX | 
        Language::TypeScript | Language::TypeScriptTSX => {
            // Handle both /** */ and // comments
            if raw.starts_with("/**") {
                raw.trim_start_matches("/**")
                    .trim_end_matches("*/")
                    .lines()
                    .map(|line| {
                        line.trim()
                            .strip_prefix("*")
                            .unwrap_or(line)
                            .trim()
                    })
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                raw.lines()
                    .map(|line| {
                        line.trim_start()
                            .strip_prefix("//")
                            .unwrap_or(line)
                            .trim()
                    })
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        },
        _ => raw.to_string(),
    }
}

/// Extract keywords from documentation text
pub fn extract_keywords(doc: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "the", "and", "for", "with", "this", "that", "from", "into", "will", "have",
        "has", "can", "should", "must", "may", "might", "could", "would", "been",
        "being", "was", "were", "are", "not", "but", "just", "only", "all", "some",
        "any", "each", "every", "either", "neither", "both", "more", "most", "less",
        "least", "very", "too", "also", "then", "than", "when", "where", "what",
        "which", "who", "how", "why", "because", "since", "while", "after", "before",
        "during", "between", "among", "through", "over", "under", "above", "below",
    ];
    
    let words: HashSet<String> = doc
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 3)
        .filter(|w| !STOP_WORDS.contains(&w.to_lowercase().as_str()))
        .map(|w| w.to_lowercase())
        .collect();
    
    words.into_iter().collect()
}

/// Extract the first sentence as a summary
pub fn extract_summary(doc: &str) -> String {
    // Find the first sentence ending
    let summary = if let Some(pos) = doc.find(". ") {
        &doc[..pos + 1]
    } else if let Some(pos) = doc.find(".\n") {
        &doc[..pos + 1]
    } else if doc.ends_with('.') {
        doc
    } else {
        // If no sentence ending, take first 100 chars or whole doc
        let limit = doc.len().min(100);
        &doc[..limit]
    };
    
    summary.to_string()
}