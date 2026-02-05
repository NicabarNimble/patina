//! Type-aware snippet extraction from FusedResult
//!
//! Produces compact representations for scan-then-focus retrieval (D3).
//! Same snippet logic used by both CLI and MCP interfaces.

use super::fusion::FusedResult;

/// Default snippet length for code and other content types
const SNIPPET_MAX_CHARS: usize = 200;

/// Extract a content-type-aware snippet from a fused result.
///
/// Content type is detected from `metadata.event_type` and `doc_id`:
/// - code (code.function, code.struct, etc.): enriched description, truncated
/// - belief (belief.surface): full statement (typically <150 chars)
/// - commit (git.commit): SHA + subject line (already compact)
/// - pattern (pattern.*): title + first sentence
/// - temporal (commit_file, co_change): already compact, pass through
/// - other: truncate to ~200 chars
pub fn snippet(result: &FusedResult) -> String {
    let event_type = result.metadata.event_type.as_deref().unwrap_or("");

    match event_type {
        // Beliefs: already compact, pass through fully
        "belief.surface" => result.content.clone(),

        // Commits: already "sha: subject (author)" format, pass through
        "git.commit" | "git.tag" => result.content.clone(),

        // Temporal/co-change: already compact
        "commit_file" | "co_change" => result.content.clone(),

        // Code: enriched description like "Function `name` in `file`, params: ..."
        // Truncate to snippet length
        t if t.starts_with("code.") => truncate_utf8(&result.content, SNIPPET_MAX_CHARS),

        // Patterns: first meaningful line
        t if t.starts_with("pattern.") => snippet_pattern(&result.content),

        // Sessions: truncate
        t if t.starts_with("session.") => truncate_utf8(&result.content, SNIPPET_MAX_CHARS),

        // Unknown/other: truncate
        _ => truncate_utf8(&result.content, SNIPPET_MAX_CHARS),
    }
}

/// Truncate a string to max_chars on a char boundary (UTF-8 safe).
fn truncate_utf8(s: &str, max_chars: usize) -> String {
    let collapsed = s.replace('\n', " ");
    let trimmed = collapsed.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

/// Extract snippet from pattern content: title + first sentence of purpose.
fn snippet_pattern(content: &str) -> String {
    // Pattern content starts with markdown: "# Title\n\n..."
    // Extract the title and first non-empty line after it
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());

    let title = match lines.next() {
        Some(l) => l.trim_start_matches('#').trim(),
        None => return truncate_utf8(content, SNIPPET_MAX_CHARS),
    };

    // Skip frontmatter-like lines (status, tags, etc.)
    let first_sentence = lines
        .find(|l| {
            let t = l.trim();
            !t.starts_with("**")
                && !t.starts_with("---")
                && !t.starts_with("id:")
                && !t.starts_with("status:")
        })
        .map(|l| l.trim())
        .unwrap_or("");

    if first_sentence.is_empty() {
        title.to_string()
    } else {
        truncate_utf8(
            &format!("{} — {}", title, first_sentence),
            SNIPPET_MAX_CHARS,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retrieval::fusion::{FusedResult, StructuralAnnotations};
    use crate::retrieval::oracle::OracleMetadata;
    use std::collections::HashMap;

    fn make_result(content: &str, event_type: &str, doc_id: &str) -> FusedResult {
        FusedResult {
            doc_id: doc_id.to_string(),
            content: content.to_string(),
            fused_score: 0.5,
            sources: vec![],
            contributions: HashMap::new(),
            metadata: OracleMetadata {
                event_type: Some(event_type.to_string()),
                file_path: None,
                timestamp: None,
                matches: None,
            },
            annotations: StructuralAnnotations::default(),
        }
    }

    #[test]
    fn belief_passes_through() {
        let r = make_result(
            "Use thiserror derive macros for error types (core, errors.md) [evidence: 3/3]",
            "belief.surface",
            "belief:use-thiserror",
        );
        assert_eq!(snippet(&r), r.content);
    }

    #[test]
    fn commit_passes_through() {
        let r = make_result(
            "a1b2c3d: fix(eval): add belief ground truth (Alice)",
            "git.commit",
            "a1b2c3d",
        );
        assert_eq!(snippet(&r), r.content);
    }

    #[test]
    fn code_truncates_long_content() {
        let long = "Function `very_long_name` in `src/deeply/nested/module.rs`, public, async, params: (conn: &Connection, query: &str, options: &QueryOptions, limit: usize, min_score: f32, dimension: Option<String>), returns: Result<Vec<FusedResult>>. Additional description that goes on and on and makes this really long.";
        let r = make_result(long, "code.function", "src/mod.rs::func");
        let s = snippet(&r);
        assert!(s.ends_with("..."));
        assert!(s.chars().count() <= SNIPPET_MAX_CHARS + 3); // +3 for "..."
    }

    #[test]
    fn code_short_no_truncation() {
        let short = "Function `is_empty` in `src/lib.rs`, public, returns: bool";
        let r = make_result(short, "code.function", "src/lib.rs::is_empty");
        assert_eq!(snippet(&r), short);
    }

    #[test]
    fn utf8_safe_truncation() {
        // Emoji and multi-byte chars shouldn't panic
        let content = "🔮 ".repeat(200); // 400 chars, well over limit
        let r = make_result(&content, "code.function", "test");
        let s = snippet(&r);
        assert!(s.ends_with("..."));
        assert!(s.chars().count() <= SNIPPET_MAX_CHARS + 3);
    }

    #[test]
    fn pattern_extracts_title_and_first_line() {
        let content =
            "# Build Recipe\n\n**Status:** active\n\nA tool that captures development patterns.";
        let r = make_result(content, "pattern.surface", "build");
        let s = snippet(&r);
        assert!(s.starts_with("Build Recipe"));
        assert!(s.contains("A tool that captures"));
    }

    #[test]
    fn newlines_collapsed() {
        let content = "Line one\nLine two\nLine three";
        let r = make_result(content, "session.decision", "session:123");
        let s = snippet(&r);
        assert!(!s.contains('\n'));
        assert!(s.contains("Line one Line two"));
    }
}
