//! Conventional commit parsing for git scrape.
//!
//! Extracts structured information from commit messages that follow
//! conventional commit format: `type(scope): description (#PR)`
//!
//! This is Phase 1 of the forge abstraction - local regex parsing,
//! no network calls, immediate value for discovering PR references.

use regex::Regex;
use std::sync::OnceLock;

/// Compiled regex for conventional commit format.
///
/// Matches: `type(scope)!: description (#1234)`
/// Groups: type, scope (optional), breaking (optional), desc, pr (optional)
fn conventional_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(?P<type>\w+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?: (?P<desc>.+?)(?:\s*\(#(?P<pr>\d+)\))?$"
        ).expect("Invalid conventional commit regex")
    })
}

/// Compiled regex for issue references in commit body.
///
/// Matches: `Fixes #123`, `Closes #456`, `Resolves #789`
fn issue_ref_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:fix(?:es)?|close[sd]?|resolve[sd]?)[:\s]+#?(\d+)")
            .expect("Invalid issue ref regex")
    })
}

/// Parsed conventional commit information.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedCommit {
    /// Commit type: feat, fix, docs, style, refactor, test, chore, etc.
    pub commit_type: Option<String>,
    /// Scope in parentheses: (sozo), (cli), (core)
    pub scope: Option<String>,
    /// Whether this is a breaking change (! marker)
    pub breaking: bool,
    /// PR number from (#1234) suffix
    pub pr_ref: Option<i64>,
    /// Issue references from body (Fixes #123, Closes #456)
    pub issue_refs: Vec<i64>,
}

impl ParsedCommit {
    /// Check if this commit has any conventional structure.
    pub fn has_structure(&self) -> bool {
        self.commit_type.is_some() || self.pr_ref.is_some() || !self.issue_refs.is_empty()
    }
}

/// Parse a commit message for conventional commit structure.
///
/// Extracts type, scope, breaking flag, PR reference from first line,
/// and issue references from the entire message body.
///
/// # Examples
///
/// ```
/// use patina::commands::scrape::git::commits::parse_conventional;
///
/// let parsed = parse_conventional("feat(sozo): add invoke command (#3384)");
/// assert_eq!(parsed.commit_type, Some("feat".to_string()));
/// assert_eq!(parsed.scope, Some("sozo".to_string()));
/// assert_eq!(parsed.pr_ref, Some(3384));
/// ```
pub fn parse_conventional(message: &str) -> ParsedCommit {
    let first_line = message.lines().next().unwrap_or("");

    // Parse conventional commit format from first line
    let (commit_type, scope, breaking, pr_ref) = conventional_regex()
        .captures(first_line)
        .map(|c| {
            (
                c.name("type").map(|m| m.as_str().to_string()),
                c.name("scope").map(|m| m.as_str().to_string()),
                c.name("breaking").is_some(),
                c.name("pr").and_then(|m| m.as_str().parse().ok()),
            )
        })
        .unwrap_or((None, None, false, None));

    // Extract issue references from entire message
    let issue_refs: Vec<i64> = issue_ref_regex()
        .captures_iter(message)
        .filter_map(|c| c.get(1)?.as_str().parse().ok())
        .collect();

    ParsedCommit {
        commit_type,
        scope,
        breaking,
        pr_ref,
        issue_refs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_conventional_commit() {
        let parsed = parse_conventional("feat(sozo): add invoke command (#3384)");
        assert_eq!(parsed.commit_type, Some("feat".to_string()));
        assert_eq!(parsed.scope, Some("sozo".to_string()));
        assert!(!parsed.breaking);
        assert_eq!(parsed.pr_ref, Some(3384));
        assert!(parsed.issue_refs.is_empty());
    }

    #[test]
    fn test_breaking_change() {
        let parsed = parse_conventional("feat(api)!: remove deprecated endpoint");
        assert_eq!(parsed.commit_type, Some("feat".to_string()));
        assert_eq!(parsed.scope, Some("api".to_string()));
        assert!(parsed.breaking);
        assert_eq!(parsed.pr_ref, None);
    }

    #[test]
    fn test_no_scope() {
        let parsed = parse_conventional("fix: correct typo in readme (#123)");
        assert_eq!(parsed.commit_type, Some("fix".to_string()));
        assert_eq!(parsed.scope, None);
        assert_eq!(parsed.pr_ref, Some(123));
    }

    #[test]
    fn test_issue_refs_in_body() {
        let msg = "feat(auth): add login flow (#100)\n\nFixes #42\nCloses #43";
        let parsed = parse_conventional(msg);
        assert_eq!(parsed.commit_type, Some("feat".to_string()));
        assert_eq!(parsed.pr_ref, Some(100));
        assert_eq!(parsed.issue_refs, vec![42, 43]);
    }

    #[test]
    fn test_issue_refs_various_formats() {
        let msg = "chore: cleanup\n\nFixes: #1\nfixes #2\nCloses #3\nclosed #4\nResolves #5\nresolved #6";
        let parsed = parse_conventional(msg);
        assert_eq!(parsed.issue_refs, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_non_conventional_commit() {
        let parsed = parse_conventional("Updated the README file");
        assert_eq!(parsed.commit_type, None);
        assert_eq!(parsed.scope, None);
        assert_eq!(parsed.pr_ref, None);
        assert!(!parsed.has_structure());
    }

    #[test]
    fn test_pr_only_no_type() {
        // Some repos use "Description (#123)" without conventional prefix
        let parsed = parse_conventional("Add new feature (#456)");
        // Won't match conventional format, but we could extend later
        assert_eq!(parsed.commit_type, None);
        assert_eq!(parsed.pr_ref, None); // Doesn't match conventional format
    }

    #[test]
    fn test_merge_commit() {
        let parsed = parse_conventional("Merge pull request #789 from branch");
        assert_eq!(parsed.commit_type, None);
        // Could extend to detect merge commits in future
    }

    #[test]
    fn test_empty_message() {
        let parsed = parse_conventional("");
        assert!(!parsed.has_structure());
    }

    #[test]
    fn test_complex_scope() {
        let parsed = parse_conventional("feat(cli/commands): add new subcommand (#100)");
        assert_eq!(parsed.commit_type, Some("feat".to_string()));
        assert_eq!(parsed.scope, Some("cli/commands".to_string()));
        assert_eq!(parsed.pr_ref, Some(100));
    }
}
