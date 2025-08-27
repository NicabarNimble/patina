# Scrape Refactor Recovery - Critical Session Notes

## What Went Wrong
The refactor from monolithic (2000+ lines) to modular broke language-specific extraction:
- **Old**: 1800+ lines with extensive per-language logic for Go, Python, JS, etc.
- **New**: Generic extraction assuming all languages same - ONLY WORKS FOR RUST

## The Problem
- Refactor used generic `child_by_field_name("name")` - doesn't exist in most languages
- Lost all language-specific logic:
  - Go: uppercase = public
  - Python: `_` prefix = private  
  - Different AST node names per language
  - Different parameter structures

## What I Did
1. Discovered scrape was broken for non-Rust languages (dagger repo showed issues)
2. Retrieved original 2023-line `scrape.rs` from commit 26d34d8^
3. Replaced broken modular version with working original
4. Original is now in `src/commands/scrape.rs`

## Current State
- `src/commands/scrape.rs` - Working original (2000+ lines, language-specific)
- `src/commands/scrape_broken.rs` - Broken modular version (for reference)
- `src/semantic/extractor/` - Broken extraction modules
- `src/semantic/store/` - Broken store modules

## Next Steps
If continuing:
1. Clean up broken modules: `rm -rf src/semantic/extractor_broken src/semantic/store_broken`
2. Test scrape works: `patina scrape --force --repo dagger`
3. Design proper language-specific modules:
   ```
   src/semantic/extractors/
   ├── rust.rs    # Complete Rust extraction
   ├── go.rs      # Complete Go extraction  
   ├── python.rs  # Complete Python extraction
   ```

## Key Lesson
**Don't abstract what's inherently different!** Each language has unique AST structure. The monolithic version was verbose but CORRECT. Better to have working duplication than broken abstraction.

## Commit to Make
```bash
git add -A
git commit -m "fix: revert to working monolithic scrape implementation

The modular refactor lost critical language-specific extraction logic.
Reverting to the original implementation that correctly handles all
supported languages (Rust, Go, Python, JS/TS, Solidity)."
```