# Side-by-Side Migration Plan

## Building the Ideal Architecture Alongside the Old

This implements EXACTLY the clean layer architecture from our analysis:

```
┌────────────────┐
│ Command (Thin) │ - Just orchestration
├────────────────┤
│ Discovery      │ - Find files, detect languages  
├────────────────┤
│ Extraction     │ - Pure functions, no I/O
│ (Per Language) │ - Returns SemanticData structs
├────────────────┤
│ Transformation │ - Add fingerprints, prepare records
├────────────────┤
│ Storage        │ - SQL generation, database I/O
└────────────────┘
```

## File Structure

```
src/
├── commands/
│   ├── scrape.rs        → scrape_old.rs  (RENAME)
│   ├── scrape.rs        (NEW thin wrapper)
│   └── mod.rs           (UPDATE)
└── scrape/              (NEW - implements clean architecture)
    ├── mod.rs           # Command Layer (thin orchestration)
    ├── discovery.rs     # Discovery Layer (find files, detect languages)
    ├── extraction/      # Extraction Layer (pure, no I/O)
    │   ├── mod.rs       # SemanticData types & trait
    │   ├── rust.rs      # RustExtractor 
    │   ├── go.rs        # GoExtractor
    │   ├── python.rs    # PythonExtractor
    │   ├── javascript.rs # JavaScriptExtractor
    │   ├── typescript.rs # TypeScriptExtractor
    │   └── solidity.rs  # SolidityExtractor
    ├── transform.rs     # Transformation Layer (fingerprints, prepare records)
    └── storage.rs       # Storage Layer (SQL generation, DB I/O)
```

## Layer Implementations

### 1. Command Layer (src/scrape/mod.rs)
```rust
// THIN orchestration only - no business logic
pub fn execute(init: bool, query: Option<String>, repo: Option<String>, force: bool) -> Result<()> {
    let (db_path, work_dir) = determine_paths(repo)?;
    
    if init {
        storage::initialize_database(&db_path)?;
    } else if let Some(q) = query {
        storage::run_query(&q, &db_path)?;
    } else {
        orchestrate_extraction(&db_path, &work_dir, force)?;
    }
    Ok(())
}

fn orchestrate_extraction(db_path: &str, work_dir: &Path, force: bool) -> Result<()> {
    // Just orchestration - each layer does its job
    let files = discovery::find_files(work_dir)?;              // Layer 1
    let semantic_data = extraction::extract_all(files)?;       // Layer 2
    let records = transform::to_records(semantic_data)?;       // Layer 3
    storage::save_batch(db_path, records)?;                    // Layer 4
    Ok(())
}
```

### 2. Discovery Layer (src/scrape/discovery.rs)
```rust
// PURE: Find files and detect languages
use crate::semantic::languages::Language;

pub fn find_files(work_dir: &Path) -> Result<Vec<(PathBuf, Language)>> {
    walkdir::WalkDir::new(work_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|entry| {
            let path = entry.path().to_owned();
            let language = Language::from_path(&path);
            (path, language)
        })
        .filter(|(_, lang)| *lang != Language::Unknown)
        .collect()
}
```

### 3. Extraction Layer (src/scrape/extraction/mod.rs)
```rust
// PURE: Extract to data structures, no I/O, no SQL
pub struct SemanticData {
    pub functions: Vec<FunctionInfo>,
    pub types: Vec<TypeInfo>,
    pub imports: Vec<ImportInfo>,
    pub calls: Vec<CallGraph>,
    pub docs: Vec<Documentation>,
}

pub struct FunctionInfo {
    pub name: String,
    pub visibility: Visibility,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub line_start: usize,
    pub line_end: usize,
}

// Simple trait - one method
pub trait LanguageExtractor {
    fn extract(&self, path: &Path, source: &str) -> Result<SemanticData>;
}

pub fn extract_all(files: Vec<(PathBuf, Language)>) -> Result<Vec<SemanticData>> {
    files.par_iter()
        .map(|(path, language)| {
            let source = std::fs::read_to_string(path)?;
            let extractor = create_extractor(*language);
            extractor.extract(path, &source)
        })
        .collect()
}

fn create_extractor(language: Language) -> Box<dyn LanguageExtractor> {
    match language {
        Language::Rust => Box::new(rust::RustExtractor),
        Language::Go => Box::new(go::GoExtractor),
        // etc...
    }
}
```

### 4. Transformation Layer (src/scrape/transform.rs)
```rust
// PURE: Transform data structures, add fingerprints
use crate::semantic::fingerprint::Fingerprint;

pub struct DatabaseRecord {
    pub table: String,
    pub values: HashMap<String, Value>,
    pub fingerprint: Option<Fingerprint>,
}

pub fn to_records(semantic_data: Vec<SemanticData>) -> Result<Vec<DatabaseRecord>> {
    semantic_data
        .into_iter()
        .flat_map(|data| {
            let mut records = Vec::new();
            
            // Transform functions
            for func in data.functions {
                records.push(DatabaseRecord {
                    table: "function_facts".to_string(),
                    values: function_to_values(func),
                    fingerprint: Some(calculate_fingerprint(&func)),
                });
            }
            
            // Transform types, imports, etc...
            records
        })
        .collect()
}
```

### 5. Storage Layer (src/scrape/storage.rs)
```rust
// I/O: All SQL generation and database operations
use crate::semantic::fingerprint;

pub fn initialize_database(db_path: &str) -> Result<()> {
    let schema = fingerprint::generate_schema();
    // Execute schema...
}

pub fn save_batch(db_path: &str, records: Vec<DatabaseRecord>) -> Result<()> {
    let sql = records_to_sql(records);
    execute_sql(db_path, &sql)?;
    Ok(())
}

fn records_to_sql(records: Vec<DatabaseRecord>) -> String {
    // Only place SQL is generated
    records.into_iter()
        .map(|record| generate_insert(record))
        .collect::<Vec<_>>()
        .join("\n")
}
```

## Key Differences from Previous Proposals

1. **Extraction NOT extractors** - The folder is named for the layer, not the contents
2. **transform.rs NOT transformation/** - Single file, it's simple enough
3. **storage.rs NOT store/** - Single file for SQL generation and DB ops
4. **SemanticData in extraction/mod.rs** - Data types live with the layer that produces them

## This Matches the Ideal Architecture Because:

1. ✅ **Command Layer** - Thin orchestration in `scrape/mod.rs`
2. ✅ **Discovery Layer** - Pure file finding in `discovery.rs`  
3. ✅ **Extraction Layer** - Pure extraction to `SemanticData` in `extraction/`
4. ✅ **Transformation Layer** - Pure data transformation in `transform.rs`
5. ✅ **Storage Layer** - I/O and SQL isolated in `storage.rs`

## Testing Each Layer Independently

```rust
#[test]
fn test_discovery() {
    let files = discovery::find_files(Path::new("test_data"))?;
    assert_eq!(files.len(), 5);
}

#[test]  
fn test_extraction() {
    let extractor = go::GoExtractor;
    let data = extractor.extract(Path::new("test.go"), "func main() {}")?;
    assert_eq!(data.functions[0].name, "main");
}

#[test]
fn test_transformation() {
    let data = SemanticData { functions: vec![...] };
    let records = transform::to_records(vec![data])?;
    assert!(records[0].fingerprint.is_some());
}

#[test]
fn test_storage() {
    let record = DatabaseRecord { ... };
    let sql = storage::records_to_sql(vec![record]);
    assert!(sql.contains("INSERT INTO"));
}
```

## Migration Steps

1. **Day 1**: Rename old, create wrapper with env var switch
2. **Day 2**: Create layer structure and data types
3. **Day 3**: Implement Rust extractor as proof of concept
4. **Day 4**: Implement remaining layers
5. **Day 5**: Add other language extractors
6. **Week 2**: Test and validate identical output

## Why This is the RIGHT Architecture

- **Layers have single responsibilities** - Discovery discovers, extraction extracts, etc.
- **Data flows explicitly** - SemanticData → DatabaseRecord → SQL
- **Pure functions are testable** - No database needed to test extraction
- **I/O is isolated** - Only storage layer touches the database
- **LLM-friendly** - "Fix Go extraction" → `extraction/go.rs`

This is EXACTLY the ideal architecture from our analysis - clean layers with clear boundaries, pure functions, and explicit data flow.