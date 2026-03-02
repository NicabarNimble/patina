//! Utility functions for assay command

use anyhow::Result;

/// Truncate string for display
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Collect rows from a query, logging deserialization failures.
/// Returns (successful_rows, failure_count).
pub fn collect_rows<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> (Vec<T>, usize) {
    let mut ok = Vec::new();
    let mut failures = 0usize;
    for r in rows {
        match r {
            Ok(v) => ok.push(v),
            Err(e) => {
                tracing::warn!(error = %e, "row deserialization failed");
                failures += 1;
            }
        }
    }
    (ok, failures)
}

/// Serialize JSON result, injecting `_warnings` if any rows failed.
/// For objects, adds the field directly. For arrays, wraps in `{items, _warnings}`.
pub fn serialize_result(value: serde_json::Value, failures: usize) -> Result<String> {
    if failures == 0 {
        return Ok(serde_json::to_string_pretty(&value)?);
    }
    let warnings = serde_json::json!([format!("{} rows failed deserialization", failures)]);
    match value {
        serde_json::Value::Object(mut map) => {
            map.insert("_warnings".to_string(), warnings);
            Ok(serde_json::to_string_pretty(&serde_json::Value::Object(
                map,
            ))?)
        }
        other => {
            let wrapped = serde_json::json!({"items": other, "_warnings": warnings});
            Ok(serde_json::to_string_pretty(&wrapped)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a very long string", 10), "a very ..."); // 7 chars + "..."
    }
}
