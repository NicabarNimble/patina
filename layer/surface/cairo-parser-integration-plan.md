# Cairo Parser Integration Plan

## Objective
Add Cairo 2.x language support to Patina's scrape tool using the official cairo-lang-parser with SimpleParserDatabase, avoiding Salsa complexity.

## Background
Cairo is a language for writing provable programs. Version 2.x has significantly different syntax from earlier versions. Patina needs to parse Cairo files to extract semantic information for the knowledge database.

## Solution: SimpleParserDatabase Approach

### Discovery
The Dojo project demonstrates that cairo-lang-parser can be used WITHOUT the full Salsa database complexity by using `SimpleParserDatabase` from `cairo_lang_parser::utils`.

### Benefits
- ✅ Official Cairo parser - guaranteed correct AST for Cairo 2.x
- ✅ No Salsa query system setup required
- ✅ Direct AST traversal capabilities
- ✅ Production-proven (used by Dojo for macro processing)
- ✅ Fits Patina's vendoring pattern

## Implementation Plan

### Phase 1: Basic Parser Wrapper
Create `patina-metal/src/cairo.rs` module that:

1. **Parser Setup**
   ```rust
   use cairo_lang_parser::utils::SimpleParserDatabase;
   
   pub struct CairoParser {
       db: SimpleParserDatabase,
   }
   ```

2. **String to AST Conversion**
   - Investigate if SimpleParserDatabase has direct string parsing
   - If not, implement string → TokenStream conversion
   - Parse TokenStream to get AST root node

3. **Symbol Extraction Interface**
   ```rust
   pub fn extract_symbols(content: &str) -> Result<CairoSymbols> {
       // Parse content
       // Walk AST
       // Extract functions, structs, traits, imports
   }
   ```

### Phase 2: AST Node Processing
Based on Dojo's pattern, handle these node types:
- `ItemStruct` → struct definitions
- `FunctionWithBody` → function definitions
- `ItemTrait` → trait definitions
- `ItemImpl` → impl blocks
- `ItemModule` → module definitions
- `ItemUse` → import statements

### Phase 3: Language Registry Integration

1. **Add Cairo to Metal enum**
   ```rust
   impl Metal {
       Cairo => Some(Language::Cairo),
   }
   ```

2. **Create LanguageSpec for Cairo**
   ```rust
   const CAIRO_SPEC: LanguageSpec = LanguageSpec {
       extensions: &[".cairo"],
       function_nodes: &["function_with_body"],
       struct_nodes: &["item_struct"],
       trait_nodes: &["item_trait"],
       import_nodes: &["item_use"],
       // ... extraction functions
   };
   ```

3. **Wire into scrape/code.rs**
   - Add Cairo to supported languages check
   - Use cairo.rs module for Cairo files
   - Map extracted symbols to common format

## Technical Considerations

### Dependencies
Add to `patina-metal/Cargo.toml`:
```toml
cairo-lang-parser = "2.x"
cairo-lang-syntax = "2.x"
cairo-lang-filesystem = "2.x"  # If needed for TokenStream
```

### Error Handling
- Parse diagnostics from cairo-lang-parser
- Graceful fallback if parsing fails
- Clear error messages for unsupported Cairo syntax

### Testing Strategy
1. Unit tests with Cairo code snippets
2. Integration test with real Cairo project
3. Verify symbol extraction accuracy
4. Performance benchmarks

## Success Criteria
- [x] Parse Cairo 2.x files without Salsa setup
- [ ] Extract functions, structs, traits, imports
- [ ] Integrate seamlessly with language registry
- [ ] Maintain vendoring pattern
- [ ] Pass all tests with Cairo files

## Next Steps
1. Create feature branch `cairo-simple-parser`
2. Add cairo-lang dependencies to patina-metal
3. Implement basic parser wrapper
4. Test with sample Cairo files
5. Integrate into language registry
6. Add comprehensive tests

## References
- Dojo's usage: Shows SimpleParserDatabase pattern
- cairo-lang-parser docs: Official parser documentation
- Patina language registry: src/commands/scrape/code.rs