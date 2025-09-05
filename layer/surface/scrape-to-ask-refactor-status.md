---
id: scrape-to-ask-refactor-status
status: completed
created: 2025-09-05
completed: 2025-09-05
tags: [refactoring, scrape, ask, pattern-detection]
---

# Scrape to Ask Command Refactoring Status

## Summary
Moving pattern detection from `scrape` command to `ask` command to separate data extraction from analysis.

## ✅ REFACTORING COMPLETED

### Summary of Changes
- **Removed**: 322 lines of pattern detection code from `src/commands/scrape/code.rs`
- **Pattern module deleted**: Lines 3116-3437 removed completely
- **Fixed references**: Removed `patterns::generate_schema()` call
- **Fixed warnings**: Cleaned up unused imports in ask command
- **Tested both commands**: Scrape extracts data, ask discovers patterns
- **Committed**: b9e257d - "refactor: complete separation of scrape and ask commands"

## What Was Completed

### 1. ✅ Created Ask Command Structure
- Created `src/commands/ask/mod.rs` - Command entry point
- Created `src/commands/ask/patterns.rs` - Pattern analysis from database
- Registered command in `src/main.rs` and `src/commands/mod.rs`

### 2. ✅ Ask Command Working
```bash
# Successfully discovers patterns from data:
patina ask "naming patterns" --repo SDL
# Found: SDL_ (3493), HIDAPI_ (579), VULKAN_ (292) - all discovered, not hardcoded!

patina ask "conventions"
# Shows error handling patterns, async usage - from actual data
```

### 3. ✅ Partially Cleaned Scrape Command
**Removed from `src/commands/scrape/code.rs`:**
- Lines 1501-1506: Pattern accumulator initialization
- Lines 1524-1530, 1607-1613: Pattern detection calls
- Lines 1686-1746: Pattern SQL generation and insertion
- Updated doc comments to remove pattern table references

## What Still Needs Doing

### 1. ✅ Remove Pattern Module (Lines 3109-3484)
The entire `pub(crate) mod patterns` module needs to be deleted from code.rs:
- Starts at line ~3109
- Ends at line ~3484
- Contains all the hardcoded pattern detection logic
- Includes structs: NamingPatterns, ArchitecturalPatterns, CodebaseConventions
- Contains functions: detect_naming_patterns, extract_prefix (with hardcoded list), etc.

**Issue encountered**: Module boundaries got confused with languages module. Need to carefully remove just the patterns module content between these markers:
```rust
pub(crate) mod patterns {
    // ... 370+ lines of pattern detection code ...
}
```

### 2. ✅ Remove Pattern Table Creation
The SQL schema creation and references have been completely removed.

## How to Complete the Refactoring

### Step 1: Remove the patterns module
```bash
# The patterns module is approximately lines 3109-3484
# Delete everything between:
# pub(crate) mod patterns {
# and its closing }
```

### Step 2: Fix any compilation errors
After removing the patterns module, there may be references that need cleaning:
- Any remaining `patterns::` calls
- Any imports of pattern types

### Step 3: Test
```bash
# Build and test
cargo build
cargo test

# Run scrape - should work without pattern detection
patina scrape code --force

# Run ask - should discover patterns from data
patina ask "naming patterns"
```

## Current Git Status
Multiple changes in progress:
- `src/commands/scrape/code.rs` - Partially cleaned
- `src/commands/ask/mod.rs` - New file (working)
- `src/commands/ask/patterns.rs` - New file (working)
- `src/commands/mod.rs` - Updated
- `src/main.rs` - Updated

## Design Decision Rationale

### Why This Change?
1. **Separation of Concerns**: `scrape` should only extract facts, not analyze them
2. **Fix Broken Inference**: Pattern detection was hardcoded and wrong (claimed "Option-preferred" for C code)
3. **Enable Adaptive Discovery**: `ask` can discover actual patterns from data (found SDL_, gtk_, etc.)
4. **Simplify code.rs**: Remove ~400 lines of pattern detection code

### Architecture
```
scrape: Extract facts → Store in DuckDB → Done
ask:    Query DuckDB → Discover patterns → Answer questions
```

## Next Session Actions
1. Complete removal of patterns module from code.rs
2. Test both commands work correctly
3. Commit with clear message about the refactoring
4. Update main design doc to mark this as complete