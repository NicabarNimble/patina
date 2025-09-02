# Dual Tree-Sitter Implementation Plan

## Goal
Fix C language support by running tree-sitter 0.25 alongside 0.24.

## Current Situation
- Main project uses tree-sitter 0.24 (supports language version 14)
- C grammar requires language version 15 (needs tree-sitter 0.25)
- C is currently broken

## Implementation Steps

### 1. Update Cargo.toml Dependencies

In root `Cargo.toml`:
```toml
[dependencies]
tree-sitter = "0.24"  # Keep for existing grammars
tree-sitter-v25 = { package = "tree-sitter", version = "0.25" }
```

In `patina-metal/Cargo.toml`:
```toml
[dependencies]
tree-sitter = "0.24"
tree-sitter-v25 = { package = "tree-sitter", version = "0.25" }
```

### 2. Update Metal Parser Selection

In `patina-metal/src/metal.rs`:
```rust
use tree_sitter::Language as TSLanguage;
use tree_sitter_v25::Language as TSLanguage25;

impl Metal {
    /// Returns true if this grammar uses language version 15
    pub fn uses_v15(&self) -> bool {
        matches!(self, Metal::C)
    }
    
    /// Get the tree-sitter language for v14 grammars
    pub fn tree_sitter_language(&self) -> Option<TSLanguage> {
        if self.uses_v15() {
            return None;
        }
        // existing implementation
    }
    
    /// Get the tree-sitter language for v15 grammars
    pub fn tree_sitter_language_v15(&self) -> Option<TSLanguage25> {
        match self {
            Metal::C => Some(unsafe { TSLanguage25::from_raw(grammars::language_c_raw()) }),
            _ => None,
        }
    }
}
```

### 3. Update Analyzer to Support Both Versions

In `patina-metal/src/lib.rs`:
```rust
use tree_sitter::{Parser, Tree};
use tree_sitter_v25::{Parser as Parser25, Tree as Tree25};

pub struct Analyzer {
    parsers: HashMap<Metal, Parser>,
    parsers_v15: HashMap<Metal, Parser25>,
    // ... existing fields
}

impl Analyzer {
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();
        let mut parsers_v15 = HashMap::new();
        
        for metal in Metal::all() {
            if metal.uses_v15() {
                // Use v25 parser for C
                if let Some(language) = metal.tree_sitter_language_v15() {
                    let mut parser = Parser25::new();
                    parser.set_language(&language)?;
                    parsers_v15.insert(metal, parser);
                }
            } else {
                // Use v24 parser for everything else
                if let Some(language) = metal.tree_sitter_language() {
                    let mut parser = Parser::new();
                    parser.set_language(&language)?;
                    parsers.insert(metal, parser);
                }
            }
        }
        
        Ok(Self { parsers, parsers_v15, ... })
    }
    
    pub fn parse(&mut self, metal: Metal, source: &str) -> Result<ParsedFile> {
        if metal.uses_v15() {
            let parser = self.parsers_v15.get_mut(&metal)
                .ok_or_else(|| anyhow!("No v15 parser for {:?}", metal))?;
            let tree = parser.parse(source, None)
                .ok_or_else(|| anyhow!("Failed to parse"))?;
            // Convert Tree25 to our ParsedFile type
        } else {
            // Existing v14 parsing logic
        }
    }
}
```

### 4. Update C Grammar Build

In `patina-metal/grammars/c/Cargo.toml`:
```toml
[dependencies]
tree-sitter = "0.25"  # C needs v25
tree-sitter-language = "0.1"
```

### 5. Test C Parsing

Create test file:
```rust
#[test]
fn test_c_parsing_works() {
    let analyzer = Analyzer::new().unwrap();
    let c_code = r#"
        int main() {
            return 0;
        }
    "#;
    
    let result = analyzer.parse(Metal::C, c_code);
    assert!(result.is_ok());
}
```

## Summary
This minimal change:
1. Adds tree-sitter 0.25 as a separate dependency
2. Routes C through the v25 parser
3. Keeps all other languages on v24
4. Makes C work without breaking anything else