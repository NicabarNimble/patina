---
type: feat
id: spec-as-work-item
status: draft
created: 2026-02-05
sessions:
  origin: 20260205-102402
blocked_by:
  - patina-platform
blocks: []
related:
  - layer/surface/build/explore/beads-patterns/SPEC.md
  - layer/surface/build/feat/patina-platform/SPEC.md
beliefs:
  - simplicity-is-architecture
  - argue-every-box
references:
  - "steveyegge/beads - MOLECULES.md, ARCHITECTURE.md"
superseded_by: patina-platform
---

# feat: Spec as Work Item

> **SUPERSEDED:** This spec is now input to [[patina-platform]]. Work tracking will be a WASM plugin (`patina-work`), not modifications to the spec system. The design here informs the plugin requirements.

> A spec should feel like a git branch to developers and a TODO list to agents.

## Problem

Patina specs are **documents**, not **work items**. They describe what we want but don't drive action.

**Symptoms:**
- Specs accumulate without closure
- "related:" links are prose, not blocking semantics
- No way to ask "what spec can I work on NOW?"
- Status fields are fuzzy (`design`, `active`) — no clear state machine
- Specs created in one session, forgotten by the next
- We keep creating specs about specs (meta-loop)

**The cart-horse-chicken-egg loop:** We need more system info → create introspection spec. Too many CLI commands → create cli-reorganization spec. Specs don't drive action → create spec-about-specs. The problem is the spec system itself.

---

## Insight from Beads

> "Work = issues with dependencies. That's it. No special types needed."

Beads treats everything as work items with:
1. **Explicit dependencies** that control execution order
2. **Ready queue** showing only unblocked work
3. **Clear status flow** with actual state machine
4. **Closure** — work items get done and close

**The key command:** `bd ready` — shows what you can work on NOW.

We need `patina spec ready`.

---

## Design

### 1. Dependency Semantics

Replace fuzzy `related:` with explicit blocking:

```yaml
---
type: feat
id: cli-reorganization
status: ready
blocked_by:
  - system-introspection  # Can't reorganize until DataContract exists
blocks:
  - science-commands      # New commands wait for reorg
related:
  - scrape-layer-unify    # Soft link, doesn't block
---
```

**Dependency types:**

| Field | Semantic | Affects `spec ready`? |
|-------|----------|----------------------|
| `blocked_by` | This spec can't start until those close | Yes |
| `blocks` | Those specs can't start until this closes | Yes (inverse) |
| `related` | Soft link for context | No |

### 2. Status State Machine

Clear, actionable states:

```
draft ──▶ ready ──▶ active ──▶ done
  │                   │         │
  └───────────────────┴─────────┘
         (can regress)
```

| Status | Meaning | Can work on it? |
|--------|---------|-----------------|
| `draft` | Still designing, not actionable | No |
| `ready` | Design complete, unblocked, can start | **Yes** |
| `active` | Currently being implemented | Yes (continue) |
| `done` | Implemented, exit criteria met | No (archive) |
| `abandoned` | Rejected or superseded | No |

**Key rule:** A spec is `ready` only if:
1. Status is `ready` or `active`
2. All `blocked_by` specs are `done`

### 3. Ready Queue Command

```bash
$ patina spec ready

READY (can start now):
  scrape-layer-unify     v0.12.0   Unify scrape layer command

BLOCKED:
  cli-reorganization     v0.12.0   blocked by: system-introspection
  system-introspection   v0.12.0   blocked by: (none, but status=draft)

ACTIVE (in progress):
  spec-as-work-item      -         This spec

$ patina spec ready --all
# Also shows done/abandoned
```

### 4. Spec Lifecycle Commands

```bash
# Advance spec status
patina spec status <id> ready      # Mark as ready to implement
patina spec status <id> active     # Mark as in-progress
patina spec status <id> done       # Mark as complete

# Dependency management
patina spec block <id> <blocked-by-id>    # Add blocker
patina spec unblock <id> <blocked-by-id>  # Remove blocker

# Query
patina spec ready                  # Show actionable specs
patina spec blocked                # Show what's waiting
patina spec tree <id>              # Show dependency tree
```

### 5. Spec Hierarchy (Optional)

Like beads molecules, specs can have children:

```yaml
---
type: feat
id: v0.12-foundation
status: active
children:
  - system-introspection
  - cli-reorganization
  - scrape-layer-unify
---
```

Parent spec is `done` when all children are `done`.

---

## Migration

### Phase 1: Frontmatter Schema

Update existing specs with new fields:

```yaml
# Before
related:
  - some-other-spec

# After
blocked_by: []           # Explicit blockers
blocks: []               # What this blocks
related:
  - some-other-spec      # Soft links (unchanged)
status: draft            # Explicit state
```

### Phase 2: Scrape Integration

`patina scrape layer` parses new fields:

```rust
pub struct SpecMetadata {
    pub id: String,
    pub status: SpecStatus,
    pub blocked_by: Vec<String>,
    pub blocks: Vec<String>,
    pub related: Vec<String>,
    pub target: Option<String>,  // version target
}

pub enum SpecStatus {
    Draft,
    Ready,
    Active,
    Done,
    Abandoned,
}
```

### Phase 3: Ready Queue

Implement `patina spec ready`:

1. Scrape all specs from `layer/surface/build/`
2. Build dependency graph
3. Filter to specs where:
   - `status` is `ready` or `active`
   - All `blocked_by` specs have `status: done`
4. Display sorted by priority/target version

### Phase 4: Status Commands

Add `patina spec status`, `patina spec block`, etc.

---

## Exit Criteria

### Immediate (this session)

- [x] Spec created with beads-inspired design
- [x] Update 3-4 existing specs with new frontmatter schema
- [ ] Validate dependency graph makes sense

### v0.12.0

- [ ] `blocked_by` / `blocks` fields recognized by scrape
- [ ] `patina spec ready` shows unblocked specs
- [ ] `patina spec status <id> <status>` works
- [ ] Existing specs migrated to new schema

### v0.13.0

- [ ] `patina spec tree <id>` shows dependency graph
- [ ] `patina spec block` / `unblock` commands
- [ ] Integration with session workflow (auto-update spec status)

---

## Non-Goals

- **Replacing beads** — Beads is for issue tracking. This is for spec lifecycle.
- **Complex workflow engine** — Keep it simple. Dependencies + ready queue.
- **Auto-generating specs** — Specs are human decisions, not automation.

---

## Open Questions

1. **Should explore docs have dependencies?**
   - Explores are research, not deliverables
   - Maybe they stay document-like, only feats get work-item treatment

2. **Circular dependency detection?**
   - `patina spec ready` should warn if cycles exist
   - Or `patina doctor` checks for cycles

3. **Version targets vs dependencies?**
   - Current: `target: v0.12.0`
   - Could derive from dependency depth instead
   - Or keep both (target is aspiration, deps are reality)

---

## Relationship to Other Specs

This spec is **foundational** — it changes how all other specs work.

| Spec | Relationship |
|------|--------------|
| system-introspection | Will use new schema |
| cli-reorganization | Will use new schema |
| scrape-layer-unify | Will use new schema |
| beads-patterns (explore) | Source of design inspiration |

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | ready | Created from beads analysis. Key insight: specs should be work items with ready queue, not documents. |
| 2026-02-05 | draft | **Superseded by patina-platform.** Work tracking becomes `patina-work` WASM plugin, not spec system changes. This spec becomes requirements input for that plugin. |
