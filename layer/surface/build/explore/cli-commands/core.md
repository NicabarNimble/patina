# Core Commands

> The capture → index → query → learn loop. These ARE Patina.

## Overview

| Command | Phase | One-Line Description |
|---------|-------|---------------------|
| `scrape` | CAPTURE | Gather knowledge from sources |
| `oxidize` | INDEX | Build embeddings and projections |
| `scry` | QUERY | Search codebase knowledge |
| `context` | QUERY | Get patterns and conventions |
| `assay` | QUERY | Query codebase structure |
| `session` | LEARN | Track development sessions |
| `belief` | LEARN | Manage epistemic beliefs |
| `persona` | LEARN | Cross-project user knowledge |

---

## `scrape` — Capture Knowledge

### What does it do?
Ingests data from various sources into the eventlog and materialized tables.

### Subcommands
```
scrape code      # Parse source files (functions, types, imports)
scrape git       # Extract commits, tags, history
scrape layer     # Patterns, beliefs, sessions (see scrape-layer-unify spec)
scrape forge     # GitHub issues and PRs
```

### What does it read?
| Subcommand | Source |
|------------|--------|
| `code` | `src/**/*` (via tree-sitter parsers) |
| `git` | `.git/` (commits, tags, history) |
| `layer` | `layer/**/*.md` (patterns, beliefs, sessions) |
| `forge` | GitHub API |

### What does it write?
- `.patina/local/data/patina.db` (eventlog + materialized tables)
- Event types: `code.*`, `git.*`, `pattern.*`, `belief.*`, `session.*`, `forge.*`

### Who uses it?
Both user and dev. Part of normal workflow.

### When is it used?
Periodic — after code changes, git activity, or session updates.

### Gaps
- `scrape sessions` is separate from `scrape layer` (see scrape-layer-unify spec)

### Overlaps
- None

---

## `oxidize` — Build Embeddings

### What does it do?
Creates vector embeddings from scraped content and builds the USearch index.

### Subcommands
None (single command)

### What does it read?
- `.patina/local/data/patina.db` (eventlog, tables)
- `~/.patina/cache/personas/*/persona.db` (persona knowledge)

### What does it write?
- `.patina/local/data/embeddings/<model>/projections/semantic.usearch`

### Who uses it?
Both user and dev. Required for semantic search.

### When is it used?
Periodic — after scrape, when content changes.

### Gaps
- No progress indicator for large codebases
- No incremental mode (re-embeds everything)

### Overlaps
- None

---

## `scry` — Search Knowledge

### What does it do?
Hybrid search across code, commits, sessions, beliefs using multiple oracles.

### Subcommands
```
scry "query"              # Default search
scry --mode find|orient|recent|detail|full|why|use
scry --limit N
scry --include-issues     # Include forge content
```

### What does it read?
- `.patina/local/data/patina.db` (tables, FTS indexes)
- `.patina/local/data/embeddings/.../semantic.usearch` (vectors)
- `~/.patina/cache/personas/*/persona.db` (persona oracle)

### What does it write?
- `.patina/local/data/patina.db` (eventlog: `scry.query`, `scry.use`, `scry.feedback`)

### Who uses it?
User — primary search interface.

### When is it used?
Every session — main way to find code/knowledge.

### Gaps
- No config flag to specify oracle weights
- No A/B comparison of results

### Overlaps
- Some overlap with `assay` for structural queries

---

## `context` — Get Patterns

### What does it do?
Returns project patterns (core + surface) and beliefs, optionally filtered by topic.

### Subcommands
```
context                   # All patterns + belief summary
context --topic "auth"    # Topic-filtered beliefs via BeliefOracle
```

### What does it read?
- `layer/core/*.md` (direct file read, not from DB)
- `layer/surface/*.md` (direct file read, not from DB)
- `.patina/local/data/patina.db` (beliefs table, for stats or topic query)
- `.patina/local/data/embeddings/.../semantic.usearch` (for topic ranking)

### What does it write?
- Nothing (read-only)

### Who uses it?
User — to understand project conventions before making changes.

### When is it used?
Every session — check patterns before architectural work.

### Gaps
- Inconsistent: reads patterns directly but beliefs from DB
- No way to see which patterns are stale

### Overlaps
- None

---

## `assay` — Query Structure

### What does it do?
Query codebase structure: modules, imports, call graph, derived signals.

### Subcommands
```
assay                     # Default: inventory
assay inventory           # List modules
assay imports <module>    # What does this import?
assay importers <module>  # What imports this?
assay functions <module>  # List functions
assay callers <fn>        # What calls this function?
assay callees <fn>        # What does this function call?
assay derive              # Compute structural signals
```

### What does it read?
- `.patina/local/data/patina.db` (code tables, call graph, module signals)

### What does it write?
- `assay derive` writes to `module_signals` table

### Who uses it?
Both user and dev. Structural analysis.

### When is it used?
Periodic — when understanding architecture or debugging imports.

### Gaps
- No visualization
- `derive` could auto-run after scrape

### Overlaps
- Some structural queries could be done via `scry --mode orient`

---

## `session` — Track Work

### What does it do?
Manage development sessions: start, update, add notes, end with classification.

### Subcommands
```
session start "goal"      # Begin tracking
session update            # Capture progress
session note "insight"    # Add a note
session end               # Archive and classify
session list              # Show recent sessions
```

### What does it read?
- `.patina/local/active-session.md` (current session)
- `git status`, `git log`, `git diff` (git context)
- `.patina/local/data/patina.db` (eventlog for classification)

### What does it write?
- `.patina/local/active-session.md` (create/update)
- `layer/sessions/*.md` (archive on end)
- `.patina/local/data/patina.db` (eventlog: `session.started`, `session.update`, `session.ended`)

### Who uses it?
User — track development work.

### When is it used?
Every session — start at beginning, end at end.

### Gaps
- No config tracking (which model/adapter was active)
- Classification could be richer

### Overlaps
- None

---

## `belief` — Manage Beliefs

### What does it do?
Create, list, and audit epistemic beliefs.

### Subcommands
```
belief create             # Interactive belief creation
belief list               # List all beliefs
belief audit              # Show usage/truth metrics
belief show <id>          # Show single belief
```

### What does it read?
- `layer/surface/epistemic/beliefs/*.md` (belief files)
- `.patina/local/data/patina.db` (beliefs table for audit)

### What does it write?
- `layer/surface/epistemic/beliefs/<id>.md` (new belief files)

### Who uses it?
User — capture and manage project decisions.

### When is it used?
Periodic — when capturing design decisions.

### Gaps
- No belief evolution (belief → value → rule from mother-v2)
- No cross-project belief federation

### Overlaps
- None

---

## `persona` — Cross-Project Knowledge

### What does it do?
Manage user-level knowledge that persists across projects.

### Subcommands
```
persona note "insight"    # Add knowledge
persona materialize       # Build persona DB from events
persona list              # Show persona notes
```

### What does it read?
- `~/.patina/personas/default/events/*.jsonl` (source stream)

### What does it write?
- `~/.patina/personas/default/events/*.jsonl` (note command)
- `~/.patina/cache/personas/default/persona.db` (materialize command)

### Who uses it?
User — build personal knowledge base.

### When is it used?
Rare — when capturing cross-project insights.

### Gaps
- Underused — not integrated into workflow
- No clear guidance on what to capture

### Overlaps
- Conceptually related to beliefs, but user-level vs project-level

---

## Summary

| Command | Reads | Writes | Frequency |
|---------|-------|--------|-----------|
| `scrape` | Source files, git, layer, APIs | patina.db | Periodic |
| `oxidize` | patina.db | semantic.usearch | Periodic |
| `scry` | patina.db, vectors, persona.db | eventlog | Every session |
| `context` | layer/*.md (direct), beliefs table | Nothing | Every session |
| `assay` | patina.db | module_signals (derive) | Periodic |
| `session` | active-session.md, git | eventlog, session files | Every session |
| `belief` | belief files, beliefs table | belief files | Periodic |
| `persona` | persona jsonl | persona.db, jsonl | Rare |
