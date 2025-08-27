use anyhow::Result;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::extraction::{SemanticData, FunctionInfo, TypeInfo, ImportInfo, Visibility, TypeKind};

/// Database record that can be converted to SQL
#[derive(Debug, Clone)]
pub struct DatabaseRecord {
    pub table: String,
    pub values: HashMap<String, SqlValue>,
    pub fingerprint: Option<[u8; 16]>,
}

/// SQL value types
#[derive(Debug, Clone)]
pub enum SqlValue {
    Text(String),
    Integer(i64),
    Real(f64),
    Blob(Vec<u8>),
    Null,
}

/// Transform semantic data into database records
pub fn to_records(semantic_data: Vec<SemanticData>) -> Result<Vec<DatabaseRecord>> {
    let mut records = Vec::new();
    
    for data in semantic_data {
        // Transform functions
        for func in data.functions {
            records.push(function_to_record(&data.file_path, &func)?);
        }
        
        // Transform types
        for type_info in data.types {
            records.push(type_to_record(&data.file_path, &type_info)?);
        }
        
        // Transform imports
        for import in data.imports {
            records.push(import_to_record(&data.file_path, &import)?);
        }
        
        // Transform call graph entries
        for call in data.calls {
            let mut values = HashMap::new();
            values.insert("file_path".to_string(), SqlValue::Text(data.file_path.clone()));
            values.insert("caller".to_string(), SqlValue::Text(call.caller));
            values.insert("callee".to_string(), SqlValue::Text(call.callee));
            values.insert("line_number".to_string(), SqlValue::Integer(call.line_number as i64));
            values.insert("is_external".to_string(), SqlValue::Integer(if call.is_external { 1 } else { 0 }));
            
            records.push(DatabaseRecord {
                table: "call_graph".to_string(),
                values,
                fingerprint: None,
            });
        }
        
        // Transform documentation
        for doc in data.docs {
            let mut values = HashMap::new();
            values.insert("file_path".to_string(), SqlValue::Text(data.file_path.clone()));
            values.insert("kind".to_string(), SqlValue::Text(format!("{:?}", doc.kind)));
            values.insert("content".to_string(), SqlValue::Text(doc.content));
            values.insert("line_start".to_string(), SqlValue::Integer(doc.line_start as i64));
            values.insert("line_end".to_string(), SqlValue::Integer(doc.line_end as i64));
            
            records.push(DatabaseRecord {
                table: "documentation".to_string(),
                values,
                fingerprint: None,
            });
        }
    }
    
    Ok(records)
}

/// Transform function to database record
fn function_to_record(file_path: &str, func: &FunctionInfo) -> Result<DatabaseRecord> {
    let mut values = HashMap::new();
    
    values.insert("file_path".to_string(), SqlValue::Text(file_path.to_string()));
    values.insert("name".to_string(), SqlValue::Text(func.name.clone()));
    values.insert("visibility".to_string(), SqlValue::Text(visibility_to_string(&func.visibility)));
    values.insert("is_async".to_string(), SqlValue::Integer(if func.is_async { 1 } else { 0 }));
    values.insert("is_unsafe".to_string(), SqlValue::Integer(if func.is_unsafe { 1 } else { 0 }));
    values.insert("line_start".to_string(), SqlValue::Integer(func.line_start as i64));
    values.insert("line_end".to_string(), SqlValue::Integer(func.line_end as i64));
    
    // Handle optional return type
    if let Some(ref return_type) = func.return_type {
        values.insert("return_type".to_string(), SqlValue::Text(return_type.clone()));
    } else {
        values.insert("return_type".to_string(), SqlValue::Null);
    }
    
    // Handle parameters as JSON string for now
    let params_json = serialize_parameters(&func.parameters);
    values.insert("parameters".to_string(), SqlValue::Text(params_json));
    
    // Handle doc comment
    if let Some(ref doc) = func.doc_comment {
        values.insert("doc_comment".to_string(), SqlValue::Text(doc.clone()));
    } else {
        values.insert("doc_comment".to_string(), SqlValue::Null);
    }
    
    // Calculate fingerprint
    let fingerprint = calculate_function_fingerprint(func);
    
    Ok(DatabaseRecord {
        table: "function_facts".to_string(),
        values,
        fingerprint: Some(fingerprint),
    })
}

/// Transform type to database record
fn type_to_record(file_path: &str, type_info: &TypeInfo) -> Result<DatabaseRecord> {
    let mut values = HashMap::new();
    
    values.insert("file_path".to_string(), SqlValue::Text(file_path.to_string()));
    values.insert("name".to_string(), SqlValue::Text(type_info.name.clone()));
    values.insert("kind".to_string(), SqlValue::Text(type_kind_to_string(&type_info.kind)));
    values.insert("visibility".to_string(), SqlValue::Text(visibility_to_string(&type_info.visibility)));
    values.insert("line_start".to_string(), SqlValue::Integer(type_info.line_start as i64));
    values.insert("line_end".to_string(), SqlValue::Integer(type_info.line_end as i64));
    
    // Handle fields as JSON string
    let fields_json = serialize_fields(&type_info.fields);
    values.insert("fields".to_string(), SqlValue::Text(fields_json));
    
    // Handle generics as comma-separated string
    if !type_info.generics.is_empty() {
        values.insert("generics".to_string(), SqlValue::Text(type_info.generics.join(", ")));
    } else {
        values.insert("generics".to_string(), SqlValue::Null);
    }
    
    // Handle doc comment
    if let Some(ref doc) = type_info.doc_comment {
        values.insert("doc_comment".to_string(), SqlValue::Text(doc.clone()));
    } else {
        values.insert("doc_comment".to_string(), SqlValue::Null);
    }
    
    // Calculate fingerprint
    let fingerprint = calculate_type_fingerprint(type_info);
    
    Ok(DatabaseRecord {
        table: "type_facts".to_string(),
        values,
        fingerprint: Some(fingerprint),
    })
}

/// Transform import to database record
fn import_to_record(file_path: &str, import: &ImportInfo) -> Result<DatabaseRecord> {
    let mut values = HashMap::new();
    
    values.insert("file_path".to_string(), SqlValue::Text(file_path.to_string()));
    values.insert("module".to_string(), SqlValue::Text(import.module.clone()));
    values.insert("is_wildcard".to_string(), SqlValue::Integer(if import.is_wildcard { 1 } else { 0 }));
    values.insert("line_number".to_string(), SqlValue::Integer(import.line_number as i64));
    
    // Handle items as comma-separated string
    if !import.items.is_empty() {
        values.insert("items".to_string(), SqlValue::Text(import.items.join(", ")));
    } else {
        values.insert("items".to_string(), SqlValue::Null);
    }
    
    Ok(DatabaseRecord {
        table: "import_facts".to_string(),
        values,
        fingerprint: None,
    })
}

/// Calculate fingerprint for a function
fn calculate_function_fingerprint(func: &FunctionInfo) -> [u8; 16] {
    let mut hasher = DefaultHasher::new();
    
    // Hash structural elements
    func.name.hash(&mut hasher);
    func.parameters.len().hash(&mut hasher);
    func.is_async.hash(&mut hasher);
    func.is_unsafe.hash(&mut hasher);
    func.return_type.hash(&mut hasher);
    
    // Calculate complexity estimate
    let complexity = estimate_function_complexity(func);
    complexity.hash(&mut hasher);
    
    let hash = hasher.finish();
    
    // Convert to 16-byte array
    let mut fingerprint = [0u8; 16];
    fingerprint[0..8].copy_from_slice(&hash.to_le_bytes());
    fingerprint[8..10].copy_from_slice(&(complexity as u16).to_le_bytes());
    // Leave rest as zeros for future use
    
    fingerprint
}

/// Calculate fingerprint for a type
fn calculate_type_fingerprint(type_info: &TypeInfo) -> [u8; 16] {
    let mut hasher = DefaultHasher::new();
    
    // Hash structural elements
    type_info.name.hash(&mut hasher);
    type_info.kind.hash(&mut hasher);
    type_info.fields.len().hash(&mut hasher);
    type_info.generics.len().hash(&mut hasher);
    
    let hash = hasher.finish();
    
    // Convert to 16-byte array
    let mut fingerprint = [0u8; 16];
    fingerprint[0..8].copy_from_slice(&hash.to_le_bytes());
    fingerprint[8..10].copy_from_slice(&(type_info.fields.len() as u16).to_le_bytes());
    
    fingerprint
}

/// Estimate function complexity (simplified)
fn estimate_function_complexity(func: &FunctionInfo) -> usize {
    let mut complexity = 1; // Base complexity
    
    // Add complexity for parameters
    complexity += func.parameters.len();
    
    // Add complexity for async/unsafe
    if func.is_async { complexity += 2; }
    if func.is_unsafe { complexity += 2; }
    
    // Estimate based on line count
    let line_count = func.line_end - func.line_start;
    complexity += (line_count / 10) as usize;
    
    complexity
}

/// Convert visibility to string
fn visibility_to_string(vis: &Visibility) -> String {
    match vis {
        Visibility::Public => "public",
        Visibility::Private => "private",
        Visibility::Protected => "protected",
        Visibility::Internal => "internal",
    }.to_string()
}

/// Convert type kind to string
fn type_kind_to_string(kind: &TypeKind) -> String {
    match kind {
        TypeKind::Struct => "struct",
        TypeKind::Enum => "enum",
        TypeKind::Interface => "interface",
        TypeKind::Class => "class",
        TypeKind::TypeAlias => "type_alias",
        TypeKind::Trait => "trait",
    }.to_string()
}

/// Serialize parameters to JSON string
fn serialize_parameters(params: &[super::extraction::Parameter]) -> String {
    if params.is_empty() {
        return "[]".to_string();
    }
    
    let param_strings: Vec<String> = params.iter().map(|p| {
        format!(r#"{{"name":"{}","type":{}}}"#,
            p.name,
            p.type_annotation.as_ref()
                .map(|t| format!(r#""{}""#, t))
                .unwrap_or_else(|| "null".to_string())
        )
    }).collect();
    
    format!("[{}]", param_strings.join(","))
}

/// Serialize fields to JSON string
fn serialize_fields(fields: &[super::extraction::Field]) -> String {
    if fields.is_empty() {
        return "[]".to_string();
    }
    
    let field_strings: Vec<String> = fields.iter().map(|f| {
        format!(r#"{{"name":"{}","type":{},"visibility":"{}"}}"#,
            f.name,
            f.type_annotation.as_ref()
                .map(|t| format!(r#""{}""#, t))
                .unwrap_or_else(|| "null".to_string()),
            visibility_to_string(&f.visibility)
        )
    }).collect();
    
    format!("[{}]", field_strings.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrape::extraction::{FunctionInfo, Parameter, TypeInfo, Field};
    use crate::scrape::discovery::Language;
    
    #[test]
    fn test_function_to_record() {
        let func = FunctionInfo {
            name: "test_func".to_string(),
            visibility: Visibility::Public,
            parameters: vec![
                Parameter {
                    name: "arg1".to_string(),
                    type_annotation: Some("String".to_string()),
                    default_value: None,
                },
            ],
            return_type: Some("Result<()>".to_string()),
            is_async: false,
            is_unsafe: false,
            line_start: 10,
            line_end: 15,
            doc_comment: Some("Test function".to_string()),
        };
        
        let record = function_to_record("test.rs", &func).unwrap();
        
        assert_eq!(record.table, "function_facts");
        assert!(record.fingerprint.is_some());
        assert_eq!(
            record.values.get("name").unwrap(),
            &SqlValue::Text("test_func".to_string())
        );
    }
    
    #[test]
    fn test_type_to_record() {
        let type_info = TypeInfo {
            name: "TestStruct".to_string(),
            kind: TypeKind::Struct,
            visibility: Visibility::Public,
            fields: vec![
                Field {
                    name: "field1".to_string(),
                    type_annotation: Some("u32".to_string()),
                    visibility: Visibility::Private,
                    doc_comment: None,
                },
            ],
            generics: vec!["T".to_string()],
            line_start: 5,
            line_end: 10,
            doc_comment: None,
        };
        
        let record = type_to_record("test.rs", &type_info).unwrap();
        
        assert_eq!(record.table, "type_facts");
        assert!(record.fingerprint.is_some());
        assert_eq!(
            record.values.get("kind").unwrap(),
            &SqlValue::Text("struct".to_string())
        );
    }
    
    #[test]
    fn test_fingerprint_generation() {
        let func = FunctionInfo {
            name: "test".to_string(),
            visibility: Visibility::Public,
            parameters: vec![],
            return_type: None,
            is_async: true,
            is_unsafe: false,
            line_start: 1,
            line_end: 5,
            doc_comment: None,
        };
        
        let fingerprint = calculate_function_fingerprint(&func);
        assert_eq!(fingerprint.len(), 16);
        
        // Different function should have different fingerprint
        let mut func2 = func.clone();
        func2.name = "different".to_string();
        let fingerprint2 = calculate_function_fingerprint(&func2);
        assert_ne!(fingerprint, fingerprint2);
    }
}