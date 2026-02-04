//! Parse `## Verification` sections from belief markdown files.

use crate::commands::scrape::beliefs::verification::VerificationQuery;

/// Parse `## Verification` section from belief markdown content.
///
/// Extracts fenced code blocks with `verify` info-string:
/// ```text
/// ```verify type="sql" label="No async functions" expect="= 0"
/// SELECT COUNT(*) FROM function_facts WHERE is_async = 1
/// ```
/// ```
pub fn parse_verification_blocks(content: &str) -> Vec<VerificationQuery> {
    let mut queries = Vec::new();
    let mut in_verification_section = false;
    let mut in_verify_block = false;
    let mut current_attrs: Option<(String, String, String)> = None; // (type, label, expect)
    let mut current_body = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Track which section we're in
        if trimmed.starts_with("## ") {
            in_verification_section = trimmed.starts_with("## Verification");
            continue;
        }

        if !in_verification_section {
            continue;
        }

        if in_verify_block {
            // End of fenced block
            if trimmed.starts_with("```") {
                if let Some((query_type, label, expect)) = current_attrs.take() {
                    let query_text = current_body.trim().to_string();
                    if !query_text.is_empty() {
                        queries.push(VerificationQuery {
                            query_type,
                            label,
                            expect,
                            query_text,
                        });
                    }
                }
                in_verify_block = false;
                current_body.clear();
            } else {
                current_body.push_str(line);
                current_body.push('\n');
            }
        } else if let Some(info_string) = trimmed.strip_prefix("```verify") {
            // Start of verify block — parse attributes from info-string
            if let Some(attrs) = parse_verify_attributes(info_string) {
                current_attrs = Some(attrs);
                in_verify_block = true;
                current_body.clear();
            }
        }
    }

    queries
}

/// Parse key="value" attributes from a verify info-string.
///
/// Input: ` type="sql" label="No async functions" expect="= 0"`
/// Returns: (type, label, expect) or None if required attributes are missing.
fn parse_verify_attributes(info: &str) -> Option<(String, String, String)> {
    let mut query_type = None;
    let mut label = None;
    let mut expect = None;

    // Simple state machine: find key="value" pairs
    let mut chars = info.chars().peekable();
    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Read key
        let key: String = chars
            .by_ref()
            .take_while(|&c| c != '=')
            .collect::<String>()
            .trim()
            .to_string();

        if key.is_empty() {
            break;
        }

        // Expect opening quote
        if chars.peek() != Some(&'"') {
            // Skip non-quoted values
            while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                chars.next();
            }
            continue;
        }
        chars.next(); // consume opening "

        // Read value until closing quote
        let value: String = chars.by_ref().take_while(|&c| c != '"').collect();

        match key.as_str() {
            "type" => query_type = Some(value),
            "label" => label = Some(value),
            "expect" => expect = Some(value),
            _ => {} // ignore unknown attributes
        }
    }

    match (query_type, label, expect) {
        (Some(t), Some(l), Some(e)) => Some((t, l, e)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_verification_blocks() {
        let content = r#"---
type: belief
id: test-belief
---

# test-belief

Some statement.

## Verification

```verify type="sql" label="No async functions" expect="= 0"
SELECT COUNT(*) FROM function_facts WHERE is_async = 1
```

```verify type="sql" label="No tokio imports" expect="= 0"
SELECT COUNT(*) FROM import_facts WHERE import_path LIKE '%tokio%'
```

## Evidence

- Some evidence here
"#;

        let queries = parse_verification_blocks(content);
        assert_eq!(queries.len(), 2);

        assert_eq!(queries[0].query_type, "sql");
        assert_eq!(queries[0].label, "No async functions");
        assert_eq!(queries[0].expect, "= 0");
        assert!(queries[0]
            .query_text
            .contains("SELECT COUNT(*) FROM function_facts"));

        assert_eq!(queries[1].label, "No tokio imports");
    }

    #[test]
    fn test_parse_empty_verification() {
        let content = "## Statement\nSome belief.\n## Evidence\n- something\n";
        let queries = parse_verification_blocks(content);
        assert!(queries.is_empty());
    }

    #[test]
    fn test_parse_verification_stops_at_next_section() {
        let content = r#"## Verification

```verify type="sql" label="Test" expect="= 0"
SELECT 1
```

## Evidence

```verify type="sql" label="Should not be parsed" expect="= 0"
SELECT 2
```
"#;

        let queries = parse_verification_blocks(content);
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].label, "Test");
    }

    #[test]
    fn test_parse_verify_attributes() {
        let attrs = parse_verify_attributes(r#" type="sql" label="Test query" expect=">= 5""#);
        assert!(attrs.is_some());
        let (t, l, e) = attrs.unwrap();
        assert_eq!(t, "sql");
        assert_eq!(l, "Test query");
        assert_eq!(e, ">= 5");
    }

    #[test]
    fn test_parse_verify_attributes_missing_field() {
        let attrs = parse_verify_attributes(r#" type="sql" label="Test""#);
        assert!(attrs.is_none()); // missing expect
    }

    #[test]
    fn test_parse_verification_blocks_mixed_types() {
        let content = r#"## Verification

```verify type="sql" label="caller count" expect=">= 20"
SELECT COUNT(*) FROM call_graph WHERE callee LIKE '%insert_event%'
```

```verify type="assay" label="callers across files" expect=">= 5"
callers --pattern "insert_event" | count(distinct file)
```

```verify type="temporal" label="commit frequency" expect=">= 1000"
derive-moments | summary.total_commits
```
"#;

        let queries = parse_verification_blocks(content);
        assert_eq!(queries.len(), 3);
        assert_eq!(queries[0].query_type, "sql");
        assert_eq!(queries[1].query_type, "assay");
        assert_eq!(
            queries[1].query_text.trim(),
            r#"callers --pattern "insert_event" | count(distinct file)"#
        );
        assert_eq!(queries[2].query_type, "temporal");
        assert_eq!(
            queries[2].query_text.trim(),
            "derive-moments | summary.total_commits"
        );
    }
}
