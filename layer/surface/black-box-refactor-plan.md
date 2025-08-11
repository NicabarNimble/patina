---
id: black-box-refactor-plan
status: active
created: 2025-08-10
tags: [refactor, implementation-plan, dependable-rust, black-box]
---

# Black-Box Refactor: The Actual Plan

## The Current Mess

We created "refactored" modules that are just wrappers calling the original messy code. This is NOT a refactor - it's theater. The original code is still there, still messy, still not dependable.

## What We're Actually Doing

Taking non-dependable code and making it ACTUALLY dependable - not just wrapping it.

## The Step-by-Step Plan

### Step 1: Fix claude_refactored (Start Here)
```bash
# Location: src/adapters/claude_refactored/
```

1. **Copy ALL code from `claude.rs` into `implementation.rs`**
   - Yes, all 900+ lines
   - Don't think about it, just copy it

2. **Fix visibility in implementation.rs**
   - Change all `pub` to `pub(super)` 
   - Make helper functions private
   - Only expose to parent module

3. **Update mod.rs to use the local implementation**
   - Remove any references to `crate::adapters::claude`
   - Use `implementation::ClaudeAdapter` instead

4. **Test it works**
   ```bash
   USE_REFACTORED_CLAUDE=true cargo test
   USE_REFACTORED_CLAUDE=true cargo build
   ```

5. **Delete the original**
   - Delete `src/adapters/claude.rs` completely
   - If anything breaks, fix the imports

### Step 2: Fix indexer_refactored
```bash
# Location: src/indexer_refactored/
```

1. **Copy ALL indexer code into implementation.rs**
   - From `src/indexer/*.rs`
   - Include NavigationMap, Database, state_machine, all of it

2. **Make it private**
   - All types become private except Pattern, Location, Confidence
   - All functions become `pub(super)` or private

3. **Test and delete original**
   ```bash
   USE_REFACTORED_INDEXER=true cargo test
   rm -rf src/indexer/
   ```

### Step 3: Fix init_refactored
```bash
# Location: src/commands/init_refactored/
```

1. **Copy init command logic**
   - From `src/commands/init/mod.rs`
   - Into `implementation.rs`

2. **Hide everything except execute()**
   - Public API: `pub fn execute(args: InitArgs) -> Result<()>`
   - Everything else private

3. **Test and delete**
   ```bash
   USE_REFACTORED_INIT=true cargo run -- init test-project
   rm -rf src/commands/init/
   ```

### Step 4: Fix navigate_refactored
```bash
# Location: src/commands/navigate_refactored/
```

Same pattern as init:
1. Copy all navigate code
2. Hide everything except execute()
3. Test and delete original

### Step 5: Clean Up The Mess

1. **Remove all environment switching**
   ```bash
   # Delete these functions from src/config.rs:
   - use_refactored_claude()
   - use_refactored_indexer()
   - use_refactored_init()
   - use_refactored_navigate()
   - use_refactored_workspace()
   ```

2. **Rename all modules (remove _refactored suffix)**
   ```bash
   mv src/adapters/claude_refactored src/adapters/claude
   mv src/indexer_refactored src/indexer
   # etc...
   ```

3. **Update all imports**
   - Search and replace `claude_refactored` → `claude`
   - Search and replace `indexer_refactored` → `indexer`
   - etc.

4. **Delete config.rs if it's only switching logic**

### Step 6: Final Verification

```bash
cargo clean
cargo build --release
cargo test --workspace
cargo clippy -- -D warnings
```

## The Rules (DO NOT BREAK THESE)

1. **One module at a time** - Finish claude completely before touching indexer
2. **Copy everything** - Don't try to be clever, just copy it all
3. **Hide everything** - If it's not in the public API, it's private
4. **Delete originals immediately** - No keeping both versions
5. **Don't optimize** - Just make it work with the same functionality

## Success Criteria

When done, each module should have:
- [ ] A `mod.rs` with <150 lines of public API
- [ ] An `implementation.rs` with ALL the actual code (can be 1000+ lines)
- [ ] NO references to original modules
- [ ] NO environment variable switching
- [ ] Tests passing

## Why This Is Right

This is what Eskil means by "dependable": 
- One module does one job
- The implementation is hidden
- The API is minimal and stable
- Once it works, you never touch it again

The 900-line implementation.rs isn't "messy" - it's HIDDEN. Nobody needs to know what's in there.

## Start Now

Start with Step 1. Don't overthink it. Just copy the code and hide it.