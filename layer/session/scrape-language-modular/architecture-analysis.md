# Current vs Ideal Architecture Analysis

## Current Architecture - How It's Actually Designed

### The Monolithic Pipeline
```
execute()
├── validate_repo_path()      # Determines DB location
├── initialize_database()     # Creates schema using fingerprint::generate_schema()
├── run_query()               # Query existing data
└── extract_and_index()       # THE MONSTER - does everything
    ├── extract_git_metrics()     # Git history analysis
    ├── extract_pattern_references() # Pattern detection in markdown
    └── extract_fingerprints()   # THE REAL MONSTER (292 lines)
        └── [Inline Processing Loop]
            ├── File discovery
            ├── Language detection  
            ├── Parser creation/switching
            ├── AST traversal
            ├── process_ast_node() # 320 lines of switch statements
            │   ├── extract_function_facts()
            │   ├── extract_type_definition()
            │   ├── extract_import_fact()
            │   └── extract_behavioral_hints()
            ├── SQL generation (inline)
            └── Batch execution
```

### Current Problems

#### 1. **Function Boundaries Are Wrong**
- `extract_fingerprints()` doesn't just extract fingerprints - it orchestrates EVERYTHING
- `process_ast_node()` is a 320-line switch statement handling all node types
- Language-specific logic is scattered across 8+ functions

#### 2. **Data Flow is Implicit**
```rust
// Current: Everything happens through side effects
let mut sql = String::new();
process_ast_node(node, source, file, &mut sql, language);
// What data was extracted? What's in sql? Mystery!
```

#### 3. **SQL Generation is Scattered**
- Every extraction function builds SQL strings directly
- No data structures - just string concatenation
- Can't test extraction without database

#### 4. **Incremental Updates Are Fragile**
```rust
// Checks mtime, but then deletes ALL data for the file
if needs_update {
    sql.push_str(&format!("DELETE FROM function_facts WHERE file = '{}'", file));
    // Hope we rebuild everything correctly!
}
```

#### 5. **No Separation of Concerns**
- Parsing mixed with extraction
- Extraction mixed with SQL generation  
- Business logic mixed with I/O

## How It SHOULD Be Designed

### Clean Architecture Layers

```
┌─────────────────────────────────────────────┐
│           Command Layer (Thin)              │
│  execute() - Just orchestration             │
└─────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────┐
│          File Discovery Layer               │
│  find_files() -> Vec<(PathBuf, Language)>   │
└─────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────┐
│          Extraction Layer (Pure)            │
│  extract(file, source) -> SemanticData      │
│  ├── RustExtractor                          │
│  ├── GoExtractor                            │
│  └── [Other Language Extractors]            │
└─────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────┐
│         Transformation Layer                │
│  SemanticData -> DatabaseRecords            │
│  (Fingerprinting happens here)              │
└─────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────┐
│          Storage Layer                      │
│  save(records) -> Result<()>                │
│  (SQL generation isolated here)             │
└─────────────────────────────────────────────┘
```

### Ideal Data Structures

```rust
// Clear data types instead of SQL strings
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
    // Language-agnostic representation
}
```

### Ideal Extraction Flow

```rust
// 1. Discovery (Pure)
let files = discover_files(&work_dir)?
    .filter(|f| should_update(f, last_run));

// 2. Extraction (Pure, Parallel)
let extractions: Vec<SemanticData> = files
    .par_iter()
    .map(|(path, language)| {
        let source = read_file(path)?;
        let extractor = create_extractor(language);
        extractor.extract(path, &source)
    })
    .collect();

// 3. Transformation (Pure)
let records: Vec<DatabaseRecord> = extractions
    .into_iter()
    .flat_map(|data| transform_to_records(data))
    .map(|record| add_fingerprint(record))
    .collect();

// 4. Storage (I/O)
database.save_batch(records)?;
```

### Language Extractor Design

```rust
// Each language in its own file, implementing a simple trait
pub trait LanguageExtractor {
    fn extract(&self, path: &Path, source: &str) -> Result<SemanticData>;
}

// go_extractor.rs - Complete, self-contained
pub struct GoExtractor;

impl GoExtractor {
    fn extract(&self, path: &Path, source: &str) -> Result<SemanticData> {
        let tree = self.parse(source)?;
        
        SemanticData {
            functions: self.extract_functions(&tree, source),
            types: self.extract_types(&tree, source),
            imports: self.extract_imports(&tree, source),
            calls: self.extract_calls(&tree, source),
            docs: self.extract_docs(&tree, source),
        }
    }
    
    // All Go-specific logic in this file
    fn extract_functions(&self, tree: &Tree, source: &str) -> Vec<FunctionInfo> {
        // Go-specific: function_declaration, method_declaration
        // Go-specific: Uppercase = public
        // Complete implementation here
    }
}
```

### Why This is Better

#### 1. **Testable**
```rust
#[test]
fn test_go_function_extraction() {
    let extractor = GoExtractor;
    let source = "func Hello() { }";
    let data = extractor.extract(Path::new("test.go"), source)?;
    assert_eq!(data.functions[0].name, "Hello");
    assert!(data.functions[0].visibility.is_public());
}
```

#### 2. **Composable**
```rust
// Easy to add new languages
let extractor: Box<dyn LanguageExtractor> = match language {
    Language::Rust => Box::new(RustExtractor),
    Language::Go => Box::new(GoExtractor),
    Language::NewLang => Box::new(NewLangExtractor), // Just add this
};
```

#### 3. **Maintainable**
- Fix Go extraction? Edit `go_extractor.rs` only
- Change database schema? Edit storage layer only
- Add caching? Wrap the extraction layer

#### 4. **LLM-Friendly**
- "Fix Go visibility detection" → Open `go_extractor.rs`, find `extract_functions()`
- "How are functions stored?" → Open storage layer
- "What data is extracted?" → Look at `SemanticData` struct

#### 5. **Incremental Updates Work**
```rust
// Current: Delete everything, hope rebuild works
// Better: Track what changed
pub struct IncrementalExtractor {
    cache: HashMap<PathBuf, SemanticData>,
}

impl IncrementalExtractor {
    fn extract_changed(&self, path: &Path) -> SemanticData {
        let old_data = self.cache.get(path);
        let new_data = extract(path);
        merge_changes(old_data, new_data)
    }
}
```

## Migration Strategy

### Phase 1: Data Structures (Day 1)
Create the data types that represent extraction results:
- `SemanticData`, `FunctionInfo`, `TypeInfo`, etc.
- Keep these in `src/semantic/scrape/types.rs`

### Phase 2: Extract One Language (Day 2)
Start with Rust (most complex):
1. Create `rust_extractor.rs`
2. Implement `extract()` returning `SemanticData`
3. Test against current SQL output

### Phase 3: Storage Layer (Day 3)
Create clean storage abstraction:
1. `storage/mod.rs` - Trait definition
2. `storage/duckdb.rs` - SQL generation
3. Transform `SemanticData` → SQL

### Phase 4: Wire It Up (Day 4)
Replace monolithic pipeline:
1. Keep existing for comparison
2. Add feature flag to switch
3. Validate identical outputs

## The Key Insight

The current architecture evolved organically - it started simple and grew by accretion. The functions don't represent logical boundaries, they represent historical growth:

- `extract_fingerprints()` - Started as fingerprinting, became the main loop
- `process_ast_node()` - Started small, grew to 320 lines
- SQL strings everywhere - Started with one INSERT, grew to hundreds

The ideal architecture enforces boundaries:
- **Extraction** is pure computation (no I/O)
- **Storage** handles all SQL (no business logic)
- **Each language** owns its logic (no scattering)
- **Data structures** make flow explicit (no string building)

This isn't over-engineering - it's setting boundaries that prevent the code from becoming another monolith.