# Scrape Pipeline Design

## Problem Statement

The current `scrape.rs` is a 2000+ line monolith that couples:
- File discovery
- Language detection  
- Tree-sitter parsing
- SQL generation
- DuckDB execution
- Incremental updates
- Git analysis

This coupling makes it hard to:
- Add new languages
- Test components independently
- Cache intermediate results
- Optimize performance
- Debug issues

## Proposed Design: Decouple Parse and Load

### Core Insight

Tree-sitter parsing and SQL generation are **different problems**. Don't mix them!

### Pipeline Architecture

```
Repository → Parse → Intermediate Format → Transform → DuckDB
```

### Implementation

```rust
// src/bin/patina-index.rs - Separate binary
fn main() {
    let repo = std::env::args().nth(1).unwrap();
    
    // Phase 1: Analysis (can be parallel)
    let git_metrics = git_analyze(&repo)?;
    let files = discover_files(&repo)?;
    
    // Phase 2: Parse to intermediate format
    for file in files {
        let ast_data = parse_file(&file)?;
        write_json(&file, &ast_data)?;  // .patina/ast_cache/file.rs.json
    }
    
    // Phase 3: Generate bulk SQL
    generate_sql(".patina/ast_cache/", ".patina/load.sql")?;
    
    // Phase 4: Single bulk load
    duckdb::execute_file(".patina/load.sql")?;
}
```

### Intermediate Format Schema

```json
{
  "file": "src/main.rs",
  "language": "rust",
  "functions": [
    {
      "name": "main",
      "visibility": "public",
      "async": false,
      "unsafe": false,
      "params": [],
      "returns": null,
      "line_start": 1,
      "line_end": 5
    }
  ],
  "types": [...],
  "imports": [...],
  "calls": [...]
}
```

### File Layout

```
.patina/
├── ast_cache/               # Parsed AST data (cacheable!)
│   ├── src/main.rs.json
│   ├── src/lib.rs.json
│   └── ...
├── git_metrics.json         # Git history analysis
├── patterns.json            # Pattern references
├── load.sql                 # Generated bulk insert
└── semantic.db              # Final DuckDB
```

## Benefits

### 1. Composability
```bash
# Unix philosophy - compose tools
find . -name "*.rs" | patina-parse | patina-to-sql | duckdb

# Or direct DuckDB import
duckdb semantic.db "COPY functions FROM '.patina/ast_cache/*.json'"
```

### 2. Language Parsers as Separate Binaries
```
patina-parse-rust     # 300 lines, just Rust parsing
patina-parse-go       # 250 lines, just Go parsing  
patina-parse-python   # 280 lines, just Python parsing
```

Each parser:
- Single responsibility
- Easy to test
- Can evolve independently
- No cross-contamination

### 3. Cacheable Intermediate Results
- Parse once, query many times
- Incremental updates only re-parse changed files
- Can version the schema
- Can compress/optimize storage

### 4. DuckDB Native Performance
```sql
-- Let DuckDB handle the heavy lifting
COPY functions FROM '.patina/ast_cache/*.json' (FORMAT JSON);

-- Or even better, Parquet for columnar storage
COPY functions FROM '.patina/ast_cache/*.parquet' (FORMAT PARQUET);
```

### 5. Testing & Debugging
- Test parsers with simple JSON output
- Test SQL generation separately
- Can inspect intermediate files
- Can replay from cached data

## Migration Path

1. **Phase 1**: Keep current scrape.rs, add JSON export
2. **Phase 2**: Build language-specific parsers that output JSON
3. **Phase 3**: Build SQL generator from JSON
4. **Phase 4**: Switch to pipeline mode by default
5. **Phase 5**: Remove old monolithic code

## Example: Adding a New Language

Current approach (modify 100+ lines in scrape.rs):
```rust
// Add to giant match statement
(Language::Ruby, "method_definition") => "function",
// Add to visibility check
Language::Ruby => !name.starts_with('_'),
// Add to async check
Language::Ruby => false,
// ... etc for 10+ places
```

New approach (new file, standalone):
```rust
// src/bin/patina-parse-ruby.rs
fn main() {
    let file = std::env::args().nth(1).unwrap();
    let ast = parse_ruby(&file);
    println!("{}", serde_json::to_string(&ast)?);
}
```

## Performance Considerations

### Current: Sequential & Coupled
```
for file in files {
    parse(file) -> generate_sql(file) -> execute_sql(file)
}
```

### Proposed: Parallel & Batched
```
// All parallel
files.par_iter().map(|f| parse(f)).collect()

// Single batch
generate_bulk_sql(all_parsed_data)
duckdb.execute_batch(sql)
```

### Expected Improvements
- **Parse phase**: 8x faster with parallel processing
- **SQL generation**: 10x faster with batching
- **DuckDB load**: 100x faster with bulk COPY vs individual INSERTs
- **Incremental**: Only re-parse changed files (cache hit rate ~95%)

## Trade-offs

### Pros
- Clean separation of concerns
- Easy to add languages
- Better performance
- Cacheable/resumable
- Testable components
- Follows Unix philosophy

### Cons  
- More moving parts
- Intermediate storage overhead (~2x disk during processing)
- Need to maintain schema compatibility
- Initial implementation effort

## Recommendation

The current monolithic design works but fights against its own nature. This pipeline approach aligns with:
- How tree-sitter works (parse to AST)
- How DuckDB works (bulk operations)  
- How developers think (separation of concerns)
- How LLMs work (can hold one parser in context)

The intermediate format is the key - it decouples parsing from storage and makes the system composable.