# Cairo Integration Learnings

## Summary
Attempted to add Cairo language support to Patina's scrape tool. Discovered architectural challenges and multiple potential approaches.

## Current State

### What Exists
1. **patina-metal** - Vendors tree-sitter grammars for all supported languages
2. **Metal enum** - Already includes Cairo, but returns `None` for tree-sitter language
3. **Vendoring pattern** - All parsers are vendored in `patina-metal/grammars/`
4. **Scrape flow** - Uses tree-sitter ASTs → extracts data → generates SQL

### Cairo Ecosystem (August 2025)
- **Current version**: Cairo 2.8.0 (major version coming September 2025)
- **Scarb**: Package manager that bundles Cairo compiler (version 2.5.4 installed)
- **cairo-lang-parser**: Rust crate for parsing Cairo (version 2.12)
- **Tree-sitter-cairo**: Outdated (last updated 2022-2023, doesn't support Cairo 2.x)

## Approaches Explored

### 1. Tree-sitter-cairo (❌ Not viable)
- Existing grammars are outdated (Cairo 0.x or 1.x)
- Cairo 2.x has significantly different syntax
- Would need major work to update grammar

### 2. cairo-lang-parser Crate (⚠️ Complex)
**Challenges:**
- Uses `rust-analyzer-salsa` not standard `salsa`
- Complex database initialization with salsa
- AST types are different from tree-sitter (enum variants not structs)
- Heavy dependencies (~14 crates)

**Attempted:**
```rust
// Requires complex setup
#[salsa::database(FilesDatabase, SyntaxDatabase, ParserDatabase)]
pub struct CairoParserDb {
    storage: salsa::Storage<Self>,
}
```

### 3. Simple Regex Parser (🤔 Works but limited)
- Can extract basic symbols (functions, structs, traits, imports)
- Misses complex semantic information
- Not a proper AST, just pattern matching
- Goes against the goal of proper parsing

### 4. Scarb Integration (🔍 Unexplored)
- Scarb has the Cairo compiler built-in
- Could potentially shell out to Scarb for analysis
- Has metadata command but unclear if it exposes AST

## Key Learnings

### Architecture Insights
1. **Vendoring is important** - All languages are vendored, Cairo should be too
2. **Tree-sitter isn't universal** - Some languages need different parsers
3. **Interface consistency matters** - Cairo should look like other languages to scrape code

### Technical Challenges
1. **Parser API complexity** - cairo-lang-parser has a complex salsa-based API
2. **Version mismatches** - Different salsa versions cause conflicts
3. **AST differences** - Cairo AST structure differs significantly from tree-sitter

### Pattern Recognition
When I hit obstacles, I tend to:
1. Start with the right approach (proper parser integration)
2. Hit complexity
3. **Fall back to simple solution (regex) instead of pushing through** ⚠️
4. This defeats the purpose of proper semantic analysis

## Potential Solutions

### Option A: Wrapper Pattern
Create a Cairo module in patina-metal that:
- Uses cairo-lang-parser internally
- Exposes a common interface matching tree-sitter
- Returns extracted symbols in same format as other languages

### Option B: Scarb Integration
- Use Scarb as external tool
- Shell out for parsing/analysis
- Parse Scarb's output

### Option C: Custom Parser
- Build a proper Cairo 2.x tree-sitter grammar
- Significant work but fits existing architecture perfectly
- Most maintainable long-term

### Option D: Hybrid Approach
- Start with simple extraction for basic support
- Add proper parser later
- At least gets Cairo files indexed

## Questions to Resolve

1. **Parser strategy**: Should we use cairo-lang-parser, Scarb, or build tree-sitter-cairo?
2. **Vendoring approach**: How to vendor a non-tree-sitter parser?
3. **Interface design**: How to make Cairo work seamlessly with existing flow?
4. **Maintenance**: Who maintains the Cairo parser as language evolves?

## Recommendation

The cleanest approach would be:
1. Create proper wrapper around cairo-lang-parser in patina-metal
2. Handle the salsa database complexity once, properly
3. Expose same interface as tree-sitter languages
4. This maintains vendoring pattern and consistency

But we need to decide if the complexity is worth it versus waiting for/building a proper tree-sitter-cairo grammar.