---
type: feat
id: scrape-layer-unify
status: ready
created: 2026-02-05
sessions:
  origin: 20260205-084522
target: v0.12.0
blocked_by: []
blocks:
  - cli-reorganization
related:
  - layer/surface/build/feat/system-introspection/SPEC.md
  - layer/surface/build/feat/mother-v2/SPEC.md
beliefs:
  - simplicity-is-architecture
  - unix-philosophy
---

# feat: Unify `scrape layer`

> One command owns one domain. `scrape layer` owns `layer/`.

## Problem

Currently we have fragmented scrape commands for layer content:

```
scrape layer    → layer/core/*.md, layer/surface/*.md (patterns + beliefs)
scrape sessions → layer/sessions/*.md (separate command)
```

This causes:
- Two commands to remember for one conceptual domain
- Layer structure changes require updating multiple commands
- Inconsistent: why are sessions special?
- Future additions (values, rules) would mean more commands

**The Unix principle:** One command should own one domain. `layer/` is one domain.

---

## Current State

```
Commands:
├── scrape code     → src/**/*
├── scrape git      → .git/
├── scrape layer    → layer/**/*.md EXCEPT sessions
├── scrape sessions → layer/sessions/*.md  ← WHY SEPARATE?
└── scrape forge    → GitHub API

Layer structure:
layer/
├── core/*.md                    → patterns (scrape layer)
├── surface/*.md                 → patterns (scrape layer)
├── surface/epistemic/
│   └── beliefs/*.md             → beliefs (scrape layer)
├── sessions/*.md                → sessions (scrape sessions) ← DIFFERENT
└── dust/*.md                    → patterns (scrape layer)
```

---

## Target State

```
Commands:
├── scrape code   → src/**/*
├── scrape git    → .git/
├── scrape layer  → layer/**/*  ← OWNS ALL OF LAYER
└── scrape forge  → GitHub API

Internal routing (implementation detail):
layer/
├── core/*.md                    → PatternScraper
├── surface/*.md                 → PatternScraper
├── surface/epistemic/
│   ├── beliefs/*.md             → BeliefScraper
│   ├── values/*.md              → ValueScraper (future)
│   └── rules/*.md               → RuleScraper (future)
├── sessions/*.md                → SessionScraper
└── dust/*.md                    → PatternScraper
```

**User sees:** `scrape layer`
**Implementation:** Routes based on path to appropriate sub-scraper

---

## Design

### Path-Based Routing

```rust
// In src/commands/scrape/layer/mod.rs

fn scrape_layer(path: &Path) -> Result<()> {
    for file in glob("layer/**/*.md")? {
        match classify_layer_path(&file) {
            LayerContent::Pattern => scrape_pattern(&file)?,
            LayerContent::Belief => scrape_belief(&file)?,
            LayerContent::Value => scrape_value(&file)?,
            LayerContent::Rule => scrape_rule(&file)?,
            LayerContent::Session => scrape_session(&file)?,
        }
    }
    Ok(())
}

fn classify_layer_path(path: &Path) -> LayerContent {
    let path_str = path.to_string_lossy();

    if path_str.contains("sessions/") {
        LayerContent::Session
    } else if path_str.contains("epistemic/beliefs/") {
        LayerContent::Belief
    } else if path_str.contains("epistemic/values/") {
        LayerContent::Value
    } else if path_str.contains("epistemic/rules/") {
        LayerContent::Rule
    } else {
        LayerContent::Pattern
    }
}
```

### Event Types

Each sub-scraper writes to eventlog with distinct event types:

| Path | Event Type | Table |
|------|------------|-------|
| `layer/core/*.md` | `pattern.core` | `patterns` |
| `layer/surface/*.md` | `pattern.surface` | `patterns` |
| `layer/surface/epistemic/beliefs/*.md` | `belief.surface` | `beliefs` |
| `layer/surface/epistemic/values/*.md` | `value.surface` | `values` (future) |
| `layer/surface/epistemic/rules/*.md` | `rule.surface` | `rules` (future) |
| `layer/sessions/*.md` | `session.archived` | `sessions` |
| `layer/dust/*.md` | `pattern.dust` | `patterns` |

### CLI Interface

```bash
# Scrape all of layer/
patina scrape layer

# Scrape specific content type (optional, for debugging/partial runs)
patina scrape layer --only patterns
patina scrape layer --only beliefs
patina scrape layer --only sessions

# Incremental (already exists, keep working)
patina scrape layer --incremental
```

---

## Migration

### Phase 1: Unify Under `scrape layer`

1. Move session scraping logic into `src/commands/scrape/layer/`
2. Add path classification
3. Keep `scrape sessions` as alias (deprecated)
4. Update `patina scrape` help text

### Phase 2: Deprecation Warning

```bash
$ patina scrape sessions
WARNING: `scrape sessions` is deprecated. Use `scrape layer` instead.
         Sessions are part of layer/ and scraped automatically.
```

### Phase 3: Remove `scrape sessions`

After one release cycle, remove the separate command.

---

## Files to Change

```
src/commands/scrape/
├── mod.rs              # Remove sessions subcommand, update layer
├── layer/
│   ├── mod.rs          # Add path routing, import session logic
│   ├── patterns.rs     # Existing
│   ├── beliefs.rs      # Existing
│   ├── sessions.rs     # NEW: moved from scrape/sessions/
│   └── router.rs       # NEW: path classification
└── sessions/           # REMOVE after migration
    └── mod.rs
```

---

## Data Contract Update

```rust
pub const SCRAPE_LAYER_CONTRACT: DataContract = DataContract {
    command: "scrape layer",
    description: "Scrape all layer content (patterns, beliefs, sessions, etc.)",
    reads: &[
        Source::Files("layer/core/*.md"),
        Source::Files("layer/surface/*.md"),
        Source::Files("layer/surface/epistemic/beliefs/*.md"),
        Source::Files("layer/surface/epistemic/values/*.md"),  // future
        Source::Files("layer/surface/epistemic/rules/*.md"),   // future
        Source::Files("layer/sessions/*.md"),
        Source::Files("layer/dust/*.md"),
    ],
    writes: &[
        Sink::Table("patterns"),
        Sink::Table("beliefs"),
        Sink::Table("values"),    // future
        Sink::Table("rules"),     // future
        Sink::Table("sessions"),
        Sink::Eventlog("pattern.*"),
        Sink::Eventlog("belief.*"),
        Sink::Eventlog("value.*"),   // future
        Sink::Eventlog("rule.*"),    // future
        Sink::Eventlog("session.archived"),
        Sink::Fts("pattern_fts"),
        Sink::Fts("belief_fts"),
    ],
    write_path: WritePath::Scrape,
};
```

---

## Exit Criteria

### v0.12.0: Unified Layer Scraping

- [ ] `patina scrape layer` scrapes sessions (no separate command needed)
- [ ] Path classification routes to correct sub-scraper
- [ ] Event types distinguish content types (`pattern.*`, `belief.*`, `session.*`)
- [ ] `--only <type>` flag works for partial scrapes
- [ ] `scrape sessions` shows deprecation warning
- [ ] Data contract declared (when DataContract type exists)

### v0.13.0: Integration

- [ ] `patina introspect scrape-layer` shows all sources/sinks (after introspect exists)

---

## Future: Values and Rules

When mother-v2 adds values and rules:

```
layer/surface/epistemic/
├── beliefs/*.md   → already handled
├── values/*.md    → add ValueScraper
└── rules/*.md     → add RuleScraper
```

No new top-level command needed. Just add classification case and sub-scraper.

---

## Non-Goals

- **Changing layer structure** — we're unifying the scraper, not reorganizing files
- **Changing event schema** — keep existing event formats
- **Breaking existing workflows** — deprecation path, not immediate removal

---

## Open Questions

1. **Should `--only` be subcommands instead?**
   - `patina scrape layer patterns` vs `patina scrape layer --only patterns`
   - Leaning toward flag (less CLI surface area)

2. **What about `context`?**
   - Context reads layer files directly (no scrape)
   - Keep for now, address separately (noted in system-introspection spec)

3. **Incremental per content type?**
   - Currently incremental is all-or-nothing
   - Could track `scrape_meta` per content type
   - Probably overkill for now

---

## Relationship to Other Specs

### Ownership Boundaries (aligned 2026-02-05)

| Concern | Owner | This Spec's Role |
|---------|-------|------------------|
| `scrape layer` command behavior | **this spec** | Defines path routing, sub-scrapers |
| `DataContract` for scrape layer | system-introspection | Declares contract when type exists |
| Code location | cli-reorganization | Places in `core/scrape/` |

**Implementation order:** This spec can be implemented independently (v0.12.0). DataContract declaration waits for system-introspection to define the type. Introspection integration waits for introspect command.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | design | Created from system-introspection session. Recognized that `scrape sessions` being separate from `scrape layer` violates "one command owns one domain" principle. |
| 2026-02-05 | design | **Spec alignment:** Target v0.12.0. No dependencies on other specs for core functionality. DataContract and introspect integration are follow-on work. |
