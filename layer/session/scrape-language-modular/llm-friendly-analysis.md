# LLM-Friendly Refactor Analysis

## The Problem with Abstraction for LLMs

The previous refactor failed because it was **too abstract**:
- Generic `child_by_field_name("name")` meant the LLM couldn't see what actually happens for each language
- Scattered logic across multiple files meant holding too much context
- Abstract traits hid the actual implementation details
- The LLM had to jump between files to understand a single operation

## Current Monolithic Structure Analysis

### File Size: 2072 lines
- **Too large** for optimal LLM context (ideal is 200-500, max ~1000)
- But **everything is discoverable** in one place

### Function Size Distribution
```
320 lines - process_ast_node()       # TOO BIG - does everything
292 lines - extract_fingerprints()   # TOO BIG - main extraction loop
280 lines - extract_function_facts() # TOO BIG - all language logic
194 lines - extract_call_expressions()
115 lines - extract_import_fact()
106 lines - extract_doc_comment()
```

### Language Logic Distribution
Each language has logic scattered across **15+ locations**:
1. Doc comment detection (line 743-809)
2. Doc text cleaning (line 844-901)
3. Call expression extraction (line 940-1121)
4. Function fact extraction (line 1541-1810)
5. Type definition extraction (line 1832-1895)
6. Import fact extraction (line 1907-2011)
7. Node name extraction (line 1276-1340)
8. Visibility detection (line 1541-1570)
9. Async detection (line 1571-1594)
10. Parameter extraction (line 1640-1737)
11. Return type extraction (line 1738-1773)
12. And more...

## LLM-Friendly Design Principles

### 1. **Goldilocks Abstraction**
- Not too abstract (previous refactor)
- Not too monolithic (current)
- Just right: Clear modules with explicit purpose

### 2. **Colocate Related Logic**
- All Go logic in one place
- All Rust logic in one place
- Shared utilities clearly marked

### 3. **Optimal File Sizes**
- **Target**: 300-500 lines per file
- **Maximum**: 800 lines
- **Minimum**: 100 lines (don't over-split)

### 4. **Explicit Over Implicit**
```rust
// BAD for LLM - Too abstract
trait Extractor {
    fn get_name(&self, node: &Node) -> String;
}

// GOOD for LLM - Explicit
impl GoExtractor {
    fn get_function_name(&self, node: &Node) -> String {
        // Go functions have name field
        if node.kind() == "function_declaration" {
            node.child_by_field_name("name")...
        }
    }
}
```

### 5. **Discoverability**
File names should tell the LLM exactly what's inside:
- `extractors/go_extractor.rs` - All Go extraction logic
- `extractors/rust_extractor.rs` - All Rust extraction logic
- `common/sql_builder.rs` - SQL generation utilities

## Proposed LLM-Friendly Architecture

### Directory Structure
```
src/semantic/
├── scrape/                      # Clear purpose: scraping
│   ├── mod.rs                  # Orchestration (200 lines)
│   ├── database.rs             # DB init & queries (150 lines)
│   ├── file_discovery.rs       # Finding files (100 lines)
│   └── extractors/
│       ├── mod.rs              # Factory & shared types (100 lines)
│       ├── rust_extractor.rs   # ALL Rust logic (400 lines)
│       ├── go_extractor.rs     # ALL Go logic (400 lines)
│       ├── python_extractor.rs # ALL Python logic (400 lines)
│       ├── js_extractor.rs     # ALL JS/TS logic (500 lines)
│       └── solidity_extractor.rs # ALL Solidity logic (300 lines)
└── [existing files unchanged]
```

### Key Design Decisions

#### 1. **One File Per Language**
Each `*_extractor.rs` contains ALL logic for that language:
```rust
// go_extractor.rs - COMPLETE Go extraction in one place
pub struct GoExtractor;

impl GoExtractor {
    pub fn extract_file(&self, path: &Path, source: &str) -> ExtractResult {
        // All Go logic here, no jumping to other files
    }
    
    fn is_public(&self, name: &str) -> bool {
        // Go: uppercase = public
        name.chars().next().map_or(false, |c| c.is_uppercase())
    }
    
    fn extract_function(&self, node: Node, source: &str) -> Function {
        // Go-specific function extraction
    }
    
    fn extract_doc_comment(&self, node: Node, source: &str) -> Option<String> {
        // Go doc comments start with //
    }
    
    // ALL other Go-specific methods...
}
```

#### 2. **Minimal Traits**
Only one simple trait for the factory pattern:
```rust
pub trait LanguageExtractor {
    fn extract_file(&self, path: &Path, source: &str) -> ExtractResult;
}
```
That's it! No complex trait hierarchies.

#### 3. **Explicit Duplication**
Some code duplication is GOOD for LLM understanding:
```rust
// rust_extractor.rs
fn extract_imports(&self, node: Node) -> Vec<Import> {
    // Rust-specific: use statements
}

// go_extractor.rs  
fn extract_imports(&self, node: Node) -> Vec<Import> {
    // Go-specific: import statements
}
```
The LLM can see exactly what each language does without abstraction.

#### 4. **Shared Utilities Are Obvious**
```rust
// common/sql_builder.rs - Clearly shared
pub fn build_function_insert_sql(func: &Function) -> String {
    // SQL generation - same for all languages
}

// common/tree_walker.rs - Clearly shared
pub fn walk_tree(node: Node, visitor: impl FnMut(Node)) {
    // Tree traversal - same for all languages
}
```

### Migration Strategy

#### Phase 1: Create Structure (Day 1)
1. Create `src/semantic/scrape/` directory
2. Move database init to `database.rs`
3. Move file discovery to `file_discovery.rs`
4. Create `extractors/` subdirectory

#### Phase 2: Extract One Language (Day 2)
1. Start with Rust (most complex)
2. Copy ALL Rust-specific code to `rust_extractor.rs`
3. Remove from monolithic file
4. Test against current output

#### Phase 3: Extract Remaining Languages (Day 3-4)
1. Go extractor
2. Python extractor  
3. JavaScript/TypeScript extractor
4. Solidity extractor

#### Phase 4: Clean Up (Day 5)
1. Remove old monolithic file
2. Optimize shared code
3. Add tests

## Why This Works for LLMs

### 1. **Single Responsibility**
Each file has ONE clear job that fits in its name.

### 2. **Complete Context**
To understand Go extraction, read ONE file: `go_extractor.rs`

### 3. **No Hidden Magic**
Everything is explicit - the LLM can see exactly what happens.

### 4. **Reasonable Chunks**
400-line files fit comfortably in context windows.

### 5. **Clear Navigation**
```
"I need to fix Go extraction" → go_extractor.rs
"I need to fix SQL generation" → common/sql_builder.rs
"I need to add a language" → Copy existing extractor, modify
```

## Anti-Patterns to Avoid

### ❌ Deep Inheritance
```rust
trait BaseExtractor {
    trait FunctionExtractor {
        trait AsyncFunctionExtractor {
            // LLM loses context jumping through traits
        }
    }
}
```

### ❌ Scattered Logic
```rust
// function_name.rs
impl GetName for Go { }

// visibility.rs  
impl GetVisibility for Go { }

// parameters.rs
impl GetParams for Go { }
// LLM needs 10 files open to understand Go
```

### ❌ Over-Abstraction
```rust
fn extract<T: Language>(node: Node) -> Box<dyn Entity> {
    T::get_extractor().extract(node).into_entity()
}
// LLM can't see what actually happens
```

### ✅ Just Right
```rust
// go_extractor.rs - Everything about Go in one place
impl GoExtractor {
    fn extract_function(&self, node: Node) -> Function {
        // Clear, explicit Go logic
        let name = node.child_by_field_name("name");
        let is_public = name.starts_with(char::is_uppercase);
        // etc...
    }
}
```

## Summary

The key insight: **LLMs need locality of reference**. Related code should be physically near each other in a single file of reasonable size. Abstraction is the enemy of LLM comprehension when taken too far.

Our refactor groups ALL logic for each language into a single 400-line file that an LLM can hold entirely in context. This is far better than either:
1. A 2000-line monolith (too big for context)
2. Abstract traits scattered across 20 files (too much jumping)

The result is code that both humans and LLMs can easily navigate and modify.