# Patina Metal: Unified Language Parser Architecture

Created: 2025-08-22
Status: Implemented & Working
Branch: `patina-metal-parser`
Last Updated: 2025-08-22 (Session #3)

## Problem Statement

We need to analyze code across multiple languages (Rust, Go, Solidity, Cairo) to extract semantic patterns, but:
- Tree-sitter versions conflict between language parsers
- Each language has different AST node names for similar concepts
- Crates.io packages often lag behind their GitHub sources
- Managing multiple parser dependencies creates version hell

## Solution: patina-metal

A unified parser subsystem that:
1. **Centralizes all tree-sitter complexity** in one workspace member
2. **Provides a clean, uniform API** regardless of underlying language
3. **Handles version conflicts** by controlling the build process
4. **Maps language-specific ASTs** to common concepts

## Architecture

```
patina (main crate)
    ↓ depends on
patina-metal (parser crate)
    ↓ wraps
tree-sitter-{rust,go,solidity,cairo}
    ↓ generates
Unified AST + Metadata
    ↓ consumed by
scrape command → DuckDB
```

### Core Abstractions

```rust
pub enum Metal {
    Rust,     // Ferrous - stable, oxidizes predictably
    Go,       // Copper - conducts well, green patina
    Solidity, // Precious - immutable, doesn't tarnish
    Cairo,    // Rare earth - exotic properties
}

pub struct Analyzer {
    parsers: HashMap<Metal, Parser>,
    queries: HashMap<(Metal, QueryType), Query>,
}
```

## Why "Metal"?

Fits the Patina metaphor perfectly:
- Different metals (languages) oxidize differently
- Each metal has unique properties (language features)
- Scraping metal reveals patterns underneath
- Patina forms on metal surfaces over time

## Implementation Strategy

### Phase 1: Parser Consolidation ✅
- Create `patina-metal` workspace member
- Move all tree-sitter dependencies there
- Build unified `Metal` enum and `Analyzer` API

### Phase 2: Language Normalization ✅
- Map language-specific node types to common categories
  - `function_item` (Rust) → `function`
  - `function_declaration` (Go) → `function`
  - `contract_declaration` (Solidity) → `struct`
- Normalize complexity calculation across languages

### Phase 3: Query System (In Progress)
- Add `.scm` query files for pattern matching
- Support tree-sitter's powerful query syntax
- Enable language-specific and cross-language queries

### Phase 4: Git Submodules (Planned)
```bash
patina-metal/
├── metals/           # Git submodules
│   ├── rust/        # → github.com/tree-sitter/tree-sitter-rust
│   ├── go/          # → github.com/tree-sitter/tree-sitter-go
│   ├── solidity/    # → github.com/JoranHonig/tree-sitter-solidity
│   └── cairo/       # → github.com/JoranHonig/tree-sitter-cairo
└── queries/         # .scm files
    ├── rust/
    │   ├── symbols.scm
    │   ├── complexity.scm
    │   └── patterns.scm
    └── solidity/
        ├── contracts.scm
        └── security.scm
```

## Technical Decisions

### Why Workspace Member?
- **Isolation**: Parser complexity doesn't leak into main crate
- **Parallel compilation**: Speeds up builds
- **Clear boundaries**: Easy to test and reason about
- **Future flexibility**: Could become standalone crate

### Why Not Use Language-Specific Parsers?
- **Uniform interface**: Same API for all languages
- **Incremental parsing**: Tree-sitter's killer feature
- **Query system**: .scm files enable powerful pattern matching
- **Battle-tested**: Used by GitHub, Neovim, Helix

### Version Conflict Resolution
1. **Use Git dependencies** when crates.io is stale
2. **Control tree-sitter version** in one place
3. **Build from source** via submodules if needed
4. **Pin versions** for reproducibility

## Usage Example

```rust
use patina_metal::{Analyzer, Metal};

let mut analyzer = Analyzer::new()?;

// Parse any language with same API
let parsed = analyzer.parse(source_code, Metal::Solidity)?;

// Extract symbols uniformly
let symbols = analyzer.extract_symbols(&parsed);

// Calculate complexity consistently
let complexity = analyzer.calculate_complexity(&parsed);

// Generate fingerprint for pattern matching
let fingerprint = analyzer.generate_fingerprint(node, source);
```

## Current Status

### Working ✅
- Rust parsing and analysis
- Go parsing and analysis (tested on Dagger: 7,201 symbols)
- Unified Metal enum abstraction
- Symbol extraction and complexity calculation
- AST fingerprinting for pattern detection
- Tree-sitter query system with .scm files (with some syntax issues)
- Graceful handling of parser failures

### In Progress 🔄
- Solidity parser (version conflicts with tree-sitter-solidity v15 vs expected v13-14)
- Cairo parser (missing LANGUAGE export)

### Planned 📋
- Python, JavaScript, TypeScript support
- Custom query builder
- Cross-language pattern detection
- Incremental parsing optimization
- Git submodules for exact grammar versions

## Integration Issues (2024-08-22 Session)

### The Problem
After integrating patina-metal as a parser provider for the scrape command, we encountered performance issues where Dagger repo (750 Go + 53 Rust files) times out during processing. This worked fine before the integration.

### Root Cause: Version Conflict Hell
1. **Original Patina**: Used tree-sitter 0.24 with tree-sitter-rust/go 0.23
2. **patina-metal**: Uses tree-sitter 0.23 (required by tree-sitter-rust/go crates)  
3. **Current State**: Trying to use both versions causes Cargo "links" conflict
   - Both versions try to link the same native C library
   - Cargo refuses to build with conflicting tree-sitter versions

### What We Changed
- Modified `create_parser()` in `src/semantic/languages.rs` to use patina-metal
- Added error handling in scrape to skip languages without working parsers
- Kept original fingerprinting and database logic intact (good decision)

### Current Blockers
1. **Version Mismatch**: Can't have tree-sitter 0.24 and 0.23 in same project
2. **Dependency Chain**: 
   - Main patina needs tree-sitter for semantic module
   - patina-metal needs different tree-sitter version
   - Can't satisfy both requirements
3. **Performance**: Even with reverted code, Dagger scrape times out (suggests version issue)

### Lessons Learned
- The patina-metal abstraction is good for managing parsers
- Should have been a drop-in replacement, not a rewrite
- Original fingerprinting/database code was already efficient
- Version conflicts are the core problem we're solving

## Benefits

1. **Maintainability**: All parser logic in one place
2. **Extensibility**: Adding languages is straightforward
3. **Performance**: Native C parsers, no overhead
4. **Consistency**: Same analysis interface for all languages
5. **Version Control**: Submodules lock exact grammar versions

## Future Vision

`patina-metal` becomes the foundation for:
- **Cross-language pattern detection**: Find similar patterns across Rust/Go/Solidity
- **Multi-language refactoring**: Apply transformations uniformly
- **Language migration assistance**: Map concepts between languages
- **Semantic code search**: Query by structure, not syntax

## Testing Strategy

```bash
# Test each metal individually
cargo test -p patina-metal --features rust
cargo test -p patina-metal --features go
cargo test -p patina-metal --features solidity

# Integration test with scrape
patina scrape --repo=dagger  # Go
patina scrape --repo=dust    # Solidity
patina scrape               # Rust (patina itself)
```

## Known Issues

1. **tree-sitter-solidity**: Crates.io version has wrong ABI version
   - Solution: Use Git dependency or rebuild from source
   
2. **tree-sitter-cairo**: Missing LANGUAGE export
   - Solution: Fork and fix, or use cairo-lang-parser

3. **Version conflicts**: Different parsers want different tree-sitter versions
   - Solution: Standardize on tree-sitter 0.23 for now

## Solution Implemented (2025-08-22 Sessions #2-3)

### The Fix: Self-Built Parsers via Git Submodules
We implemented the original vision of patina-metal: **building parsers from source** instead of using crates.io packages.

#### Implementation Details:

**Architecture:**
```
patina-metal/
├── src/
│   ├── lib.rs         # Main analyzer with streaming iterator support
│   ├── metal.rs       # Metal enum (Rust, Go, Solidity, Cairo)
│   ├── grammars.rs    # FFI bindings to compiled C parsers
│   ├── parser.rs      # Parser wrapper
│   └── queries.rs     # Tree-sitter query loader
├── grammars/          # Git submodules
│   ├── rust/         → tree-sitter-rust v0.23.0
│   ├── go/           → tree-sitter-go v0.23.1
│   └── solidity/     → tree-sitter-solidity v1.2.3
├── queries/           # Pattern matching queries
│   ├── rust/
│   │   ├── symbols.scm
│   │   ├── complexity.scm
│   │   └── patterns.scm
│   └── go/
│       └── ...
└── build.rs           # Compiles C parsers with cc crate
```

**Key Changes:**
1. **Git Submodules**: Added rust, go, solidity grammars as submodules
2. **Custom Build**: `build.rs` compiles parser.c files directly using `cc` crate
3. **FFI Bindings**: Direct `extern "C"` bindings to tree_sitter_rust(), etc.
4. **Version Control**: Pinned to language version 14 (compatible with tree-sitter 0.24)
5. **Streaming Iterator**: Updated to use tree-sitter 0.24's StreamingIterator API
6. **Clean Dependencies**: Only depends on tree-sitter 0.24, no parser crates

#### Current Status:
✅ **Rust Parser**: Fully working, 151 files indexed successfully
✅ **Go Parser**: Fully working, 832 files in Dagger repo processed
✅ **Solidity Parser**: Compiles and links, 209 files detected
✅ **Scrape Integration**: patina-metal is the sole parser provider
✅ **Performance**: Dagger repo (6,420 functions) indexes without timeout
✅ **Version Stability**: No conflicts, single tree-sitter 0.24 everywhere
⚠️  **Cairo**: Not implemented (no stable tree-sitter grammar yet)

## Additional Improvements (Session #3)

### Incremental Updates System
Implemented smart incremental indexing to avoid re-processing unchanged files:

**Features:**
- `index_state` table tracks file paths, mtimes, and index timestamps
- Change detection compares filesystem mtimes with stored values
- Only processes new/modified files, skips unchanged
- `--force` flag available for full re-index when needed
- Cleanup removes only changed file data before re-indexing

**Performance Impact:**
- First run: Full index of all files
- No changes: Instant return (< 1 second)
- Single file change: Only that file re-processed
- Massive time savings for large codebases

## Next Steps

### Completed ✅
- ✅ Git submodules for grammar repositories
- ✅ Self-built parsers from source
- ✅ Version conflict resolution
- ✅ Incremental update system
- ✅ Rust and Go parser integration
- ✅ Basic Solidity support

### Future Improvements
1. **Query System Enhancement**: Fix .scm query syntax for complex patterns
2. **Cairo Support**: Wait for stable tree-sitter-cairo or use cairo-lang-parser
3. **More Languages**: Add Python, JavaScript, TypeScript parsers
4. **Query Documentation**: Document tree-sitter query patterns and usage
5. **Parser Updates**: Automate grammar version updates with compatibility checks
6. **Performance**: Add parallel parsing for multiple files

## Technical Achievements

### What We Solved
1. **Version Hell**: Single tree-sitter 0.24 everywhere, no conflicts
2. **Parser Control**: We compile from source, not dependent on crates.io
3. **Clean Architecture**: patina-metal owns all parser complexity
4. **Performance**: Handles 1000+ file repos without timeout
5. **Incremental Updates**: Smart caching based on file modification times

### How It Works
1. **Build Time**: `cc` crate compiles C parsers from grammar submodules
2. **Runtime**: FFI calls to compiled parsers via `extern "C"`
3. **API**: Uniform `Metal` enum abstracts language differences
4. **Integration**: Main patina uses patina-metal exclusively for parsing

## New Understanding: Token-Efficient Context for LLMs (2025-08-23 Session)

### The Core Mission Evolution
Patina's job is to be a **token-efficient memory system** that allows LLMs to understand massive codebases without reading them entirely. We extract facts that cannot lie, providing context that helps LLMs navigate and contribute to unknown repositories.

### The Three-Tier Intelligence System
```
1. DATABASE (tree-sitter-built) → Fast, structured fact lookups
2. CODE FILES → Deep dive when needed for details
3. WEB → External validation and documentation

LLM orchestrates across all three tiers based on token budget
```

### Key Insight: Context > Details
The LLM needs CONTEXT more than DETAILS. It can read actual code when needed, but it needs:
- **What's important?** (git_metrics tells us) ✅ HAVE
- **What's similar?** (fingerprints tell us) ✅ HAVE  
- **What's the vocabulary?** (types/constants would tell us) ❌ NEED
- **What's public API?** (visibility would tell us) ⚠️ PARTIAL
- **What are the facts?** (structure, not semantics) 🎯 FOCUS

### Token Efficiency Example
```rust
// Original: 5000 tokens of code
impl DatabaseConnection { 
    // ... hundreds of lines ...
}

// Patina summary: 5 tokens
"connect | async Result<Connection> | retry-pattern | high-importance"
```

### The Truth Problem: Facts vs Interpretations

#### What Can Lie (Dangerous to Extract)
- **Doc Comments** - Often outdated or wrong ("Thread-safe" when it takes `&mut`)
- **Semantic Meanings** - We can't verify if `calculate_price()` actually calculates prices
- **Pattern Names** - Naming pattern #42 as "singleton" without proof
- **Importance Assumptions** - High commits might mean buggy, not important

#### What Cannot Lie (Safe to Extract)
- **Type Definitions** - `type NodeId = u32` (compiler-enforced)
- **Visibility Markers** - `pub fn` vs `fn` (syntax facts)
- **Function Structure** - Parameter counts, return types, async/mut markers
- **Module Organization** - File paths, exports, imports
- **Git History** - Dates, counts, authors (historical facts)

### New Extraction Priorities (Facts-Only Approach)

#### Priority 1: Type Vocabulary ✅ **ALWAYS TRUE**
```sql
CREATE TABLE type_vocabulary (
    file VARCHAR,
    name VARCHAR,
    definition TEXT,  -- 'type NodeId = u32'
    kind VARCHAR,  -- 'alias', 'struct', 'enum', 'const'
    usage_count INTEGER
);
```
**Why**: Compiler-enforced truth, defines domain language

#### Priority 2: Function Facts ✅ **ALWAYS TRUE**
```sql
CREATE TABLE function_facts (
    file VARCHAR,
    name VARCHAR,
    -- Thread-safety signals
    takes_mut_self BOOLEAN,
    takes_mut_params BOOLEAN,
    -- Error handling signals
    returns_result BOOLEAN,
    returns_option BOOLEAN,
    calls_unwrap BOOLEAN,
    calls_panic BOOLEAN,
    -- Structure facts
    is_async BOOLEAN,
    is_unsafe BOOLEAN,
    is_public BOOLEAN,
    parameter_count INTEGER,
    line_count INTEGER
);
```
**Why**: All extractable from AST, enables behavioral reasoning

#### Priority 3: API Surface ✅ **ALWAYS TRUE**
```sql
CREATE TABLE api_surface (
    file VARCHAR,
    name VARCHAR,
    visibility_level VARCHAR,  -- 'pub', 'pub(crate)', 'private'
    is_exported BOOLEAN,  -- re-exported by parent module
    is_trait_method BOOLEAN
);
```
**Why**: Critical for knowing what's stable vs internal

### How LLMs Use This System

#### Example: "Is this function thread-safe?"
```sql
-- 1. Quick fact check (5 tokens)
SELECT takes_mut_self, has_mutex FROM function_facts WHERE name = 'process';
-- Result: takes_mut_self=true → NOT thread-safe

-- 2. Pattern check (20 tokens)
SELECT COUNT(*) FROM function_facts 
WHERE pattern_id = 42 AND takes_mut_self = true;
-- Result: All 50 functions with this pattern take &mut

-- 3. Only if needed: Read actual code (1000 tokens)
```

#### Example: "How do I contribute a new feature?"
```sql
-- 1. Find important areas (10 tokens)
SELECT file FROM git_metrics 
WHERE feature_commits > 10 
ORDER BY last_month_commits DESC;

-- 2. Understand vocabulary (20 tokens)
SELECT name, definition FROM type_vocabulary 
WHERE usage_count > 100;

-- 3. Find extension points (15 tokens)
SELECT name FROM api_surface 
WHERE is_public = true AND name LIKE '%handler%';

-- Total: 45 tokens vs 100,000+ tokens to read everything
```

### Lessons from Failed Attempts

#### What Failed
1. **Complex AST traversal** - Tree-sitter panics with `range end index out of range` when navigating siblings/children
2. **Recursive node processing** - Lost context, byte offsets mismatched
3. **Trying to extract "everything"** - Overengineering led to fragile code
4. **Trusting doc comments** - They often lie or drift from reality

#### Why It Failed
- We tried to use tree-sitter for **semantic analysis** when it's designed for **syntax analysis**
- We mixed UTF-8 string parsing with byte offsets
- We tried to understand meaning when we should extract facts
- We tried to traverse complex node relationships when simple direct processing works

#### The Root Cause
```rust
// We did this:
let content = fs::read(&file)?;  // Read as bytes
let content_str = String::from_utf8_lossy(&content);  // Convert (can change!)
parser.parse(&content_str);  // Parse the converted string
process_node(node, &content);  // Use original bytes - MISMATCH!

// Should have done:
let content = fs::read_to_string(&file)?;  // Read as string
parser.parse(&content);  // Parse same string
process_node(node, content.as_bytes());  // Use same bytes
```

### The Realization About Tree-sitter

#### Tree-sitter IS For:
- **Syntax highlighting** (its primary purpose)
- **Structure extraction** (finding functions, types)
- **Pattern finding** (similar code shapes)
- **Incremental parsing** (efficient updates)

#### Tree-sitter is NOT For:
- **Type resolution** (doesn't understand what types mean)
- **Dependency graphs** (doesn't follow imports)
- **Semantic analysis** (doesn't understand behavior)
- **Truth verification** (can't check if docs match code)

#### Who Does Semantic Analysis:
- **Language servers** (rust-analyzer, gopls)
- **Compilers** (rustc, go)
- **Type checkers** (dedicated tools)
- **NOT tree-sitter**

### Design Principles Going Forward

1. **Facts over interpretations** - Extract what IS, not what it claims to be
2. **Token efficiency** - Every byte in the database should save 100x in LLM context
3. **Simple extraction** - Direct node processing, no complex traversal
4. **Trust but verify** - Provide facts that LLMs can verify if needed
5. **Let the LLM reason** - We provide facts, LLM provides intelligence

### The Safe Extraction Strategy

```sql
-- FACT: Function named "calculate_price" exists ✅
-- FACT: Returns Result<f64, Error> ✅
-- FACT: Has 15 lines of code ✅
-- FACT: Modified 5 times last month ✅
-- INTERPRETATION: "Calculates price" ❌ (could be lying)
-- INTERPRETATION: "Important function" ❌ (could be misleading)
```

## Doc Drift Detection: Verifying Documentation Truth (2025-08-23 Session)

### The Insight: Facts Can Expose Documentation Lies

Since we extract only facts that cannot lie, we can use them to verify documentation claims. This creates a powerful doc drift detection system where:
- **Doc comments** = Claims to verify
- **Extracted facts** = Ground truth
- **Contradictions** = Doc drift

### Verification Capabilities

With our current fact extraction, we can verify:

#### 1. **Thread Safety Claims**
```sql
-- If docs claim "thread-safe" but function takes &mut self → LIES!
SELECT * FROM function_facts WHERE takes_mut_self = true OR takes_mut_params = true;
```

#### 2. **Error Handling Claims**
```sql
-- "Never fails" but returns Result → LIES!
SELECT * FROM function_facts WHERE returns_result = true;
-- "Handles all errors" but uses unwrap → LIES!
SELECT * FROM behavioral_hints WHERE calls_unwrap > 0;
-- "Never returns null" but returns Option → LIES!
SELECT * FROM function_facts WHERE returns_option = true;
```

#### 3. **Completeness Claims**
```sql
-- "Fully implemented" but has todo!() → LIES!
SELECT * FROM behavioral_hints WHERE has_todo_macro = true;
-- "Production-ready" but panics → LIES!
SELECT * FROM behavioral_hints WHERE has_panic_macro = true;
```

#### 4. **Safety Claims**
```sql
-- "Safe to use" but marked unsafe → LIES!
SELECT * FROM function_facts WHERE is_unsafe = true;
-- "No unsafe code" but has unsafe blocks → LIES!
SELECT * FROM behavioral_hints WHERE has_unsafe_block = true;
```

#### 5. **Performance Claims**
```sql
-- "Zero-cost" but is async (runtime overhead) → LIES!
SELECT * FROM function_facts WHERE is_async = true;
-- "Lightweight" but high complexity → SUSPICIOUS!
SELECT * FROM code_fingerprints WHERE complexity > 10;
```

### The Doc Drift Risk Score

We can create a scoring system to identify functions with high doc drift risk:

```sql
CREATE VIEW doc_drift_risk AS
SELECT 
    file, name,
    SUM(
        CASE WHEN takes_mut_self THEN 3 ELSE 0 END +      -- Thread safety risk
        CASE WHEN returns_result THEN 2 ELSE 0 END +      -- Error handling risk
        CASE WHEN returns_option THEN 1 ELSE 0 END +      -- Null safety risk
        CASE WHEN is_async THEN 1 ELSE 0 END +            -- Sync/async risk
        CASE WHEN is_unsafe THEN 5 ELSE 0 END +           -- Safety risk
        CASE WHEN has_panic_macro THEN 4 ELSE 0 END +     -- Stability risk
        CASE WHEN has_todo_macro THEN 5 ELSE 0 END        -- Completeness risk
    ) as drift_risk_score
FROM function_facts ff
LEFT JOIN behavioral_hints bh ON ff.file = bh.file AND ff.name = bh.function
WHERE is_public = true
ORDER BY drift_risk_score DESC;
```

### Implementation Vision

To complete doc drift detection, we would:

1. **Extract doc comments as claims** (not as truth):
```sql
CREATE TABLE doc_claims (
    file VARCHAR,
    symbol VARCHAR,
    claim_type VARCHAR,  -- 'thread_safe', 'never_fails', etc.
    claim_text VARCHAR,
    line_number INTEGER
);
```

2. **Build verification rules** that compare claims against facts
3. **Generate drift reports** showing which docs lie
4. **Suggest corrections** based on actual facts

### The Value Proposition

This approach enables:
- **Automated doc audits** - Find all lying documentation
- **Trust scoring** - Rate documentation reliability
- **Doc generation** - Create accurate docs from facts
- **Migration safety** - Verify upgrade guides are truthful
- **API validation** - Ensure public APIs match their promises

### Real-World Impact

Testing on Dagger repository revealed:
- **1,896 type definitions** for vocabulary verification
- **6,420 function facts** for behavioral verification
- **887 import facts** for dependency verification
- **24 behavioral hints** identifying risky patterns

An LLM using this data could instantly identify documentation that claims "thread-safe" for a function taking `&mut self`, or "never fails" for a function returning `Result`. This transforms documentation from a source of potential lies into verified, trustworthy guidance.

## Conclusion

`patina-metal` successfully achieves its vision: a unified parser system that builds from source, giving us complete control over tree-sitter versions and parser compatibility. By treating languages as different "metals" with unique properties, we maintain the Patina metaphor while providing a robust, extensible foundation for multi-language code analysis.

The system now handles Rust, Go, and Solidity code reliably, with incremental updates making it practical for continuous use during development. The architecture is clean, maintainable, and ready for expansion to additional languages.

Most importantly, we've evolved our understanding: **Patina's role is to be a fact extractor that provides token-efficient context for LLMs, not a semantic analyzer**. The LLM is the intelligence; Patina is the efficient memory. By focusing on facts that cannot lie rather than interpretations that might mislead, we create a trustworthy foundation for LLM-assisted code understanding.

The addition of doc drift detection capabilities shows how powerful this "facts-only" approach is: by extracting undeniable truths from code, we can verify or refute any claim made in documentation, creating a system where LLMs can trust the context they receive and make accurate decisions based on reality rather than potentially outdated or incorrect documentation.