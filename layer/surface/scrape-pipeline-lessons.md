---
id: scrape-pipeline-lessons
status: active
created: 2025-08-27
tags: [architecture, llm-patterns, black-box, modularity, lessons-learned, scrape-pipeline]
---

# Scrape Pipeline: Why the Monolith Won

Lessons learned from attempting to modularize a 2000-line semantic code scraper.

---

## The Story

- **Original**: 2000-line monolithic `scrape.rs` - built in a few hours, works perfectly
- **Attempt 1**: Separate `patina-index` binary with pipeline architecture - failed with schema mismatches
- **Attempt 2**: Modular parsers (Rust, Go, Python, JS) - failed with missing fingerprints
- **Attempt 3**: Rich analysis integration - failed with incomplete implementations
- **Time spent**: 2-3 days of failed modularization vs few hours for working monolith

## Why the Monolith Works

### 1. No Serialization Boundaries
```rust
// Monolith: Everything in memory, no schema contracts
let mut symbol_count = 0;
process_ast_node(&mut cursor, content.as_bytes(), &file, &mut sql, language, &mut context);
symbol_count += process_ast_node(...);  // Just increment a counter

// Modular: Must serialize/deserialize, schemas must match
let ast_data = parse_file(file)?;  // Returns AstData with required fields
write_json(&ast_data)?;             // Must serialize completely
let data = read_json()?;            // Must deserialize perfectly
generate_sql(&data)?;               // Expects all fields present
```

### 2. Implicit State Sharing
The monolith passes around mutable references and builds SQL incrementally:
- No need to define intermediate data structures
- "Fingerprints" are just a counter, not actual objects
- SQL is built as a string, appended to during tree walking
- Context flows through function calls without formal contracts

### 3. Single-Pass Processing
```rust
// Monolith: One pass through the AST does everything
for file in files {
    parse_and_generate_sql_and_insert(file);  // All in one go
}

// Modular: Multiple passes with coordination overhead
for file in files { parse_to_json(file); }      // Pass 1
for json in jsons { validate_schema(json); }    // Pass 2  
generate_bulk_sql(all_jsons);                   // Pass 3
execute_sql(sql);                                // Pass 4
```

## Why Modularization Failed

### The Schema Mismatch Problem

```rust
// New system expects rich data
pub struct AstData {
    pub fingerprints: Vec<CodeFingerprint>,  // Required but not generated
    pub symbols: Vec<Symbol>,                // Required but not generated
    pub file_metrics: Option<FileMetrics>,   // Optional but still expected
}

// Only Rust parser was updated to provide these
// JavaScript parser: TODO
// Python parser: TODO  
// Go parser: TODO
// TypeScript parser: TODO
```

### The Actual "Fingerprints"
```rust
// What we thought fingerprints were:
struct CodeFingerprint {
    hash: u32,
    pattern: String,
    location: Location,
}

// What they actually are in the monolith:
symbol_count += 1;  // That's it. Just counting SQL INSERTs.
println!("✓ Fingerprinted {} symbols", symbol_count);
```

### The Lost Context Problem
- Monolith: "fingerprints" never leave the function, just a local counter
- Modular: Must serialize fingerprints to JSON, then deserialize, then generate SQL
- The abstraction doesn't match the implementation

## The Real Lesson: When to Modularize

### DON'T Modularize When:
- **It works** (2000 lines that work > 10 files that don't)
- **You're the only user** (no need for team boundaries)
- **Performance matters** (serialization overhead is real)
- **The domain is still evolving** (scraping logic still changing)
- **There's no reuse opportunity** (nobody else needs these parsers)

### DO Modularize When:
- **Multiple teams** need to work independently
- **Components need different deployment** strategies
- **Real reuse opportunity** exists (not hypothetical)
- **Performance isolation** is needed (one slow part shouldn't block others)
- **The contracts are stable** (you know exactly what data flows between parts)

## The Better Solution: Black Box With API

Instead of breaking apart the monolith, wrap it:

```rust
// src/commands/scrape.rs - Keep ALL 2000 lines intact

// Add at the top:
pub struct Scraper {
    repo: String,
}

impl Scraper {
    pub fn new(repo: &str) -> Self {
        Self { repo: repo.to_string() }
    }
    
    pub fn scrape(&self, force: bool) -> Result<ScrapeStats> {
        // Call existing execute() function
        execute(false, None, Some(&self.repo), force)?;
        
        // Query the database it created for stats
        let db_path = format!("layer/dust/repos/{}.db", self.repo);
        Ok(extract_stats(&db_path)?)
    }
    
    pub fn query(&self, sql: &str) -> Result<QueryResult> {
        execute(false, Some(sql.to_string()), Some(&self.repo), false)?;
        // ... convert prints to data
    }
}

// The original 2000 lines stay EXACTLY as they are
```

Now you can build on top:
```rust
// New file: src/commands/compare.rs
use crate::commands::scrape::Scraper;

pub fn compare(repo1: &str, repo2: &str) -> Result<()> {
    let scraper1 = Scraper::new(repo1);
    let scraper2 = Scraper::new(repo2);
    
    let stats1 = scraper1.scrape(false)?;
    let stats2 = scraper2.scrape(false)?;
    // ... comparison logic
}
```

## For LLM Development

### The Wrong Approach
"Break the 2000-line file into smaller files so the LLM can read them"
- Result: LLM needs MORE context to understand how pieces fit together
- Schema mismatches between components
- Days of debugging serialization issues

### The Right Approach
"Add a 50-line public API to the 2000-line file"
- LLM only reads the API signatures
- Implementation stays hidden but working
- New features built in separate files using the API

### The Prompt Pattern
```
The scrape module has these public functions:
- Scraper::new(repo) -> Scraper
- Scraper::scrape(force) -> ScrapeStats  
- Scraper::query(sql) -> QueryResult

Don't read or modify the implementation. Build a new feature using these APIs.
```

## Specific Takeaways

1. **The monolith's speed comes from avoiding serialization** - Everything stays in memory
2. **"Fingerprints" were never real objects** - Just a counter incremented during SQL generation
3. **Rich analysis fields were aspirational** - Only Rust parser implemented them
4. **The pipeline architecture is theoretically better** - But practically worse without full implementation
5. **2-3 days of modularization < few hours of monolith** - Time to value matters

## The Verdict

**Keep the monolith. Add an API. Build new features on top.**

The 2000-line `scrape.rs` is:
- Fast (no serialization overhead)
- Working (handles 6000+ functions in Dagger repo)
- Maintainable (one file, one owner)
- Complete (all features implemented)

The modular pipeline would need:
- Schema versioning
- Complete parser implementations for all languages
- Error handling for missing fields
- Serialization/deserialization overhead
- Coordination between components

**Sometimes, a working monolith is the right architecture.**

## References

- Original design: `layer/surface/scrape-pipeline-design.md`
- Session notes: `layer/sessions/20250827-152837.md`
- Philosophy: `layer/surface/eskil-steenberg-rust.md`
- General pattern: `layer/surface/black-box-modules-for-llms.md`