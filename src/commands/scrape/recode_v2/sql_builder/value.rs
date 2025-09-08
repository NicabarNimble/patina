// ============================================================================
// SQL VALUE TYPES AND ESCAPING
// ============================================================================
//! Type-safe SQL value handling with proper DuckDB escaping.

use std::fmt;

/// Type-safe SQL values with automatic escaping
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Text(String),
    Integer(i64),
    Boolean(bool),
    Null,
    Array(Vec<String>),
}

impl SqlValue {
    /// Convert to DuckDB SQL representation with proper escaping
    pub fn to_sql(&self) -> String {
        match self {
            SqlValue::Text(s) => format!("'{}'", escape_string(s)),
            SqlValue::Integer(n) => n.to_string(),
            SqlValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            SqlValue::Null => "NULL".to_string(),
            SqlValue::Array(items) => {
                if items.is_empty() {
                    "ARRAY[]::VARCHAR[]".to_string()
                } else {
                    let escaped: Vec<String> = items
                        .iter()
                        .map(|s| format!("'{}'", escape_string(s)))
                        .collect();
                    format!("ARRAY[{}]", escaped.join(", "))
                }
            }
        }
    }
}

impl fmt::Display for SqlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_sql())
    }
}

// Conversion traits for ergonomic API
impl From<String> for SqlValue {
    fn from(s: String) -> Self {
        SqlValue::Text(s)
    }
}

impl From<&str> for SqlValue {
    fn from(s: &str) -> Self {
        SqlValue::Text(s.to_string())
    }
}

impl From<i64> for SqlValue {
    fn from(n: i64) -> Self {
        SqlValue::Integer(n)
    }
}

impl From<i32> for SqlValue {
    fn from(n: i32) -> Self {
        SqlValue::Integer(n as i64)
    }
}

impl From<usize> for SqlValue {
    fn from(n: usize) -> Self {
        SqlValue::Integer(n as i64)
    }
}

impl From<bool> for SqlValue {
    fn from(b: bool) -> Self {
        SqlValue::Boolean(b)
    }
}

impl<T: Into<SqlValue>> From<Option<T>> for SqlValue {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(val) => val.into(),
            None => SqlValue::Null,
        }
    }
}

impl From<Vec<String>> for SqlValue {
    fn from(v: Vec<String>) -> Self {
        SqlValue::Array(v)
    }
}

impl From<Vec<&str>> for SqlValue {
    fn from(v: Vec<&str>) -> Self {
        SqlValue::Array(v.into_iter().map(|s| s.to_string()).collect())
    }
}

/// Escape a string for use in DuckDB SQL
/// 
/// DuckDB uses standard SQL escaping:
/// - Single quotes are doubled: ' becomes ''
/// - No backslash escaping needed (unlike MySQL)
pub fn escape_string(s: &str) -> String {
    s.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("it's"), "it''s");
        assert_eq!(escape_string("'quoted'"), "''quoted''");
        assert_eq!(escape_string("back\\slash"), "back\\slash"); // No escaping needed
    }
    
    #[test]
    fn test_sql_value_text() {
        let val = SqlValue::Text("it's a test".to_string());
        assert_eq!(val.to_sql(), "'it''s a test'");
    }
    
    #[test]
    fn test_sql_value_array() {
        let val = SqlValue::Array(vec!["one".to_string(), "it's two".to_string()]);
        assert_eq!(val.to_sql(), "ARRAY['one', 'it''s two']");
        
        let empty = SqlValue::Array(vec![]);
        assert_eq!(empty.to_sql(), "ARRAY[]::VARCHAR[]");
    }
    
    #[test]
    fn test_sql_value_conversions() {
        let text: SqlValue = "test".into();
        assert_eq!(text.to_sql(), "'test'");
        
        let num: SqlValue = 42i64.into();
        assert_eq!(num.to_sql(), "42");
        
        let boolean: SqlValue = true.into();
        assert_eq!(boolean.to_sql(), "TRUE");
        
        let null: SqlValue = None::<String>.into();
        assert_eq!(null.to_sql(), "NULL");
    }
}