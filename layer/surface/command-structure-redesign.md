---
id: command-structure-redesign
version: 1
status: draft
created_date: 2025-08-06
oxidizer: nicabar
references: [core/context-orchestration.md, topics/development/repository-workflow.md]
tags: [architecture, cli, commands, ux, refactoring]
---

# Command Structure Redesign for Patina

A comprehensive redesign of Patina's command structure to eliminate confusion between init, update, and setup commands while providing a clearer mental model for users.

## Executive Summary

Current Patina has overlapping responsibilities between `init`, `update`, and external `setup.sh`, causing confusion about when to use each command. This design proposes a clear separation of concerns with intuitive command names that match user intent.

## Core Problem: Overlapping Commands

```
Current State:
- setup.sh: Installs tools AND creates PROJECT_DESIGN.toml
- patina init: Validates environment, creates config
- patina update: Updates components OR refreshes environment

User Confusion:
"Do I run setup or init?"
"Is update for patina or my environment?"
"Why do I need setup.sh if I have patina?"
```

## Proposed Command Architecture

### Command Hierarchy

```
patina
├── init [name|.]      # Initialize project (first time or refresh)
├── sync               # Sync project state with environment
├── self-update        # Update patina binary
├── components         # Manage internal components
│   ├── update         # Update adapters/templates
│   ├── list           # Show installed components
│   └── status         # Check for updates
└── doctor [--fix]     # Health check and repair
```

### Command Responsibilities

#### `patina init [name|.]`
**Purpose**: Universal project initialization

```bash
# New project
patina init myproject
> Create PROJECT_DESIGN.toml? [Y/n]
> Install missing tools (dagger, claude)? [Y/n]
> ✓ Project initialized

# Existing project
patina init .
> Found PROJECT_DESIGN.toml
> Missing tools: dagger
> Install? [Y/n]
```

**Key Innovation**: Integrates setup.sh functionality directly into init.

#### `patina sync`
**Purpose**: Refresh project state (clearer than current `update`)

```bash
patina sync
> ✓ Environment scanned
> ✓ Context regenerated (.claude/CLAUDE.md)
> ✓ Config updated
```

#### `patina components`
**Purpose**: Manage Patina's internal components (original `update` purpose)

```bash
patina components update
> Updating claude adapter: v1.2 → v1.3
> Updating dagger templates: v0.1 → v0.2

patina components status
> claude adapter: v1.3 (latest)
> gemini adapter: v0.8 (v0.9 available)
```

## Environment Scanning Strategy

### Smart Caching System

```json
// .patina/environment-cache.json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "path_hash": "sha256:abc123...",  // Hash of PATH env
  "ttl_seconds": 3600,               // 1 hour default
  "tools": {
    "rust": { "version": "1.88.0", "path": "/usr/bin/rustc" },
    "docker": { "version": "28.3.2", "path": "/usr/local/bin/docker" }
  }
}
```

### Scan Decision Tree

```
Command executed
       ↓
Is it init/sync/doctor?
   ├─ Yes → Full scan (ignore cache)
   └─ No → Check cache
            ├─ Valid? → Use cache
            └─ Expired? → Background refresh
```

## Tool Installation Integration

### Current Problem
```
setup.sh (external) → Installs tools
patina init → Expects tools exist
Result: Circular dependency
```

### New Flow
```
patina init → Detects missing → Offers install → Ready
(One tool, one flow)
```

### Installation Functions Move Into Patina
```rust
// src/commands/init.rs
fn ensure_tools(required: Vec<Tool>) -> Result<()> {
    let missing = detect_missing(&required);
    if !missing.is_empty() {
        println!("Missing tools: {}", missing.join(", "));
        if confirm("Install?") {
            install_tools(missing)?;
        }
    }
    Ok(())
}
```

## Migration Strategy

### Phase 1: Addition (Non-breaking)
- Add `sync` command
- Add `components` subcommands
- Add tool installation to `init`

### Phase 2: Deprecation
```rust
// patina update
println!("Warning: 'update' is deprecated.");
println!("Use 'sync' for environment refresh");
println!("Use 'components update' for adapters");
```

### Phase 3: Removal (Major version)
- Remove deprecated commands
- Remove setup.sh
- Update documentation

## User Experience Improvements

### New Developer Experience
```bash
# Single command to start
cargo install patina
patina init myproject

# Everything handled:
> Creating PROJECT_DESIGN.toml...
> Installing tools: rust ✓ docker ✓ dagger ✓
> Initializing adapters...
> Ready! Run 'cd myproject'
```

### Team Onboarding
```bash
git clone team-project
cd team-project
patina init .

# Automatic setup:
> Found PROJECT_DESIGN.toml
> Checking requirements...
> Installing: dagger
> Syncing environment...
> Ready to code!
```

### Daily Workflow
```bash
# Clear, purposeful commands
patina sync                    # Refresh my environment
patina components update       # Update patina tools
patina doctor --fix           # Something's wrong, fix it
```

## Implementation Plan

### 1. Environment Caching (Week 1)
- Add cache structure
- Implement cache validation
- Add background refresh

### 2. Sync Command (Week 2)
- Extract refresh logic from update
- Implement as new command
- Add tests

### 3. Init Enhancement (Week 3)
- Port setup.sh functions to Rust
- Add interactive design wizard
- Integrate tool installation

### 4. Components Subcommands (Week 4)
- Design component registry
- Implement update logic
- Add version management

### 5. Migration Support (Week 5)
- Add deprecation warnings
- Update documentation
- Release notes

## Success Metrics

1. **Reduced Confusion**: Fewer "which command?" questions
2. **Faster Onboarding**: One command to start
3. **Clearer Mental Model**: Commands match intent
4. **Better Performance**: Cached environment scans

## Conclusion

This redesign eliminates the overlapping responsibilities that cause confusion while maintaining backward compatibility during migration. The result is a cleaner, more intuitive command structure that better serves both new and experienced users.