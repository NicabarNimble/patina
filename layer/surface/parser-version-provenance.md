# Parser Version Provenance System

## Problem Statement

We're constrained by the lowest common denominator for tree-sitter compatibility. Currently:
- Solidity uses tree-sitter 0.20.10 (language version 14)
- Most grammars use 0.24 (language version 14)
- C grammar wants 0.25.4 (language version 15)
- Cairo bypasses tree-sitter entirely with a custom parser

This creates a version pinning problem where we can't upgrade to support newer grammars without breaking older ones.

## Current Architecture Analysis

### Version Landscape
```
Grammar          | TS Version | Lang Version | Status
-----------------|------------|--------------|--------
Solidity         | 0.20.10    | 14          | Working (anchor)
Rust             | 0.23       | 14          | Working
Go               | 0.23       | 14          | Working
Python           | 0.24       | 14          | Working
JavaScript       | 0.24       | 14          | Working
TypeScript       | 0.24       | 14          | Working
C                | 0.25.4     | 15          | BROKEN
C++              | 0.24       | 14          | Working
Cairo            | N/A        | N/A         | Custom parser
```

### The Cairo Solution
Cairo demonstrates an escape hatch: when tree-sitter versioning becomes problematic, use an alternative parser. This works because:
1. The scraper abstracts over parser implementations
2. The Metal enum doesn't care how parsing happens
3. Results are normalized to a common schema (SQLite tables)

## Proposed Solution: Version Provenance Tracking

### 1. Grammar Metadata Table
Track provenance for each grammar in SQLite:

```sql
CREATE TABLE grammar_provenance (
    language TEXT PRIMARY KEY,
    parser_type TEXT, -- 'tree-sitter' or 'custom'
    
    -- What the grammar wants
    grammar_version TEXT,
    required_ts_version TEXT,
    required_lang_version INTEGER,
    
    -- What we're actually using
    actual_ts_version TEXT,
    actual_lang_version INTEGER,
    
    -- Compatibility assessment
    compatibility_status TEXT, -- 'compatible', 'degraded', 'broken'
    compatibility_notes TEXT,
    
    -- Alternative parser info
    alt_parser_available BOOLEAN,
    alt_parser_type TEXT, -- e.g., 'cairo-lang-parser', 'rust-analyzer', etc.
    
    -- Tracking
    last_checked TIMESTAMP,
    last_working_version TEXT
);
```

### 2. Multi-Parser Strategy

```rust
pub enum ParserBackend {
    TreeSitter {
        version: String,
        language_version: u32,
    },
    Custom {
        name: String,
        version: String,
    },
}

impl Metal {
    pub fn parser_backend(&self) -> ParserBackend {
        match self {
            Metal::Cairo => ParserBackend::Custom {
                name: "cairo-lang-parser".to_string(),
                version: "2.4.0".to_string(),
            },
            Metal::C if self.tree_sitter_incompatible() => {
                // Could use alternative C parser
                ParserBackend::Custom {
                    name: "clang-ast".to_string(),
                    version: "15.0".to_string(),
                }
            },
            _ => ParserBackend::TreeSitter {
                version: "0.24".to_string(),
                language_version: 14,
            }
        }
    }
}
```

### 3. Compatibility Matrix

Build a compatibility check that runs on `patina doctor`:

```rust
pub fn check_grammar_compatibility() -> Result<CompatibilityReport> {
    let mut report = CompatibilityReport::new();
    
    for metal in Metal::all() {
        let status = match metal.parser_backend() {
            ParserBackend::TreeSitter { .. } => {
                // Try to parse a simple test file
                test_tree_sitter_grammar(metal)?
            },
            ParserBackend::Custom { .. } => {
                // Check if custom parser is available
                test_custom_parser(metal)?
            }
        };
        
        report.add(metal, status);
    }

    // Write to SQLite for tracking
    report.persist_to_db()?;
    
    Ok(report)
}
```

### 4. Escape Hatches by Language

#### Option A: Dual Parser Support
Support both tree-sitter and alternative parsers, choosing based on availability:

```rust
pub fn parse_file(path: &Path, content: &str) -> Result<ParsedSymbols> {
    let metal = Metal::from_path(path)?;
    
    match metal.parser_backend() {
        ParserBackend::TreeSitter { .. } if metal.tree_sitter_works() => {
            parse_with_tree_sitter(metal, content)
        },
        _ => {
            // Fall back to custom parser
            parse_with_custom(metal, content)
        }
    }
}
```

#### Option B: Language-Specific Parsers
- **C/C++**: Use `clang` AST dump or `cppast`
- **Rust**: Use `rust-analyzer` or `syn`
- **Go**: Use `go/parser` package via subprocess
- **Python**: Use Python's `ast` module
- **JavaScript/TypeScript**: Use `@babel/parser` or `swc`
- **Solidity**: Keep tree-sitter (it works) or use `solc --ast`

### 5. Migration Path

1. **Phase 1**: Add provenance tracking (non-breaking)
   - Create grammar_provenance table
   - Log version info during scraping
   - Add `patina doctor --grammars` command

2. **Phase 2**: Add alternative parsers (additive)
   - Start with C (it's broken anyway)
   - Add clang-based parser as experiment
   - Keep tree-sitter as fallback

3. **Phase 3**: Smart parser selection
   - Choose parser based on compatibility
   - Prefer tree-sitter when it works
   - Use alternatives when needed

## Benefits

1. **No More Version Lock**: Each language can use its best parser
2. **Graceful Degradation**: If tree-sitter breaks, fall back to alternatives
3. **Better Coverage**: Can support languages tree-sitter doesn't handle well
4. **Future Proof**: New languages can bring their own parsers
5. **Provenance Tracking**: Know exactly what versions are in use

## Implementation Notes

- Keep tree-sitter as primary when possible (it's fast and uniform)
- Alternative parsers only for problematic languages
- All parsers must output normalized schema
- Version info becomes part of the knowledge database
- Could eventually support multiple parsers per language for comparison

## Example: Fixing C Support

Instead of downgrading all grammars to support C's tree-sitter v15:

```rust
impl Metal {
    pub fn parse(&self, content: &str) -> Result<Symbols> {
        match self {
            Metal::C => {
                // C grammar needs tree-sitter v15, we have v14
                // Use clang instead
                parse_c_with_clang(content)
            },
            _ => {
                // Use tree-sitter for everything else
                self.parse_with_tree_sitter(content)
            }
        }
    }
}
```

This way we keep fast tree-sitter parsing for most languages while escaping to alternatives when needed.