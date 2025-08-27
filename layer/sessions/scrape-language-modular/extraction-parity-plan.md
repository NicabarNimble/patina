# Extraction Parity Implementation Plan

## Goal
Achieve 100% feature parity between old scrape and new modular extractors while maintaining clean layer separation.

## Current Gap Analysis

### What We Have
- ✅ Basic semantic extraction (functions, types, imports, calls)
- ✅ Documentation comments
- ✅ Line numbers and visibility

### What We're Missing
1. **Fingerprints** (AST pattern hash, complexity, feature flags)
2. **Function Facts** (behavioral signals like mut_self, returns_result)
3. **Type Vocabulary** (full type definitions)
4. **Behavioral Hints** (unwrap/panic detection)
5. **Code Context** (surrounding code snippets)
6. **Documentation Analysis** (keywords, summary extraction)

## Implementation Plan

### Phase 1: Enhance Data Structures
**File: `src/scrape/extraction/mod.rs`**

```rust
// LINE 23: Expand FunctionInfo to capture ALL behavioral facts
pub struct FunctionInfo {
    pub name: String,
    pub visibility: Visibility,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub line_start: usize,
    pub line_end: usize,
    pub doc_comment: Option<String>,
    
    // ADD: New fields for function_facts table
    pub signature: String,              // Full function signature
    pub takes_mut_self: bool,          // &mut self parameter
    pub takes_mut_params: bool,        // Any &mut parameters
    pub returns_result: bool,          // Returns Result<...>
    pub returns_option: bool,          // Returns Option<...>
    pub parameter_count: usize,        // Total parameter count
    pub has_self: bool,               // Is a method (has self)
    pub context_snippet: String,       // Surrounding code for search
}

// LINE 38: Expand TypeInfo to capture full definitions
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
    pub visibility: Visibility,
    pub fields: Vec<Field>,
    pub generics: Vec<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub doc_comment: Option<String>,
    
    // ADD: New fields for type_vocabulary table
    pub full_definition: String,       // Complete type definition
    pub signature: String,              // Type signature
    pub context_snippet: String,       // Surrounding code
}

// LINE 69: Expand Documentation to match old schema
pub struct Documentation {
    pub kind: DocKind,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    
    // ADD: New fields for documentation table
    pub raw_content: String,           // Original with markers
    pub summary: String,                // First sentence
    pub keywords: Vec<String>,          // Extracted keywords
    pub has_examples: bool,             // Contains code blocks
}

// LINE 12: Add new structures for missing data
pub struct FunctionFingerprint {
    pub pattern: u32,                  // AST shape hash
    pub imports: u32,                  // Dependency hash
    pub complexity: u16,               // Cyclomatic complexity
    pub flags: u16,                    // Feature flags
}

pub struct BehavioralHints {
    pub function_name: String,
    pub calls_unwrap: usize,          // Count of .unwrap()
    pub calls_expect: usize,          // Count of .expect()
    pub has_panic_macro: bool,        // Contains panic!()
    pub has_todo_macro: bool,         // Contains todo!()
    pub has_unsafe_block: bool,       // Contains unsafe {}
    pub has_mutex: bool,              // Thread synchronization
    pub has_arc: bool,                // Shared ownership
}

// LINE 13: Add to SemanticData
pub struct SemanticData {
    pub file_path: String,
    pub language: Language,
    pub functions: Vec<FunctionInfo>,
    pub types: Vec<TypeInfo>,
    pub imports: Vec<ImportInfo>,
    pub calls: Vec<CallGraph>,
    pub docs: Vec<Documentation>,
    
    // ADD: New fields
    pub fingerprints: Vec<(String, FunctionFingerprint)>,  // name -> fingerprint
    pub behavioral_hints: Vec<BehavioralHints>,
}
```

### Phase 2: Update Rust Extractor
**File: `src/scrape/extraction/rust.rs`**

```rust
// LINE 143: Update extract_function to capture ALL details
fn extract_function(node: &Node, source: &str) -> Option<FunctionInfo> {
    let mut func = FunctionInfo {
        name: String::new(),
        visibility: Visibility::Private,
        parameters: Vec::new(),
        return_type: None,
        is_async: false,
        is_unsafe: false,
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: extract_doc_comment(node, source),
        
        // ADD: Initialize new fields
        signature: String::new(),
        takes_mut_self: false,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        parameter_count: 0,
        has_self: false,
        context_snippet: extract_context(node, source),
    };
    
    // LINE 168: When processing parameters
    "parameters" => {
        let params = extract_parameters(&child, source);
        func.parameter_count = params.len();
        
        // Check for mut self and mut params
        for param in &params {
            if param.name == "self" || param.name == "&self" {
                func.has_self = true;
            }
            if param.name == "&mut self" {
                func.takes_mut_self = true;
            }
            if let Some(ref type_ann) = param.type_annotation {
                if type_ann.contains("&mut ") {
                    func.takes_mut_params = true;
                }
            }
        }
        
        func.parameters = params;
    }
    
    // LINE 172: When processing return type
    "type" => {
        let return_str = child.utf8_text(source.as_bytes()).ok()?.to_string();
        func.returns_result = return_str.contains("Result<");
        func.returns_option = return_str.contains("Option<");
        func.return_type = Some(return_str);
    }
    
    // AFTER LINE 183: Generate full signature
    func.signature = generate_function_signature(&func, node, source);
    
    Some(func)
}

// ADD NEW FUNCTION after extract_function
fn generate_function_signature(func: &FunctionInfo, node: &Node, source: &str) -> String {
    // Build signature like: "pub fn foo(x: i32, y: &str) -> Result<String>"
    let mut sig = String::new();
    
    if func.visibility == Visibility::Public {
        sig.push_str("pub ");
    }
    if func.is_async {
        sig.push_str("async ");
    }
    if func.is_unsafe {
        sig.push_str("unsafe ");
    }
    
    sig.push_str("fn ");
    sig.push_str(&func.name);
    sig.push('(');
    
    let param_strs: Vec<String> = func.parameters.iter().map(|p| {
        if let Some(ref ty) = p.type_annotation {
            format!("{}: {}", p.name, ty)
        } else {
            p.name.clone()
        }
    }).collect();
    sig.push_str(&param_strs.join(", "));
    sig.push(')');
    
    if let Some(ref ret) = func.return_type {
        sig.push_str(" -> ");
        sig.push_str(ret);
    }
    
    sig
}

// LINE 85: Update extract_node to calculate fingerprints
fn extract_node(node: &Node, source: &str, data: &mut SemanticData, context: &mut ParseContext) {
    match node.kind() {
        "function_item" => {
            if let Some(func) = extract_function(node, source) {
                // ADD: Calculate fingerprint
                let fingerprint = calculate_fingerprint(node, source);
                data.fingerprints.push((func.name.clone(), fingerprint));
                
                // ADD: Extract behavioral hints
                let hints = extract_behavioral_hints(&func.name, node, source);
                if has_interesting_behavior(&hints) {
                    data.behavioral_hints.push(hints);
                }
                
                context.enter_function(func.name.clone());
                data.functions.push(func);
            }
        }
        // ... rest of match arms
    }
}

// ADD NEW FUNCTION: Calculate fingerprint
fn calculate_fingerprint(node: &Node, source: &str) -> FunctionFingerprint {
    use crate::semantic::fingerprint::Fingerprint;
    
    let fingerprint = Fingerprint::from_ast(*node, source.as_bytes());
    
    FunctionFingerprint {
        pattern: fingerprint.pattern,
        imports: fingerprint.imports,
        complexity: fingerprint.complexity,
        flags: fingerprint.flags,
    }
}

// ADD NEW FUNCTION: Extract behavioral hints
fn extract_behavioral_hints(name: &str, node: &Node, source: &str) -> BehavioralHints {
    let mut hints = BehavioralHints {
        function_name: name.to_string(),
        calls_unwrap: 0,
        calls_expect: 0,
        has_panic_macro: false,
        has_todo_macro: false,
        has_unsafe_block: false,
        has_mutex: false,
        has_arc: false,
    };
    
    // Walk the function body AST
    let mut cursor = node.walk();
    count_behavioral_patterns(&mut cursor, source, &mut hints);
    
    hints
}

// ADD NEW FUNCTION: Count behavioral patterns
fn count_behavioral_patterns(cursor: &mut tree_sitter::TreeCursor, source: &str, hints: &mut BehavioralHints) {
    let node = cursor.node();
    
    match node.kind() {
        "method_call_expression" => {
            if let Some(method) = node.child_by_field_name("name") {
                if let Ok(method_name) = method.utf8_text(source.as_bytes()) {
                    match method_name {
                        "unwrap" => hints.calls_unwrap += 1,
                        "expect" => hints.calls_expect += 1,
                        _ => {}
                    }
                }
            }
        }
        "macro_invocation" => {
            if let Ok(macro_text) = node.utf8_text(source.as_bytes()) {
                if macro_text.starts_with("panic!") {
                    hints.has_panic_macro = true;
                } else if macro_text.starts_with("todo!") {
                    hints.has_todo_macro = true;
                }
            }
        }
        "unsafe_block" => {
            hints.has_unsafe_block = true;
        }
        "type_identifier" => {
            if let Ok(type_name) = node.utf8_text(source.as_bytes()) {
                if type_name == "Mutex" || type_name.contains("Mutex<") {
                    hints.has_mutex = true;
                }
                if type_name == "Arc" || type_name.contains("Arc<") {
                    hints.has_arc = true;
                }
            }
        }
        _ => {}
    }
    
    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            count_behavioral_patterns(cursor, source, hints);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

// ADD NEW FUNCTION: Check if hints are worth storing
fn has_interesting_behavior(hints: &BehavioralHints) -> bool {
    hints.calls_unwrap > 0 ||
    hints.calls_expect > 0 ||
    hints.has_panic_macro ||
    hints.has_todo_macro ||
    hints.has_unsafe_block ||
    hints.has_mutex ||
    hints.has_arc
}

// ADD NEW FUNCTION: Extract context snippet
fn extract_context(node: &Node, source: &str) -> String {
    // Get 2 lines before and after for context
    let start_line = node.start_position().row.saturating_sub(2);
    let end_line = (node.end_position().row + 3).min(source.lines().count());
    
    source.lines()
        .skip(start_line)
        .take(end_line - start_line)
        .collect::<Vec<_>>()
        .join("\n")
}

// LINE 187: Update extract_struct similarly
fn extract_struct(node: &Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        // ... existing fields ...
        
        // ADD: New fields
        full_definition: node.utf8_text(source.as_bytes()).ok()?.to_string(),
        signature: String::new(),
        context_snippet: extract_context(node, source),
    };
    
    // ... existing extraction code ...
    
    // Generate signature
    type_info.signature = generate_type_signature(&type_info);
    
    Some(type_info)
}

// ADD NEW FUNCTION: Generate type signature
fn generate_type_signature(type_info: &TypeInfo) -> String {
    let mut sig = String::new();
    
    if type_info.visibility == Visibility::Public {
        sig.push_str("pub ");
    }
    
    match type_info.kind {
        TypeKind::Struct => sig.push_str("struct "),
        TypeKind::Enum => sig.push_str("enum "),
        TypeKind::Trait => sig.push_str("trait "),
        _ => {}
    }
    
    sig.push_str(&type_info.name);
    
    if !type_info.generics.is_empty() {
        sig.push('<');
        sig.push_str(&type_info.generics.join(", "));
        sig.push('>');
    }
    
    sig
}

// ADD NEW FUNCTION: Extract documentation keywords
fn extract_keywords(doc: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "the", "and", "for", "with", "this", "that", "from", "into",
        "will", "can", "may", "must", "should", "would", "could",
        "has", "have", "had", "does", "did", "are", "was", "were",
        "been", "being", "get", "set", "new", "all", "some", "any",
    ];
    
    doc.split_whitespace()
        .flat_map(|word| word.split(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 3)
        .map(|w| w.to_lowercase())
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}
```

### Phase 3: Update Transform Layer
**File: `src/scrape/transform.rs`**

```rust
// LINE 27: Update to_records to generate all table records
pub fn to_records(semantic_data: Vec<SemanticData>) -> Result<Vec<DatabaseRecord>> {
    let mut records = Vec::new();
    
    for data in semantic_data {
        // Transform functions -> function_facts + code_fingerprints + code_search
        for func in data.functions {
            // 1. Function facts record
            records.push(function_to_facts_record(&data.file_path, &func)?);
            
            // 2. Code search record
            records.push(function_to_search_record(&data.file_path, &func)?);
            
            // 3. Find and add fingerprint
            if let Some((_, fingerprint)) = data.fingerprints.iter()
                .find(|(name, _)| name == &func.name) {
                records.push(fingerprint_to_record(&data.file_path, &func.name, "function", fingerprint)?);
            }
        }
        
        // Transform types -> type_vocabulary + code_fingerprints + code_search
        for type_info in data.types {
            // 1. Type vocabulary record
            records.push(type_to_vocabulary_record(&data.file_path, &type_info)?);
            
            // 2. Code search record
            records.push(type_to_search_record(&data.file_path, &type_info)?);
            
            // 3. Fingerprint if struct/trait
            if matches!(type_info.kind, TypeKind::Struct | TypeKind::Trait) {
                if let Some((_, fingerprint)) = data.fingerprints.iter()
                    .find(|(name, _)| name == &type_info.name) {
                    records.push(fingerprint_to_record(&data.file_path, &type_info.name,
                        &type_info.kind.to_string(), fingerprint)?);
                }
            }
        }
        
        // Transform imports -> import_facts
        for import in data.imports {
            records.push(import_to_facts_record(&data.file_path, &import)?);
        }
        
        // Transform behavioral hints
        for hint in data.behavioral_hints {
            records.push(behavioral_hint_to_record(&data.file_path, &hint)?);
        }
        
        // Transform documentation
        for doc in data.docs {
            records.push(documentation_to_record(&data.file_path, &doc)?);
        }
        
        // Transform call graph
        for call in data.calls {
            records.push(call_graph_to_record(&data.file_path, &call)?);
        }
    }
    
    Ok(records)
}

// ADD: New transformation functions
fn function_to_facts_record(file_path: &str, func: &FunctionInfo) -> Result<DatabaseRecord> {
    let mut values = HashMap::new();
    values.insert("file".to_string(), SqlValue::Text(file_path.to_string()));
    values.insert("name".to_string(), SqlValue::Text(func.name.clone()));
    values.insert("takes_mut_self".to_string(), SqlValue::Integer(func.takes_mut_self as i64));
    values.insert("takes_mut_params".to_string(), SqlValue::Integer(func.takes_mut_params as i64));
    values.insert("returns_result".to_string(), SqlValue::Integer(func.returns_result as i64));
    values.insert("returns_option".to_string(), SqlValue::Integer(func.returns_option as i64));
    values.insert("is_async".to_string(), SqlValue::Integer(func.is_async as i64));
    values.insert("is_unsafe".to_string(), SqlValue::Integer(func.is_unsafe as i64));
    values.insert("is_public".to_string(), SqlValue::Integer((func.visibility == Visibility::Public) as i64));
    values.insert("parameter_count".to_string(), SqlValue::Integer(func.parameter_count as i64));
    values.insert("line_start".to_string(), SqlValue::Integer(func.line_start as i64));
    values.insert("line_end".to_string(), SqlValue::Integer(func.line_end as i64));
    
    Ok(DatabaseRecord {
        table: "function_facts".to_string(),
        values,
        fingerprint: None,
    })
}

fn function_to_search_record(file_path: &str, func: &FunctionInfo) -> Result<DatabaseRecord> {
    let mut values = HashMap::new();
    values.insert("path".to_string(), SqlValue::Text(file_path.to_string()));
    values.insert("name".to_string(), SqlValue::Text(func.name.clone()));
    values.insert("signature".to_string(), SqlValue::Text(func.signature.clone()));
    values.insert("context".to_string(), SqlValue::Text(func.context_snippet.clone()));
    
    Ok(DatabaseRecord {
        table: "code_search".to_string(),
        values,
        fingerprint: None,
    })
}

fn type_to_vocabulary_record(file_path: &str, type_info: &TypeInfo) -> Result<DatabaseRecord> {
    let mut values = HashMap::new();
    values.insert("file".to_string(), SqlValue::Text(file_path.to_string()));
    values.insert("name".to_string(), SqlValue::Text(type_info.name.clone()));
    values.insert("definition".to_string(), SqlValue::Text(type_info.full_definition.clone()));
    values.insert("kind".to_string(), SqlValue::Text(type_info.kind.to_string()));
    values.insert("visibility".to_string(), SqlValue::Text(
        match type_info.visibility {
            Visibility::Public => "pub",
            Visibility::Private => "private",
            _ => "unknown"
        }.to_string()
    ));
    values.insert("usage_count".to_string(), SqlValue::Integer(0));
    
    Ok(DatabaseRecord {
        table: "type_vocabulary".to_string(),
        values,
        fingerprint: None,
    })
}

fn fingerprint_to_record(file_path: &str, name: &str, kind: &str, 
                         fingerprint: &FunctionFingerprint) -> Result<DatabaseRecord> {
    let mut values = HashMap::new();
    values.insert("path".to_string(), SqlValue::Text(file_path.to_string()));
    values.insert("name".to_string(), SqlValue::Text(name.to_string()));
    values.insert("kind".to_string(), SqlValue::Text(kind.to_string()));
    values.insert("pattern".to_string(), SqlValue::Integer(fingerprint.pattern as i64));
    values.insert("imports".to_string(), SqlValue::Integer(fingerprint.imports as i64));
    values.insert("complexity".to_string(), SqlValue::Integer(fingerprint.complexity as i64));
    values.insert("flags".to_string(), SqlValue::Integer(fingerprint.flags as i64));
    
    Ok(DatabaseRecord {
        table: "code_fingerprints".to_string(),
        values,
        fingerprint: None,
    })
}
```

### Phase 4: Update Go Extractor (Similar Pattern)
**File: `src/scrape/extraction/go.rs`**

Apply the same pattern as Rust:
1. Add new fields to function extraction
2. Calculate fingerprints
3. Extract behavioral hints
4. Generate signatures and context

### Phase 5: Update Storage Layer
**File: `src/scrape/storage.rs`**

```rust
// LINE 90: Update records_to_sql to handle all table types
fn records_to_sql(records: Vec<DatabaseRecord>) -> String {
    let mut sql = String::new();
    
    // Group records by table for batch inserts
    let mut by_table: HashMap<String, Vec<DatabaseRecord>> = HashMap::new();
    for record in records {
        by_table.entry(record.table.clone()).or_default().push(record);
    }
    
    // Generate SQL for each table
    for (table, records) in by_table {
        match table.as_str() {
            "code_fingerprints" => {
                for record in records {
                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', '{}', {}, {}, {}, {});\n",
                        escape_sql(&record.values["path"]),
                        escape_sql(&record.values["name"]),
                        escape_sql(&record.values["kind"]),
                        record.values["pattern"],
                        record.values["imports"],
                        record.values["complexity"],
                        record.values["flags"]
                    ));
                }
            }
            "function_facts" => {
                for record in records {
                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO function_facts (file, name, takes_mut_self, takes_mut_params, returns_result, returns_option, is_async, is_unsafe, is_public, parameter_count, line_start, line_end) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, {});\n",
                        escape_sql(&record.values["file"]),
                        escape_sql(&record.values["name"]),
                        record.values["takes_mut_self"],
                        record.values["takes_mut_params"],
                        record.values["returns_result"],
                        record.values["returns_option"],
                        record.values["is_async"],
                        record.values["is_unsafe"],
                        record.values["is_public"],
                        record.values["parameter_count"],
                        record.values["line_start"],
                        record.values["line_end"]
                    ));
                }
            }
            // ... handle other tables
        }
    }
    
    sql
}
```

## Testing Strategy

### 1. Unit Tests
- Test each extraction function individually
- Verify fingerprint calculation
- Test behavioral hint detection

### 2. Integration Tests
```rust
#[test]
fn test_extraction_parity() {
    // Run old scrape on a test file
    let old_results = run_old_scrape("test.rs");
    
    // Run new extraction on same file
    let new_results = run_new_extraction("test.rs");
    
    // Compare all fields
    assert_eq!(old_results.fingerprints, new_results.fingerprints);
    assert_eq!(old_results.function_facts, new_results.function_facts);
    assert_eq!(old_results.behavioral_hints, new_results.behavioral_hints);
}
```

### 3. Database Comparison
```sql
-- After running both old and new on same codebase
SELECT * FROM old_db.code_fingerprints
EXCEPT
SELECT * FROM new_db.code_fingerprints;
-- Should return empty set
```

## Migration Timeline

1. **Hour 1-2**: Update data structures in extraction/mod.rs
2. **Hour 3-4**: Update Rust extractor with all features
3. **Hour 5-6**: Update Go extractor with all features  
4. **Hour 7**: Update transform layer
5. **Hour 8**: Update storage layer
6. **Hour 9**: Write comprehensive tests
7. **Hour 10**: Run side-by-side comparison

## Success Criteria

- [ ] All 10 database tables populated identically
- [ ] Fingerprints match for same functions
- [ ] Behavioral hints detected accurately
- [ ] Function facts complete and correct
- [ ] Documentation keywords extracted
- [ ] Call graph complete
- [ ] Side-by-side test shows identical output