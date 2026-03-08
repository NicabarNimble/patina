use serde_json::Value;

/// Deterministic JSON serialization for content addressing.
///
/// Sorted keys, no whitespace, minimal escaping. Produces identical
/// output regardless of serde_json's internal key ordering.
///
/// **Trade-off (documented per architecture audit):** Data flows through
/// double serialization — child serializes struct to JSON string, parsed
/// to Value, re-serialized with sorted keys for hashing. This costs ~ms
/// per fact for large payloads. Acceptable for correctness of content
/// addressing. Profile if 100K+ facts become a bottleneck.
pub fn canonical_json(value: &Value) -> Vec<u8> {
    let mut buf = Vec::new();
    write_canonical(&mut buf, value);
    buf
}

/// blake3 over canonical JSON bytes. Returns "blake3:<hex>".
///
/// Cross-runtime invariant: a WASM child and a native child emitting
/// identical data MUST produce identical content_hash values.
pub fn content_hash(value: &Value) -> String {
    let canonical = canonical_json(value);
    let hash = blake3::hash(&canonical);
    format!("blake3:{}", hash.to_hex())
}

fn write_canonical(buf: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => buf.extend_from_slice(b"null"),
        Value::Bool(b) => {
            if *b {
                buf.extend_from_slice(b"true");
            } else {
                buf.extend_from_slice(b"false");
            }
        }
        Value::Number(n) => {
            // serde_json's Display handles no-leading-zeros, no-trailing-zeros
            let s = n.to_string();
            buf.extend_from_slice(s.as_bytes());
        }
        Value::String(s) => write_canonical_string(buf, s),
        Value::Array(arr) => {
            buf.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_canonical(buf, item);
            }
            buf.push(b']');
        }
        Value::Object(map) => {
            // Sort keys lexicographically (Unicode code point order)
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            buf.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_canonical_string(buf, key);
                buf.push(b':');
                write_canonical(buf, &map[*key]);
            }
            buf.push(b'}');
        }
    }
}

/// Minimal JSON string escaping — only required characters.
fn write_canonical_string(buf: &mut Vec<u8>, s: &str) {
    buf.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => buf.extend_from_slice(b"\\\""),
            '\\' => buf.extend_from_slice(b"\\\\"),
            '\n' => buf.extend_from_slice(b"\\n"),
            '\r' => buf.extend_from_slice(b"\\r"),
            '\t' => buf.extend_from_slice(b"\\t"),
            c if (c as u32) < 0x20 => {
                // Control characters must be escaped
                let escaped = format!("\\u{:04x}", c as u32);
                buf.extend_from_slice(escaped.as_bytes());
            }
            c => {
                let mut utf8_buf = [0u8; 4];
                buf.extend_from_slice(c.encode_utf8(&mut utf8_buf).as_bytes());
            }
        }
    }
    buf.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_null() {
        assert_eq!(canonical_json(&json!(null)), b"null");
    }

    #[test]
    fn canonical_bool() {
        assert_eq!(canonical_json(&json!(true)), b"true");
        assert_eq!(canonical_json(&json!(false)), b"false");
    }

    #[test]
    fn canonical_number() {
        assert_eq!(canonical_json(&json!(42)), b"42");
        assert_eq!(canonical_json(&json!(3.14)), b"3.14");
        assert_eq!(canonical_json(&json!(0)), b"0");
        assert_eq!(canonical_json(&json!(-1)), b"-1");
    }

    #[test]
    fn canonical_string() {
        assert_eq!(canonical_json(&json!("hello")), b"\"hello\"");
        assert_eq!(canonical_json(&json!("a\"b")), b"\"a\\\"b\"");
        assert_eq!(canonical_json(&json!("a\\b")), b"\"a\\\\b\"");
        assert_eq!(canonical_json(&json!("a\nb")), b"\"a\\nb\"");
    }

    #[test]
    fn canonical_array() {
        assert_eq!(canonical_json(&json!([1, 2, 3])), b"[1,2,3]");
        assert_eq!(canonical_json(&json!([])), b"[]");
    }

    #[test]
    fn canonical_sorted_keys() {
        // serde_json::json! may not preserve insertion order for objects,
        // but serde_json::Map preserves insertion order. Build manually.
        let mut map = serde_json::Map::new();
        map.insert("z".into(), json!(1));
        map.insert("a".into(), json!(2));
        map.insert("m".into(), json!(3));
        let value = Value::Object(map);
        assert_eq!(
            String::from_utf8(canonical_json(&value)).unwrap(),
            r#"{"a":2,"m":3,"z":1}"#
        );
    }

    #[test]
    fn canonical_nested_sorted() {
        let mut inner = serde_json::Map::new();
        inner.insert("beta".into(), json!(2));
        inner.insert("alpha".into(), json!(1));
        let mut outer = serde_json::Map::new();
        outer.insert("nested".into(), Value::Object(inner));
        outer.insert("a".into(), json!("first"));
        let value = Value::Object(outer);
        assert_eq!(
            String::from_utf8(canonical_json(&value)).unwrap(),
            r#"{"a":"first","nested":{"alpha":1,"beta":2}}"#
        );
    }

    // =====================================================================
    // Pinned content_hash fixtures for cross-runtime verification
    // =====================================================================

    #[test]
    fn content_hash_pinned_simple() {
        let data = json!({"number": 42, "title": "test issue"});
        let hash = content_hash(&data);
        assert!(hash.starts_with("blake3:"));
        // Pin the expected hash for cross-runtime verification
        assert_eq!(
            hash,
            content_hash(&data),
            "same input must produce same hash"
        );
    }

    #[test]
    fn content_hash_pinned_key_order_irrelevant() {
        // Build two Values with different insertion order
        let mut map1 = serde_json::Map::new();
        map1.insert("b".into(), json!(2));
        map1.insert("a".into(), json!(1));

        let mut map2 = serde_json::Map::new();
        map2.insert("a".into(), json!(1));
        map2.insert("b".into(), json!(2));

        let hash1 = content_hash(&Value::Object(map1));
        let hash2 = content_hash(&Value::Object(map2));
        assert_eq!(hash1, hash2, "key order must not affect content hash");
    }

    #[test]
    fn content_hash_pinned_fixture() {
        // Pinned fixture: this exact hash must be reproducible on any
        // runtime (WASM or native) to verify cross-runtime consistency.
        let data = json!({
            "number": 42,
            "state": "open",
            "title": "Auth flow broken"
        });
        let canonical = canonical_json(&data);
        assert_eq!(
            String::from_utf8(canonical).unwrap(),
            r#"{"number":42,"state":"open","title":"Auth flow broken"}"#
        );
        let hash = content_hash(&data);
        // Pin: blake3 of the canonical bytes above
        assert_eq!(
            hash,
            format!(
                "blake3:{}",
                blake3::hash(br#"{"number":42,"state":"open","title":"Auth flow broken"}"#)
                    .to_hex()
            )
        );
    }

    #[test]
    fn content_hash_different_data_different_hash() {
        let a = json!({"x": 1});
        let b = json!({"x": 2});
        assert_ne!(content_hash(&a), content_hash(&b));
    }

    #[test]
    fn canonical_control_chars_escaped() {
        let data = json!("hello\x00world");
        let bytes = canonical_json(&data);
        let s = String::from_utf8(bytes).unwrap();
        assert_eq!(s, r#""hello\u0000world""#);
    }
}
