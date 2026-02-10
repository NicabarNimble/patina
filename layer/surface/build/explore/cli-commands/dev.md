# Dev Commands

> How is Patina built? For contributors.

## Overview

| Command | Status | One-Line Description |
|---------|--------|---------------------|
| `doctor` | EXISTS | Health checks (moving here) |
| `report` | EXISTS | System state dump (moving here) |
| `introspect` | **NEW** | Data flows, contracts, sources, sinks |
| `contracts` | **NEW** | List all declared data contracts |

---

## `doctor` — Health Checks

### What does it do?
Checks project health: file existence, database integrity, embedding status.

### Current Interface
```
patina doctor                     # Run all checks
patina doctor --fix               # Attempt to fix issues
```

### Proposed Additions
```
patina dev doctor                 # Same as above (new location)
patina dev doctor --coherence     # Contract verification
patina dev doctor --contracts     # Check declared vs actual
```

### What does it read?
- `.patina/local/data/patina.db` (existence, schema)
- `.patina/local/data/embeddings/` (existence, integrity)
- `layer/` (pattern files)
- Config files

### What does it write?
- stdout (health report)
- May fix issues with `--fix`

### Who uses it?
- User: "Is my Patina setup working?"
- Dev: "Are contracts accurate?"

### When is it used?
Periodic — when things seem broken, or CI.

### Gaps
- No contract verification (from system-introspection spec)
- No coherence metrics

### Overlaps
- `--coherence` overlaps with `introspect --orphans`

---

## `report` — System State Dump

### What does it do?
Generates a report of project state using Patina's own tools.

### Current Interface
```
patina report                     # Full report
patina report --format json       # Machine-readable
```

### What does it read?
- Everything (runs scry, context, assay internally)

### What does it write?
- stdout or file (report)

### Who uses it?
- Dev: "What's the current state of this project?"
- User: "Give me a summary"

### When is it used?
Rare — for diagnostics, sharing state.

### Gaps
- Unclear what's in the report vs what's not
- Could include introspection data

### Overlaps
- Could subsume some of `doctor` output

---

## `introspect` — Data Flows and Contracts (NEW)

### What would it do?
Query the system blueprint: what commands read/write, trace data flows.

### Proposed Interface
```
# Per-command queries
patina dev introspect <command>           # What does this command touch?
patina dev introspect --table <name>      # Who reads/writes this table?
patina dev introspect --event <type>      # Who emits/consumes this event?

# Aggregate views
patina dev introspect --sources           # ALL raw data sources
patina dev introspect --sinks             # ALL storage locations
patina dev introspect --write-paths       # Categorize by write path type

# Analysis
patina dev introspect --schema            # Full schema dump
patina dev introspect --trace <path>      # Trace data flow from source
patina dev introspect --orphans           # Find unused tables/events
patina dev introspect --impact <table>    # What breaks if I change this?
```

### Example Output
```
$ patina dev introspect scry
Command: scry
Description: Search codebase knowledge

Reads:
  Tables: beliefs, commits_fts, co_changes, code_fts, module_signals
  FTS: code_fts, commits_fts, belief_fts
  Vectors: semantic.usearch
  External: ~/.patina/cache/personas/default/persona.db

Writes:
  Eventlog: scry.query, scry.use, scry.feedback

Write Path: action-time
Oracles: semantic, lexical, temporal, persona, belief
```

```
$ patina dev introspect --sources
Raw Data Sources (across all commands):

Files:
  src/**/*                              scrape code
  .git/                                 scrape git
  layer/**/*.md                         scrape layer
  layer/core/*.md                       context (direct read)
  layer/surface/*.md                    context (direct read)

APIs:
  GitHub API                            scrape forge

User Input:
  stdin/args                            persona note, belief create
  git status/log                        session start/update/end
```

### What would it read?
- Data contracts declared in code (`src/introspection/`)
- Actual database schema (for verification)
- Actual eventlog (for counts)

### What would it write?
- stdout (introspection report)

### Who would use it?
- Dev: "What does scry touch? If I change beliefs, what breaks?"

### When would it be used?
During development — before making changes.

### Dependencies
- Requires data contracts to be declared in code (Phase 1 of system-introspection)

---

## `contracts` — List Data Contracts (NEW)

### What would it do?
List all declared data contracts in a summary view.

### Proposed Interface
```
patina dev contracts                      # List all
patina dev contracts --coverage           # Show % of commands with contracts
patina dev contracts --verify             # Check against reality
```

### Example Output
```
$ patina dev contracts
Data Contracts (23/25 commands declared)

CORE:
  scrape code     reads: src/**/*           writes: patina.db (code.*)
  scrape git      reads: .git/              writes: patina.db (git.*)
  scrape layer    reads: layer/**/*.md      writes: patina.db (pattern.*, belief.*, session.*)
  oxidize         reads: patina.db          writes: semantic.usearch
  scry            reads: patina.db, vectors writes: eventlog (scry.*)
  context         reads: layer/*.md, DB     writes: (none)
  assay           reads: patina.db          writes: module_signals (derive)
  session         reads: git, session.md    writes: eventlog, session files
  belief          reads: belief files       writes: belief files
  persona         reads: persona jsonl      writes: persona.db, jsonl

SCIENCE:
  eval            reads: patina.db, vectors writes: (none)
  bench           reads: ground truth, DB   writes: (none)

INFRA:
  init            reads: templates          writes: project structure
  ...

Missing contracts: help, version
```

### What would it read?
- Contract declarations from code

### What would it write?
- stdout (contracts summary)

### Who would use it?
- Dev: "What contracts exist? Are we at 100% coverage?"

### When would it be used?
During development — tracking contract coverage.

### Dependencies
- Requires data contracts to be declared in code

---

## Summary

| Command | Status | Purpose | Frequency |
|---------|--------|---------|-----------|
| `doctor` | Exists (moving) | Health checks | Periodic |
| `report` | Exists (moving) | State dump | Rare |
| `introspect` | **NEW** | Data flows | During dev |
| `contracts` | **NEW** | Contract list | During dev |

## Relationship to system-introspection Spec

This namespace implements the system-introspection spec:

| Spec Phase | Dev Command |
|------------|-------------|
| Phase 1: Data Contracts | Foundation for all `dev` commands |
| Phase 2: Introspect Command | `patina dev introspect` |
| Phase 4: Contract Verification | `patina dev doctor --coherence` |
| Phase 5: Impact Analysis | `patina dev introspect --impact` |
| Phase 6: Coherence Metrics | `patina dev doctor --coherence` |
