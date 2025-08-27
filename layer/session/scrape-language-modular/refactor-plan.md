# Scrape Language-Aware Modularization Plan (LLM-Optimized)

## Context
After a failed attempt to modularize the 2000+ line monolithic `scrape.rs` that lost critical language-specific extraction logic, we're taking an LLM-friendly approach that prioritizes comprehension and discoverability.

## Problem Analysis

### What Went Wrong Previously
- Generic extraction using `child_by_field_name("name")` only works for Rust
- Lost language-specific visibility rules (Go uppercase, Python underscore)
- Lost language-specific AST node names and structures
- Resulted in 58% MORE functions and 11x MORE relations (incorrect behavior)
- **Too abstract for LLM comprehension** - logic scattered across 20+ files

### Current State
- `src/commands/scrape.rs`: 2072 lines, monolithic but WORKING
- Supports: Rust, Go, Python, JavaScript/JSX, TypeScript/TSX, Solidity
- Each language has 100-200 lines of specific logic scattered throughout
- Database storage:
  - `.patina/knowledge.db` for local project scraping
  - `layer/dust/repos/{repo_name}.db` for external repository analysis

### Current Function Sizes (Too Large)
```
320 lines - process_ast_node()       # Does everything
292 lines - extract_fingerprints()   # Main extraction loop
280 lines - extract_function_facts() # All language logic
194 lines - extract_call_expressions()
```

## LLM-Friendly Design Principles

### 1. **Goldilocks Abstraction**
- Not too abstract (loses context)
- Not too monolithic (exceeds context window)
- Just right: 300-500 line files with complete logic

### 2. **Locality of Reference**
All logic for a language in ONE file:
- `rust_extractor.rs` - ALL Rust logic
- `go_extractor.rs` - ALL Go logic
- No jumping between files to understand one language

### 3. **Explicit Over Clever**
```rust
// BAD - Too abstract
trait Extractor {
    fn get_name(&self, node: &Node) -> String;
}

// GOOD - Explicit and discoverable
impl GoExtractor {
    fn get_function_name(&self, node: &Node) -> String {
        // Go: function_declaration has name field
        node.child_by_field_name("name")...
    }
}
```

## Proposed LLM-Optimized Architecture

### Existing Files to Keep

1. **`languages.rs`** - Critical infrastructure, heavily used:
   - `Language` enum (Rust, Go, Python, etc.)
   - `Language::from_path()` - Detects language from file extension
   - `create_parser()` - Creates tree-sitter parsers for each language
   - Used throughout scrape.rs for language detection

2. **`fingerprint.rs`** - Partially used, provides DB schema:
   - `generate_schema()` - Creates database tables (REQUIRED)
   - `Fingerprint` struct - Code pattern detection (currently unused but could be useful)
   - Generates 16-byte fingerprints from AST nodes

3. **`queries.rs`** - Completely unused, can be deleted:
   - Contains Rust-specific tree-sitter queries
   - Never imported anywhere in codebase
   - Likely from abandoned experiment

4. **`mod.rs`** - Module exports, needs update:
   - Currently just exports the 3 modules
   - Will need to add `pub mod scrape;` after refactor

### Directory Structure
```
src/semantic/
├── scrape/                          # Clear purpose: scraping
│   ├── mod.rs                      # Orchestration only (200 lines)
│   ├── database.rs                 # DB init & queries (150 lines)
│   ├── file_discovery.rs           # Finding files (100 lines)
│   ├── common/
│   │   ├── sql_builder.rs          # SQL generation (200 lines)
│   │   └── tree_walker.rs          # Tree traversal (100 lines)
│   └── extractors/
│       ├── mod.rs                  # Simple factory (50 lines)
│       ├── rust_extractor.rs       # ALL Rust logic (400 lines)
│       ├── go_extractor.rs         # ALL Go logic (400 lines)
│       ├── python_extractor.rs     # ALL Python logic (400 lines)
│       ├── javascript_extractor.rs # JS/JSX logic (400 lines)
│       ├── typescript_extractor.rs # TS/TSX logic (400 lines)
│       └── solidity_extractor.rs   # ALL Solidity logic (300 lines)
├── fingerprint.rs  # (KEEP) Code fingerprinting, provides DB schema
├── languages.rs    # (KEEP) Language enum, file detection, parser creation
├── queries.rs      # (UNUSED) Could be deleted - tree-sitter queries never used
└── mod.rs          # (UPDATE) Will export new scrape module
```

### Minimal Trait (One Simple Interface)
```rust
// extractors/mod.rs - Just enough for polymorphism
pub trait LanguageExtractor {
    fn extract_file(&self, path: &Path, source: &str) -> ExtractResult;
}

pub fn create_extractor(language: Language) -> Box<dyn LanguageExtractor> {
    match language {
        Language::Rust => Box::new(RustExtractor),
        Language::Go => Box::new(GoExtractor),
        Language::Python => Box::new(PythonExtractor),
        Language::JavaScript | Language::JavaScriptJSX => Box::new(JavaScriptExtractor),
        Language::TypeScript | Language::TypeScriptTSX => Box::new(TypeScriptExtractor),
        Language::Solidity => Box::new(SolidityExtractor),
        _ => Box::new(NullExtractor),
    }
}
```

### Example: Complete Go Extractor
```rust
// go_extractor.rs - Everything about Go in ONE file
pub struct GoExtractor;

impl LanguageExtractor for GoExtractor {
    fn extract_file(&self, path: &Path, source: &str) -> ExtractResult {
        let mut result = ExtractResult::default();
        
        // Parse with Go grammar
        let tree = self.parse(source)?;
        
        // Walk tree and extract
        self.walk_tree(tree.root_node(), source, &mut result);
        
        result
    }
}

impl GoExtractor {
    // ALL Go-specific methods in this ONE file
    
    fn is_public(&self, name: &str) -> bool {
        // Go: uppercase = public
        name.chars().next().map_or(false, |c| c.is_uppercase())
    }
    
    fn extract_function(&self, node: Node, source: &str) -> Function {
        // Go function_declaration or method_declaration
        let name = node.child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .unwrap_or("unknown");
            
        let is_method = node.kind() == "method_declaration";
        let receiver = if is_method {
            self.extract_receiver(node, source)
        } else {
            None
        };
        
        Function {
            name: name.to_string(),
            is_public: self.is_public(name),
            receiver,
            // ... complete Go-specific extraction
        }
    }
    
    fn extract_doc_comment(&self, node: Node, source: &str) -> Option<String> {
        // Go: // comments above declaration
        let mut cursor = node.walk();
        if cursor.goto_previous_sibling() {
            let prev = cursor.node();
            if prev.kind() == "comment" {
                let text = prev.utf8_text(source).ok()?;
                if text.starts_with("//") {
                    return Some(text[2..].trim().to_string());
                }
            }
        }
        None
    }
    
    fn extract_imports(&self, node: Node, source: &str) -> Vec<Import> {
        // Go: import statements
        // Complete implementation here
    }
    
    // ALL other Go methods...
}
```

## Language-Specific Requirements

### Rust
- **Visibility**: `pub`, `pub(crate)`, `pub(super)`, private by default
- **Doc Comments**: `///` for items, `//!` for modules
- **Async/Unsafe**: First-class keywords
- **Generics**: Type parameters and lifetimes
- **Special**: `impl` blocks, traits, macros

### Go
- **Visibility**: Uppercase = public, lowercase = private
- **Doc Comments**: `//` above declaration
- **Methods**: Receiver syntax `(r *Type)`
- **Special**: Multiple return values, error as second return

### Python
- **Visibility**: `_` prefix = private, `__` = name mangling
- **Doc Comments**: Docstrings as first statement
- **Async**: `async def` functions
- **Type Hints**: Optional but extract if present
- **Special**: Decorators, class methods

### JavaScript/JSX
- **Visibility**: `export` keyword
- **Doc Comments**: `/** */` JSDoc style
- **Async**: `async` functions, Promise returns
- **Special**: Arrow functions, React components (JSX)
- **File**: `javascript_extractor.rs` handles both JS and JSX

### TypeScript/TSX
- **Visibility**: `export` keyword, class member modifiers
- **Doc Comments**: `/** */` JSDoc/TSDoc style
- **Async**: `async` functions, Promise<T> returns
- **Types**: Full type annotations, interfaces, generics
- **Special**: Type guards, decorators, React components (TSX)
- **File**: `typescript_extractor.rs` handles both TS and TSX

### Solidity
- **Visibility**: `public`, `private`, `internal`, `external`
- **Doc Comments**: `///` NatSpec format
- **Special**: Modifiers, events, state variables

## Implementation Strategy

### Phase 1: Foundation (Day 1)
1. Create `src/semantic/scrape/` directory structure
2. Move database operations to `database.rs`
3. Move file discovery to `file_discovery.rs`
4. Create minimal trait in `extractors/mod.rs`

### Phase 2: Rust Extractor (Day 2)
1. Create `rust_extractor.rs` with ALL Rust logic
2. Copy all Rust-specific code from monolith
3. Test output matches exactly

### Phase 3: Other Languages (Day 3-4)
1. `go_extractor.rs` - Test with dagger repo
2. `python_extractor.rs` - Test with Python project
3. `javascript_extractor.rs` - Handles JS and JSX
4. `typescript_extractor.rs` - Handles TS and TSX
5. `solidity_extractor.rs` - Test with smart contracts

### Phase 4: Cleanup (Day 5)
1. Remove old monolithic file
2. Optimize common utilities
3. Performance testing

## Testing Strategy

### Database Storage (Unchanged)
- **Local Project**: `.patina/knowledge.db`
- **External Repos**: `layer/dust/repos/{repo_name}.db`

### Validation Process
```bash
# Before refactor
patina scrape --force --repo dagger > before.sql

# After refactor
patina scrape --force --repo dagger > after.sql

# Must be identical
diff before.sql after.sql
```

### Test Repositories
- **Rust**: patina (this repo)
- **Go**: dagger/dagger
- **Python**: Popular Python project
- **JavaScript**: React project
- **TypeScript**: TypeScript compiler itself
- **Solidity**: OpenZeppelin contracts

## Why This Works for LLMs

### 1. **Single File Context**
"Fix Go extraction" = Read `go_extractor.rs` only

### 2. **No Hidden Behavior**
Everything explicit in one place, no trait maze

### 3. **Clear Navigation**
File names match exactly what they contain

### 4. **Optimal Size**
400 lines fits in context window with room for edits

### 5. **Pattern Recognition**
Each extractor follows same structure, easy to learn

## Anti-Patterns We're Avoiding

### ❌ Deep Trait Hierarchies
```rust
// BAD - LLM loses context
trait BaseExtractor {
    trait LanguageExtractor: BaseExtractor {
        trait TypedExtractor: LanguageExtractor {
            // 3 files deep to understand
        }
    }
}
```

### ❌ Scattered Logic
```rust
// BAD - Need 10 files open
impl GetName for Go { }      // in name.rs
impl GetVisibility for Go { } // in visibility.rs  
impl GetParams for Go { }     // in params.rs
```

### ✅ Colocated Logic
```rust
// GOOD - Everything in one place
// go_extractor.rs
impl GoExtractor {
    fn get_name() { }
    fn get_visibility() { }
    fn get_params() { }
}
```

## Success Criteria

1. **Functional Parity**: Identical extraction results
2. **File Size**: No file over 500 lines
3. **LLM Friendly**: Can understand any language with one file
4. **Performance**: Within 10% of monolithic version
5. **Maintainable**: Adding language = copy extractor, modify

## Key Insight

**For LLMs, locality beats abstraction.** A 400-line file with some duplication is far better than perfect DRY across 20 files. The LLM can hold the entire context for "Go extraction" in memory at once, making it easy to understand and modify.

This isn't about being anti-abstraction - it's about choosing the right abstraction level for AI-assisted development. We abstract at the language boundary, not at the micro-feature level.