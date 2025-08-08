---
id: command-structure-redesign
version: 2
status: active
created_date: 2025-08-06
oxidizer: nicabar
references: [core/unix-philosophy.md, core/progressive-disclosure.md]
tags: [architecture, cli, commands, ux, refactoring]
---

# Command Structure Redesign for Patina

A minimal redesign to fix the confusing overlap between init, update, and setup.sh by making init truly universal.

## Executive Summary

Current Patina has confusing command overlap. The fix is simple: enhance `init` to handle all initialization scenarios, create `upgrade` for Patina binary updates, and simplify `doctor` to diagnosis only.

**Status**: Core functionality implemented. Dev tooling and cleanup remaining.

**Last Updated**: 2025-08-06

**Reality Check**: 
- ✅ Init handles both new projects AND refreshing existing ones
- ✅ Init automatically detects and updates outdated components
- ✅ Component update logic fully working with changelogs
- ⚠️  Upgrade exists but still uses mock data
- ⚠️  Dev commands exist but most are skeleton implementations  
- ⚠️  Update command still present but can now be removed

## The Problem

```
setup.sh    → Installs tools, creates PROJECT_DESIGN.toml
patina init → Dies if no PROJECT_DESIGN.toml exists  
patina update → Ambiguous: updates patina or refreshes environment?
```

## The Solution

```
patina
├── init [name|.]   # Universal initialization (handles ALL scenarios)
├── upgrade         # Check for new Patina CLI versions
├── doctor          # Diagnose project health (no auto-fix)
└── dev             # Developer commands (only with --features dev)
    ├── update      # Update adapters and components
    ├── validate    # Validate resources
    └── release     # Prepare releases
```

## Key Implementation Details

### Smart Re-initialization Detection
```rust
// Automatically detects existing projects
let is_reinit = if name == "." {
    Path::new(".patina").exists() || Path::new("PROJECT_DESIGN.toml").exists()
} else {
    false
};

if is_reinit {
    // Check for component updates
    // Show changelogs
    // Update if user approves
}
```

### Component Update Flow
1. Load existing version manifest (`.patina/versions.json`)
2. Call `UpdateChecker::check_for_updates()`
3. Show available updates with changelogs
4. Re-run adapter initialization to update files
5. Update version manifest with new versions

### No --refresh Flag Needed!
The original design mentioned `--refresh` but the implementation is smarter:
- `patina init .` automatically detects it's a re-init
- Checks and offers updates seamlessly
- One command, intelligent behavior

## Implementation: Enhance Init

The current `init` command is 90% complete. Add three core features:

### 1. PROJECT_DESIGN.toml Creation
```rust
// Current: Dies if missing
if !Path::new(&design).exists() {
    println!("Cannot initialize without PROJECT_DESIGN.toml");
    std::process::exit(1);
}

// Enhanced: Offer to create
if !Path::new(&design).exists() {
    println!("No PROJECT_DESIGN.toml found.");
    if confirm("Create one interactively? [Y/n]") {
        create_project_design_wizard()?;  // Port from setup/bootstrap.rs
    }
}
```

### 3. Tool Installation
```rust
// Current: Just displays what's detected
for (tool, info) in &environment.tools {
    println!("  ✓ {}: {}", tool, info.version);
}

// Enhanced: Offer to install missing
let missing = detect_missing_tools(&environment, &design_toml);
if !missing.is_empty() {
    println!("Missing tools: {}", missing.join(", "));
    if confirm("Install? [Y/n]") {
        install_tools(missing)?;  // Port from setup/bootstrap.rs
    }
}
```

## How It Works Now

### New Project
```bash
patina init myproject --llm claude
# Creates PROJECT_DESIGN.toml interactively if missing
# Installs missing tools if desired
# Sets up all adapter files
# Initializes navigation database and indexes patterns
```

### Existing Project (Auto-Refresh!)
```bash
cd existing-project && patina init .
# 🔄 Re-initializing Patina project...
# 🔍 Checking for component updates...
# 🔍 Reindexing patterns for navigation...
# 
# 📦 Component updates available:
#   • claude-adapter: 0.4.0 → 0.5.0
# 
#   What's new in Claude adapter:
#     - Enhanced session-update with time-span tracking
#     - Fixed: Scripts now properly stored in .claude/bin/
# 
# Update components to latest versions? [Y/n]
```

### Join Team Project
```bash
git clone repo && cd repo && patina init .
# Detects existing PROJECT_DESIGN.toml
# Checks and updates components to match manifest  
# Rebuilds navigation index with latest patterns
# Ready to work and discover wisdom!
```

## Implementation Steps

### Code Changes

### ✅ Completed: Core Functionality (Phase 1)

1. **Enhanced init command**
   - [x] Add `create_project_design_wizard()` function
   - [x] Add `install_tools()` function for each platform
   - [x] Add `confirm()` utility for user prompts
   - [x] Make init idempotent (safe to run multiple times)
   - [x] **NEW: Automatic refresh on re-init (no flag needed!)**
   - [x] **NEW: Check version manifests for existing projects**
   - [x] **NEW: Call UpdateChecker to find outdated components**
   - [x] **NEW: Update adapter files when newer versions available**
   - [x] **NEW: Show component changelogs during updates**
   - [x] **NEW: Make component updates part of init workflow**

2. **Created upgrade command**
   - [x] Add `upgrade` command to CLI enum
   - [x] Mock version checking implementation
   - [x] Display upgrade instructions
   - [x] JSON output support

3. **Simplified doctor**
   - [x] Remove --fix flag from doctor command
   - [x] Focus on diagnostics only
   - [x] Keep health checks, removed repair logic

4. **Dev tooling structure**
   - [x] Add `dev` feature flag to Cargo.toml
   - [x] Create Dev subcommand with feature gate
   - [x] Move update command under `patina dev update`
   - [x] Add dev commands (validate, release, sync-adapters, etc.)

### ⚠️ IN PROGRESS: Cleanup & Polish

1. **Navigation Integration**
   - [ ] Add navigation database initialization to init command
   - [ ] Implement pattern indexing during init
   - [ ] Add reindexing on `patina init .` for existing projects
   - [ ] Show newly discovered patterns after indexing
   - [ ] Add navigation health check to doctor command

2. **Remove update command**
   - [ ] Add final deprecation warning to update command
   - [ ] Remove Commands::Update from main.rs
   - [ ] Delete src/commands/update.rs
   - [ ] Update all documentation references

3. **Complete upgrade command**
   - [ ] Replace mock with actual GitHub API calls
   - [ ] Use reqwest or ureq for HTTP requests
   - [ ] Parse GitHub releases API response
   - [ ] Add proper semver comparison
   - [ ] Consider caching to avoid rate limits

4. **Deprecate and remove update**
   - [ ] After init --refresh works, add deprecation warning
   - [ ] Update all documentation
   - [ ] Remove in next major version

### ❌ TODO: Developer Tool Implementation

1. **Complete sync-adapters command**
   - [ ] Actually read files from resources/
   - [ ] Compare versions with upstream/templates
   - [ ] Update files instead of just printing
   - [ ] Handle all adapter types
   - [ ] Add --check mode for CI

2. **Complete bump-version command**
   - [ ] Update version constants in all files:
     - [ ] src/adapters/claude.rs CLAUDE_ADAPTER_VERSION
     - [ ] src/adapters/gemini.rs GEMINI_ADAPTER_VERSION
     - [ ] Cargo.toml version field
   - [ ] Update version manifests
   - [ ] Create git tags when bumping
   - [ ] Update CHANGELOG.md template

3. **Complete update-fixtures command**
   - [ ] Generate real test fixtures from current state
   - [ ] Update fixtures for:
     - [ ] PROJECT_DESIGN.toml examples
     - [ ] Environment detection results
     - [ ] Version manifest formats
     - [ ] CLAUDE.md output
   - [ ] Validate fixtures work in tests

### ❌ TODO: Documentation Updates

- [ ] Update README.md to show new command structure
- [ ] Remove all references to setup.sh
- [ ] Document dev feature flag usage
- [ ] Add migration guide from update to init --refresh
- [ ] Update all examples to use new commands
- [ ] Document dev workflow for contributors

## Feature Flag Implementation

### Building Patina

**For Users** (default - no dev commands):
```bash
cargo install patina
# or
cargo build --release
```

**For Developers** (includes dev commands):
```bash
cargo build --release --features dev
# or
cargo install --path . --features dev
```

### Code Structure

```rust
// Only included when built with --features dev
#[cfg(feature = "dev")]
Commands::Dev {
    #[command(subcommand)]
    command: DevCommands,
}
```

This ensures:
- User binaries stay lean
- Dev functionality is completely absent unless explicitly built
- Clear separation of concerns
- Follows Rust ecosystem patterns

## Implementation Roadmap

### ✅ Phase 1: Make Init Complete (DONE!)
1. ~~Add `--refresh` flag to init~~ → Better: automatic detection!
2. ✅ Port UpdateChecker logic to init
3. ✅ Enable init to update existing adapter files
4. ✅ Test thoroughly with existing projects
5. ✅ Show changelogs for component updates
6. ✅ Handle version.json vs versions.json inconsistency

### 🚧 Phase 2: Clean Up Update Command (Priority: HIGH)
1. Add deprecation warning pointing to `patina init .`
2. Remove Commands::Update variant
3. Delete src/commands/update.rs
4. Update README and docs
5. Remove update references from help text

### 📝 Phase 3: Complete Dev Commands (Priority: MEDIUM)
1. Implement real file updates in sync-adapters
2. Complete version bumping across all files
3. Generate real test fixtures
4. Add integration tests for dev workflow

### 🔄 Phase 4: Finish Upgrade Command (Priority: MEDIUM)
1. Replace mock with GitHub API
2. Add proper error handling
3. Consider self-update mechanism
4. Add version caching

### 🎯 Phase 5: Polish & Ship (Priority: HIGH)
1. Update all documentation
2. Create migration guide for users
3. Test on real projects
4. Release v0.2.0 with simplified commands

## Benefits

1. **Simpler mental model**: Just three commands, each with one clear purpose
2. **No external scripts**: Everything built into patina
3. **Better onboarding**: `patina init` handles everything
4. **Follows Unix philosophy**: Each command does one thing well
5. **Clean separation**: User commands vs developer tooling
6. **Lean binaries**: Users don't get dev code unless they build with --features dev
7. **Standard Rust pattern**: Feature flags are idiomatic in Rust ecosystem
8. **Discoverable wisdom**: Navigation system initialized from day one
9. **Git-aware knowledge**: Pattern evolution tracked automatically

## Migration Guide

### For Existing Users:
```bash
# Old way (will be deprecated)
patina update

# New way - just re-init!
patina init .
```

Key changes:
- ✅ `patina init .` now handles component updates automatically
- ✅ Shows changelogs so you know what's new
- ✅ No need to remember separate update command
- 🚧 `patina upgrade` checks for new Patina CLI versions (not components)
- ✅ `patina doctor` provides diagnostics without auto-fixing
- ✅ setup.sh has been removed - everything is in `patina init`

### For Patina Developers:
```bash
# Build with dev features
cargo build --features dev

# Access dev commands
patina dev sync-adapters
patina dev bump-version
patina dev release
```

The `update` command temporarily lives under `patina dev update` until fully removed.