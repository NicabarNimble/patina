# Spec: Architectural Alignment (Core Values Compliance)

**Status**: Living Document
**Created**: 2025-12-27
**Updated**: 2025-12-28
**Purpose**: Ensure patina is unmistakably designed to layer/core values

---

## Design Philosophy

Patina follows two core architectural principles:

1. **dependable-rust**: Small, stable public interfaces; hide implementation in internal/
2. **unix-philosophy**: Single responsibility; composition over monolith

This document maps every module to these values and tracks alignment.

---

## The Two-Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  BINARY: src/commands/ + main.rs                            │
│  CLI parsing, user interaction, output formatting           │
│  "How the user talks to patina"                             │
└──────────────────────────┬──────────────────────────────────┘
                           │ uses
┌──────────────────────────▼──────────────────────────────────┐
│  LIBRARY: src/{secrets,embeddings,db,...}/ + lib.rs         │
│  Core logic, data structures, algorithms                    │
│  "What patina actually does"                                │
└─────────────────────────────────────────────────────────────┘
```

**Principle**: Commands are thin coordinators. Libraries do the work.

**Implication**: When evaluating a command's complexity, look at where the logic lives. A 325-line command that delegates to a 1,764-line library is well-designed. A 600-line command with embedded logic needs refactoring.

---

## Alignment Tiers

| Tier | Pattern | Threshold | Action |
|------|---------|-----------|--------|
| **Exemplary** | mod.rs ≤150 + internal/ | Perfect black-box | Reference for others |
| **Compliant** | mod.rs ≤300, clear structure | Follows principles | Maintain |
| **Acceptable** | Single file ≤400, simple logic | Naturally procedural | Monitor |
| **Review** | 400-600 lines or mixed concerns | Growing complexity | Plan refactor |
| **Refactor** | >600 lines or violated principles | Clear violation | Priority work |

---

## Command Alignment Matrix

### Tier: Exemplary (Reference Implementations)

These commands perfectly embody our core values. Use as templates.

| Command | Structure | Lines | Notes |
|---------|-----------|------:|-------|
| **init** | mod.rs (118) + internal/ (962) | 1,080 | Perfect: doctests in mod.rs, all logic in internal/ |
| **scrape** | mod.rs (126) + subdirs | 5,600+ | Perfect: thin coordinator, domain subdirs |
| **scry** | mod.rs (221) + internal/ (1,856) | 2,077 | Refactored 2025-12-28, 7 internal modules |
| **assay** | mod.rs (134) + internal/ (924) | 1,058 | Refactored 2025-12-28, 6 internal modules |

**What makes them exemplary:**
- mod.rs contains: docs, public types, `execute()` entry point, `pub use` re-exports
- internal/ contains: all implementation logic, helper functions, complex algorithms
- Clear "Do X" purpose for each internal module

### Tier: Compliant

These commands follow the pattern correctly.

| Command | Structure | Lines | Notes |
|---------|-----------|------:|-------|
| serve | mod.rs (34) + internal (269) | 303 | Good separation |
| launch | mod.rs (61) + internal (457) | 518 | Good separation |
| bench | mod.rs (49) + internal (399) | 448 | Good separation |
| repo | mod.rs (309) + internal (817) | 1,126 | mod.rs slightly large but acceptable |
| rebuild | mod.rs (259) | 259 | Simple enough for single file |

### Tier: Acceptable (Thin CLI Glue)

These are appropriately simple - complexity lives in library modules.

| Command | Lines | Delegates To | Library Lines | Notes |
|---------|------:|--------------|---------------|-------|
| **secrets** | 325 | src/secrets/ | 1,764 | Correctly thin - library has complexity |
| build | 32 | workspace | - | Minimal glue |
| test | 31 | workspace | - | Minimal glue |
| version | 160 | version lib | - | Appropriate size |
| upgrade | 162 | version lib | - | Appropriate size |
| model | 211 | models lib | ~600 | Appropriate size |
| adapter | 363 | adapters lib | ~800 | Borderline, mostly dispatch |

**Key insight**: `secrets` was initially flagged for refactoring, but analysis revealed the command is thin glue over a well-structured library. The library (`src/secrets/`) already follows dependable-rust perfectly with 7 internal modules.

### Tier: Review (Monitor for Growth)

These work but don't follow ideal patterns.

| Command | Lines | Concern | Recommendation |
|---------|------:|---------|----------------|
| oxidize | 363 + 1,241 siblings | Uses `pub mod` peer modules instead of internal/ | Consider internalizing when touched |
| yolo | 137 + 1,476 siblings | Uses `pub mod` peer modules instead of internal/ | Consider internalizing when touched |
| eval | 596 | Single file, growing | Plan internal/ if it grows further |
| persona | 609 | Single file, growing | Plan internal/ if it grows further |

**The peer module problem:**
```rust
// Current (oxidize, yolo):
pub mod trainer;      // Exposes trainer as public API
pub mod generator;    // Exposes generator as public API

// Preferred:
mod internal;         // Private
pub use internal::TrainerResult;  // Curated exports only
```

### Tier: Refactor (Priority Work)

These violate core values and need restructuring.

| Command | Lines | Violation | Priority | Target Structure |
|---------|------:|-----------|----------|------------------|
| **audit** | 797 | Monolithic, complex scanning logic | MEDIUM | mod.rs + internal/{scanning,reporting,rules}.rs |
| **doctor** | 602 | Monolithic, many independent checks | MEDIUM | mod.rs + internal/{system,tools,project}.rs |

---

## Library Alignment Matrix

### Tier: Exemplary

| Module | Structure | Lines | Notes |
|--------|-----------|------:|-------|
| **secrets** | mod.rs (503) + 6 internal modules | 1,764 | vault, keychain, identity, recipients, registry, session |
| **embeddings** | mod.rs + 4 modules | ~1,200 | onnx, database, similarity, models |
| **models** | mod.rs + internal + download | ~600 | Good black-box pattern |

### Tier: Compliant

| Module | Structure | Lines | Notes |
|--------|-----------|------:|-------|
| db | mod.rs + modules | ~800 | Appropriate |
| workspace | mod.rs + internal | ~400 | Good separation |
| mcp | mod.rs + protocol + server | ~600 | Good separation |
| retrieval | mod.rs + engine | ~800 | Query engine well-structured |
| adapters | mod.rs + claude/gemini | ~500 | Trait-based, good |

### Tier: Review

| Module | Lines | Concern |
|--------|------:|---------|
| environment.rs | 447 | Single file, could split by platform |
| main.rs | 997 | CLI definition - inherently large, acceptable |
| paths.rs | 280 | Single file, utility focused - acceptable |

---

## Structural Patterns

### Pattern A: Black-Box with internal/ (Preferred for Complex Commands)

```
command/
├── mod.rs              # External interface (≤200 lines)
│   ├── //! Module docs with example
│   ├── mod internal;   # Private!
│   ├── pub use internal::{Type, Result};  # Curated exports
│   └── pub fn execute() # Entry point
└── internal/
    ├── mod.rs          # Re-exports for parent
    ├── feature_a.rs    # Single responsibility
    └── feature_b.rs    # Single responsibility
```

**Use when**: >400 lines, multiple concerns, complex algorithms

**Examples**: init, scry, scrape

### Pattern B: Thin CLI Glue (Acceptable for Simple Commands)

```
command.rs              # Single file (≤350 lines)
├── //! Module docs
├── pub fn execute()    # Delegates to library
└── fn helper()         # Minimal local helpers
```

**Use when**: Mostly delegation to library, simple dispatch logic

**Examples**: secrets, build, test, version

### Pattern C: Domain Subdirectories (For Multi-Domain Commands)

```
command/
├── mod.rs              # Thin coordinator
├── domain_a/           # Self-contained domain
│   ├── mod.rs
│   └── internal/
└── domain_b/           # Self-contained domain
    ├── mod.rs
    └── internal/
```

**Use when**: Command spans multiple distinct domains

**Examples**: scrape (code/, git/, sessions/, layer/)

### Pattern D: Peer Modules (Discouraged)

```
command/
├── mod.rs              # Coordinator
├── feature_a.rs        # pub mod - EXPOSED!
└── feature_b.rs        # pub mod - EXPOSED!
```

**Problem**: Exposes implementation details as public API. Users can depend on internals.

**Migrate to**: Pattern A with internal/

**Current violators**: oxidize, yolo

---

## The "Do X" Test

Before creating a module, state what it does in one sentence:

**Good (clear modules):**
- `internal/search.rs`: "Execute vector and lexical searches"
- `internal/logging.rs`: "Log queries for feedback loop"
- `internal/vault.rs`: "Encrypt and store secrets"

**Bad (unclear scope):**
- `utils.rs`: "Various utilities" (what utilities?)
- `helpers.rs`: "Helper functions" (for what?)
- `common.rs`: "Common code" (common to what?)

**When "Do X" is unclear:**
1. Split it into multiple focused modules
2. Or accept it as glue code and keep minimal

---

## Completed Alignments

### 2025-12-28: scry refactoring

**Trigger**: 2,141 lines in single file, 30 functions, no separation

**Before**:
```
src/commands/scry.rs    # 2,141 lines, monolithic
```

**After**:
```
src/commands/scry/
├── mod.rs (221 lines)
└── internal/
    ├── query_prep.rs (228)   - FTS query preparation
    ├── logging.rs (178)      - Query logging, feedback
    ├── enrichment.rs (271)   - Result enrichment
    ├── search.rs (398)       - Vector/lexical search
    ├── hybrid.rs (158)       - RRF fusion
    ├── routing.rs (188)      - Mothership, all-repos
    └── subcommands.rs (601)  - orient, recent, why, etc.
```

**Result**:
- 90% reduction in mod.rs (2,141 → 221)
- 7 focused internal modules
- MRR 0.588 (exceeds 0.55 target)
- 10 surgical commits

**Lessons learned**:
1. Extract leaf modules first (no internal dependencies)
2. Use `super::super::Type` for parent type references
3. One commit per module extraction for clean history

### 2025-12-28: assay refactoring

**Trigger**: 997 lines in single file, 16 functions, multiple query types mixed

**Before**:
```
src/commands/assay.rs    # 997 lines, monolithic
```

**After**:
```
src/commands/assay/
├── mod.rs (134 lines)
└── internal/
    ├── mod.rs (15)        - Re-exports
    ├── util.rs (21)       - String truncation
    ├── imports.rs (113)   - Import relationship queries
    ├── inventory.rs (172) - File/module listing
    ├── functions.rs (228) - Function definitions, call graph
    └── derive.rs (375)    - Structural signal computation
```

**Result**:
- 86% reduction in mod.rs (997 → 134)
- 6 focused internal modules
- 6 surgical commits (one per module extraction)

**"Do X" for each module**:
- `util.rs`: "Truncate strings to display width"
- `imports.rs`: "Query import relationships between files"
- `inventory.rs`: "List files and modules in codebase"
- `functions.rs`: "Query function definitions and call graph"
- `derive.rs`: "Compute structural signals from code facts"

---

## Planned Alignments

### Priority 1: doctor/audit/repo Architectural Cleanup - HIGH

**Analysis Date**: 2025-12-28

**Discovery**: Deep analysis revealed doctor.rs violates unix-philosophy by doing three jobs:

```
doctor.rs (603 lines) - THREE COMMANDS IN ONE
├── Health Checks (210 lines)         ← Core purpose
│   ├── analyze_environment()
│   ├── display_health_check()
│   └── count_patterns/sessions()
│
├── Audit Delegation (4 lines)        ← Just calls audit::execute()
│   └── if audit_files { audit::execute() }
│
└── Repo Management (305 lines)       ← Completely separate system
    ├── handle_repos()                  targets layer/dust/repos/
    ├── discover_repos()
    ├── check_repo_status()
    ├── update_repo()
    └── log_repo_status()
```

**Critical Finding - Two Separate Repo Systems**:

| System | Location | Purpose | Management |
|--------|----------|---------|------------|
| `doctor --repos` | `layer/dust/repos/` | Legacy research repos | Manual clones, unregistered |
| `patina repo` | `~/.patina/cache/repos/` | Modern managed repos | Registry at `~/.patina/registry.yaml` |

These systems don't communicate. `layer/dust/repos/` appears to be legacy from before `patina repo` existed.

**audit.rs (797 lines) - Hidden Power Tool**:

audit is NOT wired as a command - only accessible via `patina doctor --audit`. It's a full filesystem analyzer:

```
audit.rs (797 lines)
├── Scanning (150 lines)
│   ├── scan_files() - Walk tree, categorize
│   ├── categorize_file() - Source/Doc/Config/Build/etc
│   └── determine_safety() - Critical/Protected/ReviewNeeded/SafeToDelete
│
├── Layer Analysis (197 lines)
│   ├── analyze_layer_directory()
│   ├── analyze_layer_subdir() - core/surface/dust
│   ├── analyze_repos() - count repos in dust/repos/
│   └── analyze_sessions()
│
├── Display (230 lines)
│   ├── display_audit()
│   └── display_layer_insights()
│
└── Utils (27 lines)
    ├── format_date()
    └── format_size()
```

**patina repo (1,126 lines) - Already Well-Structured**:

```
repo/
├── mod.rs (309 lines)           ← Public interface
│   ├── RepoCommands enum (clap)
│   ├── execute_cli() entry
│   └── Public functions: add, list, update, remove, show
│
└── internal.rs (817 lines)      ← Implementation
    ├── Registry management
    ├── add_repo() - Clone, scaffold, scrape
    ├── update_repo() - Pull, rescrape
    ├── remove_repo()
    └── Helpers
```

---

#### Action Plan

**Step 1: Promote audit to `patina audit` command**
- audit.rs already exists and works
- Add to main.rs CLI as standalone command
- Remove `--audit` flag from doctor

**Step 2: Deprecate doctor --repos**
- `layer/dust/repos/` is legacy concept
- Print deprecation message pointing to `patina repo`
- Remove 305 lines of repo code from doctor

**Step 3: Slim doctor to pure health checks (~250 lines)**
After cleanup:
```
doctor.rs (~250 lines)
├── Health Checks
│   ├── analyze_environment()
│   └── check project config
│
└── Display
    └── display_health_check()
```

At ~250 lines, doctor becomes "Tier: Acceptable" - may not need internal/ pattern.

**Step 4: Refactor audit to internal/ pattern**
After promotion to command:
```
src/commands/audit/
├── mod.rs (~100 lines)
│   ├── pub fn execute()
│   └── types (FileAudit, SafetyLevel, etc.)
└── internal/
    ├── scanner.rs (~200 lines) - "Scan filesystem and categorize files"
    ├── analysis.rs (~200 lines) - "Analyze layer structure"
    ├── display.rs (~200 lines) - "Format and display results"
    └── util.rs (~30 lines) - "Format dates and sizes"
```

**Optional Step 5: Migration path for layer/dust/repos/**
```bash
# Future command to discover unregistered repos
patina repo discover layer/dust/repos/  # Find and register
```

---

#### Summary Table

| Component | Current State | Action | Result |
|-----------|--------------|--------|--------|
| audit | Hidden behind --audit flag | Promote to `patina audit` | Standalone command |
| doctor --repos | 305 lines, legacy system | Deprecate, remove | Cleaner doctor |
| doctor health | 210 lines, core purpose | Keep, simplify | ~250 line command |
| patina repo | 1,126 lines, well-structured | Keep as-is | Reference impl |
| layer/dust/repos/ | Legacy manual repos | Deprecate concept | Use patina repo |

### Deferred: oxidize/yolo peer modules

**Reason**: Lower priority - code works, just not ideal pattern
**Action**: Internalize when these modules are touched for other reasons

### Cancelled: secrets command refactoring

**Original plan**: Split 325-line command into internal/ modules

**Why cancelled**: Analysis revealed architecture is correct:
- Command (325 lines) is thin CLI glue
- Library (`src/secrets/`, 1,764 lines) has the complexity
- Library already follows black-box pattern with 7 modules
- Refactoring would add structure without benefit

**Lesson**: Always check where complexity lives before planning refactors.

---

## Enforcement

### Code Review Checklist

When reviewing new code or refactors:

- [ ] Commands >400 lines use internal/ pattern
- [ ] No `pub mod internal` (internal stays private)
- [ ] No `internal::` types in public function signatures
- [ ] Library modules expose minimal public API
- [ ] Commands delegate to libraries, don't duplicate logic
- [ ] Each internal module passes "Do X" test
- [ ] New peer modules (`pub mod foo`) are discouraged

### Thresholds for Action

| Metric | Threshold | Action |
|--------|-----------|--------|
| Command mod.rs | >300 lines | Consider internal/ |
| Command total | >600 lines without internal/ | Refactor required |
| Single file command | >400 lines | Plan restructure |
| Library module | >500 lines without structure | Consider splitting |

### Metrics to Track

- Commands in each alignment tier
- Average mod.rs size for directory-based commands
- % of commands following black-box pattern
- Library modules with proper internal/ structure

---

## References

- [dependable-rust.md](../../core/dependable-rust.md) - Black-box module pattern
- [unix-philosophy.md](../../core/unix-philosophy.md) - Single responsibility principle
- [Session 20251228-062007](../../../sessions/20251228-062007.md) - scry refactoring session
- [Session 20251228-070251](../../../sessions/20251228-070251.md) - This architectural review
