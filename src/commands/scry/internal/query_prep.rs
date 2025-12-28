//! Query preparation for FTS5 search
//!
//! Transforms natural language queries into FTS5-compatible search patterns.
//! Extracts technical terms and handles code-like queries appropriately.

use std::collections::HashSet;

/// Prepare query for FTS5 - extract technical terms for better matching
///
/// Strategy:
/// 1. If query looks like code (snake_case, CamelCase, ::), use as-is
/// 2. Otherwise, extract technical terms from natural language
/// 3. Use OR search for multiple terms
pub fn prepare_fts_query(query: &str) -> String {
    let trimmed = query.trim();

    // If it looks like code already, use direct search
    if is_code_like(trimmed) {
        return if trimmed.contains(' ') {
            format!("\"{}\"", trimmed)
        } else {
            trimmed.to_string()
        };
    }

    // Extract technical terms from natural language
    let terms = extract_technical_terms(trimmed);

    if terms.is_empty() {
        // Fallback: use whole query as phrase
        format!("\"{}\"", trimmed)
    } else if terms.len() == 1 {
        terms[0].clone()
    } else {
        // OR search for multiple terms (FTS5 defaults to AND, we want OR)
        terms.join(" OR ")
    }
}

/// Check if query looks like code (not natural language)
pub fn is_code_like(query: &str) -> bool {
    query.contains("::")
        || query.contains("()")
        || query.contains('_') && !query.contains(' ')
        || query.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Extract technical terms from natural language query
pub fn extract_technical_terms(query: &str) -> Vec<String> {
    // Words to filter out (question words, common verbs, articles)
    let stop_words: HashSet<&str> = [
        // Question words
        "how",
        "what",
        "why",
        "when",
        "where",
        "which",
        "who",
        // Common verbs
        "does",
        "do",
        "is",
        "are",
        "was",
        "were",
        "can",
        "could",
        "will",
        "would",
        "work",
        "works",
        "working",
        "handle",
        "handles",
        "handling",
        "perform",
        "performs",
        "performing",
        "combine",
        "combines",
        "combining",
        "coordinate",
        "coordinates",
        "extract",
        "extracts",
        "build",
        "builds",
        "get",
        "gets",
        "set",
        "sets",
        "use",
        "uses",
        "using",
        "create",
        "creates",
        "manage",
        "manages",
        "ensure",
        "ensures",
        "apply",
        "applies",
        // Articles and prepositions
        "the",
        "a",
        "an",
        "for",
        "from",
        "with",
        "to",
        "in",
        "on",
        "of",
        "by",
        // Other common words
        "and",
        "or",
        "but",
        "this",
        "that",
        "these",
        "those",
        "multiple",
        "different",
        "various",
        "specific",
    ]
    .into_iter()
    .collect();

    let mut terms = Vec::new();

    for word in query.split_whitespace() {
        // Clean punctuation
        let cleaned: String = word
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();

        if cleaned.is_empty() {
            continue;
        }

        let lower = cleaned.to_lowercase();

        // Skip stop words
        if stop_words.contains(lower.as_str()) {
            continue;
        }

        // Keep if:
        // 1. Contains underscore (snake_case)
        // 2. Contains uppercase in middle (CamelCase)
        // 3. Is all uppercase (acronym like RRF, MCP, JSON)
        // 4. Is a technical term (length > 2 and not common)
        let is_snake_case = cleaned.contains('_');
        let is_camel_case = cleaned.chars().skip(1).any(|c| c.is_uppercase());
        let is_acronym = cleaned.len() >= 2 && cleaned.chars().all(|c| c.is_uppercase());
        let is_technical = cleaned.len() > 2;

        if is_snake_case || is_camel_case || is_acronym || is_technical {
            // Quote hyphenated terms to prevent FTS5 interpreting - as NOT
            if cleaned.contains('-') {
                terms.push(format!("\"{}\"", cleaned));
            } else {
                terms.push(cleaned);
            }
        }
    }

    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_code_like() {
        // Code-like patterns
        assert!(is_code_like("rrf_fuse"));
        assert!(is_code_like("std::env"));
        assert!(is_code_like("execute()"));
        assert!(is_code_like("QueryEngine"));

        // Natural language
        assert!(!is_code_like("How does RRF work?"));
        assert!(!is_code_like("semantic search"));
    }

    #[test]
    fn test_extract_technical_terms() {
        // Natural language query
        let terms =
            extract_technical_terms("How does RRF fusion combine results from multiple oracles?");
        assert!(terms.contains(&"RRF".to_string()));
        assert!(terms.contains(&"fusion".to_string()));
        assert!(terms.contains(&"results".to_string()));
        assert!(terms.contains(&"oracles".to_string()));
        // Should NOT contain stop words
        assert!(!terms.iter().any(|t| t.to_lowercase() == "how"));
        assert!(!terms.iter().any(|t| t.to_lowercase() == "does"));
        assert!(!terms.iter().any(|t| t.to_lowercase() == "from"));

        // CamelCase preserved
        let terms2 = extract_technical_terms("What is the QueryEngine interface?");
        assert!(terms2.contains(&"QueryEngine".to_string()));

        // Acronyms preserved, hyphenated terms quoted for FTS5
        let terms3 = extract_technical_terms("How does MCP server handle JSON-RPC?");
        assert!(terms3.contains(&"MCP".to_string()));
        assert!(terms3.contains(&"\"JSON-RPC\"".to_string())); // Quoted for FTS5
    }

    #[test]
    fn test_prepare_fts_query() {
        // Code symbols pass through
        assert_eq!(prepare_fts_query("rrf_fuse"), "rrf_fuse");
        assert_eq!(prepare_fts_query("QueryEngine"), "QueryEngine");

        // Natural language extracts terms with OR
        let result = prepare_fts_query("How does RRF fusion work?");
        assert!(result.contains("RRF"));
        assert!(result.contains("fusion"));
        assert!(result.contains(" OR "));
    }
}
