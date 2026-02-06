---
type: feat
id: spec-as-work-item
status: complete
created: 2026-02-05
updated: 2026-02-05
sessions:
  origin: 20260205-102402
  updated: 20260205-130049
related:
- layer/surface/build/explore/beads-patterns/SPEC.md
- layer/surface/build/feat/patina-platform/SPEC.md
beliefs:
- simplicity-is-architecture
- argue-every-box
- unix-philosophy
references:
- steveyegge/beads - ARCHITECTURE.md, MOLECULES.md, CLI_REFERENCE.md
---

# feat: Spec as Work Item

> A spec should feel like a git branch to developers and a TODO list to agents.

## Problem

Patina specs are **documents**, not **work items**. They describe what we want but don't drive action.

**Symptoms:**
- Specs accumulate without closure
- "related:" links are prose, not blocking semantics
- No way to ask "what spec can I work on NOW?"
- Status fields are fuzzy — no clear state machine
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

### What We Take from Beads

| Beads Feature | Patina Equivalent | Notes |
|---------------|-------------------|-------|
| `bd ready` | `patina spec ready` | Core feature |
| `bd blocked` | `patina spec blocked` | Shows blockers |
| `bd dep tree` | `patina spec tree` | Nice-to-have |
| `bd close --reason` | `patina spec archive` | Already have |
| Status flow | draft → ready → active → complete | In frontmatter |
| blocks/related | blocked_by, blocks, related | In frontmatter, not parsed |

### What We DON'T Take from Beads

| Beads Feature | Why Skip |
|---------------|----------|
| Hash-based IDs | Filename IDs work fine for specs |
| JSONL sync | layer/ + scrape is our pattern |
| Daemon | mother handles daemon needs |
| Molecules/wisps | Overkill for spec workflow |
| Labels | beliefs serve similar purpose |
| Priority 0-4 | `target: v0.X.0` is better for releases |

---

## Design

### 1. Dependency Semantics

**Already in our frontmatter:**

```yaml
---
type: feat
id: cli-reorganization
status: ready
blocked_by:
  - system-introspection  # Can't start until this is complete
blocks:
  - science-commands      # Those wait for this
related:
  - scrape-layer-unify    # Soft link, doesn't block
target: v0.12.0
---
```

**Dependency types:**

| Field | Semantic | Affects `spec ready`? |
|-------|----------|----------------------|
| `blocked_by` | This spec can't start until those complete | Yes |
| `blocks` | Those specs can't start until this completes | Yes (inverse) |
| `related` | Soft link for context | No |

**Gap:** These fields exist but aren't parsed by scrape. We need to parse them.

### 2. Status State Machine

Clear, actionable states:

```
draft ──▶ ready ──▶ active ──▶ complete
  │                   │          │
  └───────────────────┴──────────┘
         (can regress)
```

| Status | Meaning | Shows in `spec ready`? |
|--------|---------|------------------------|
| `draft` | Still designing, not actionable | No |
| `ready` | Design complete, unblocked, can start | **Yes** |
| `active` | Currently being implemented | Yes (continue) |
| `complete` | Implemented, exit criteria met | No (archive) |
| `abandoned` | Rejected or superseded | No |

**Key rule:** A spec is actionable only if:
1. Status is `ready` or `active`
2. All `blocked_by` specs have status `complete`

### 3. Ready Queue Command

```bash
$ patina spec ready

READY (can start now):
  scrape-layer-unify     v0.12.0   Unify scrape layer command

BLOCKED:
  cli-reorganization     v0.12.0   blocked by: system-introspection
  system-introspection   v0.12.0   (status: draft)

ACTIVE (in progress):
  spec-as-work-item      v0.12.0   This spec

$ patina spec ready --json   # For agent use
```

### 4. Blocked View

```bash
$ patina spec blocked

cli-reorganization       blocked by: system-introspection (draft)
                                     scrape-layer-unify (ready)

$ patina spec blocked --json
```

### 5. Dependency Tree (Nice-to-have)

```bash
$ patina spec tree cli-reorganization

cli-reorganization (blocked)
├── blocked by: system-introspection (draft)
│   └── (no blockers)
└── blocked by: scrape-layer-unify (ready)
    └── (no blockers)
```

### 6. Status Update Command

```bash
# Advance spec status (edits frontmatter in file)
patina spec status <id> ready      # Mark as ready to implement
patina spec status <id> active     # Mark as in-progress
patina spec status <id> complete   # Mark as complete (then archive)
```

### 7. Cycle Detection

Circular dependencies should be detected and warned:

```bash
$ patina spec ready
WARNING: Circular dependency detected:
  A → B → C → A

$ patina doctor
  Spec cycles: 1 found (A → B → C → A)
```

---

## Implementation

### Phase 1: Scrape Integration (Foundation)

Extend `scrape layer` to parse dependency fields:

```rust
// In src/commands/scrape/layer/mod.rs

#[derive(Debug, Deserialize)]
struct SpecFrontmatter {
    id: String,
    status: Option<String>,
    blocked_by: Option<Vec<String>>,
    blocks: Option<Vec<String>>,
    target: Option<String>,
    // ... existing fields
}
```

**New table:**

```sql
CREATE TABLE spec_deps (
    spec_id TEXT NOT NULL,
    depends_on TEXT NOT NULL,
    UNIQUE(spec_id, depends_on)
);

-- Also extend patterns table
ALTER TABLE patterns ADD COLUMN target TEXT;
```

**Exit:** `blocked_by` and `blocks` fields populate `spec_deps` table.

### Phase 2: Ready Queue

Implement `patina spec ready`:

```rust
// Query: specs where status in (ready, active) AND all blocked_by are complete
pub fn get_ready_specs(conn: &Connection) -> Result<Vec<Spec>> {
    conn.prepare(r#"
        SELECT p.id, p.title, p.status, p.target
        FROM patterns p
        WHERE p.status IN ('ready', 'active')
          AND NOT EXISTS (
            SELECT 1 FROM spec_deps d
            JOIN patterns blocker ON d.depends_on = blocker.id
            WHERE d.spec_id = p.id
              AND blocker.status NOT IN ('complete', 'done')
          )
        ORDER BY p.target, p.id
    "#)?.query_map(...)
}
```

**Exit:** `patina spec ready` shows unblocked specs.

### Phase 3: Blocked View

Implement `patina spec blocked`:

```rust
// Query: specs that have incomplete blockers
pub fn get_blocked_specs(conn: &Connection) -> Result<Vec<BlockedSpec>> {
    conn.prepare(r#"
        SELECT p.id, p.title, d.depends_on, blocker.status
        FROM patterns p
        JOIN spec_deps d ON d.spec_id = p.id
        JOIN patterns blocker ON d.depends_on = blocker.id
        WHERE blocker.status NOT IN ('complete', 'done')
        ORDER BY p.id
    "#)?.query_map(...)
}
```

**Exit:** `patina spec blocked` shows blocked specs with reasons.

### Phase 4: Status Command

Implement `patina spec status <id> <status>`:

```rust
// 1. Find spec file by id
// 2. Parse frontmatter
// 3. Update status field
// 4. Write file back
// 5. Update database directly
// 6. If status = complete, trigger auto-release
```

**Exit:** `patina spec status <id> active` updates file and database.

### Phase 4b: Auto-Release on Completion (Added 2026-02-05)

**Not from Beads** — emerged from version rules exploration.

When `patina spec status <id> complete` is called:

```
Spec type determines version impact:
  fix/refactor → patch bump (0.0.x)
  feat         → minor bump (0.x.0)
  explore      → no bump

→ Update Cargo.toml
→ Commit: "release: v{version} — {spec title}"
→ Create git tag: v{version}
```

**Key insight:** A spec IS a milestone. One spec = one version bump. No batching, no `target` planning, no `milestones` array. Git tags are history.

See: [[version-rules-system]], [[spec-is-milestone]]

**Exit:** Completing a spec auto-releases based on type.

### Phase 5: Cycle Detection

Add to `patina doctor`:

```rust
// Detect cycles in spec dependency graph
pub fn detect_spec_cycles(conn: &Connection) -> Result<Vec<Vec<String>>> {
    // Tarjan's algorithm or simple DFS
}
```

**Exit:** `patina doctor` warns about circular dependencies.

---

## Trait-Based Design (Plugin Ready)

Design for future extraction to WASM plugin:

```rust
/// The interface that becomes WIT later
pub trait SpecTracker {
    fn ready(&self) -> Result<Vec<SpecSummary>>;
    fn blocked(&self) -> Result<Vec<BlockedSpec>>;
    fn get(&self, id: &str) -> Result<Option<Spec>>;
    fn update_status(&self, id: &str, status: Status) -> Result<()>;
    fn add_dep(&self, spec: &str, blocked_by: &str) -> Result<()>;
    fn remove_dep(&self, spec: &str, blocked_by: &str) -> Result<()>;
    fn detect_cycles(&self) -> Result<Vec<Vec<String>>>;
}

// Implementation v1: Native (now)
pub struct NativeSpecTracker { conn: Connection }

// Implementation v2: WASM plugin (when plugin system exists)
pub struct PluginSpecTracker { plugin: WasmPlugin }
```

---

## Exit Criteria

### v0.12.0: Core Functionality

- [x] `blocked_by` / `blocks` fields parsed by scrape into `spec_deps` table
- [x] `patina spec ready` shows unblocked specs
- [x] `patina spec blocked` shows blocked specs with reasons
- [x] `patina spec status <id> <status>` updates spec file
- [x] Existing specs work with new system (backwards compatible)
- [x] `--json` output for agent use
- [x] Auto-release on completion (spec type → version bump → git tag)

### v0.13.0: Polish

- [x] `patina spec list` with filters (--status, --target)

**Rehomed (2026-02-06):**
- `patina spec tree` + cycle detection → [[fix/spec-tree-and-cycles/SPEC.md]]
- Plugin extraction (crate, WIT, WASM) → stays in [[feat/patina-platform/SPEC.md]] scope

---

## Non-Goals

- **Replacing beads** — Beads is for general issue tracking. This is for spec lifecycle.
- **Complex workflow engine** — Keep it simple. Dependencies + ready queue.
- **Auto-generating specs** — Specs are human decisions, not automation.
- **Hash-based IDs** — Filename-based IDs work fine for specs.
- **JSONL format** — Keep markdown, it's human-readable.

---

## Open Questions (Resolved 2026-02-06)

1. **Should explore docs have dependencies?**
   → **No.** Explores are research, not deliverables. Parse all, filter in query. Only `feat/` and `refactor/` get work-item treatment.

2. **Version targets vs dependencies?**
   → **Resolved by [[spec-is-milestone]] belief.** Target fields removed. Spec type determines version impact. Dependencies control execution order. No dual system.

3. **Command rename later?**
   → **Deferred.** `patina spec` works. Functionality > naming.

---

## Files to Change

```
src/commands/
├── scrape/layer/mod.rs      # Parse blocked_by, blocks, target
├── spec/
│   ├── mod.rs               # Add subcommands
│   ├── ready.rs             # NEW: ready queue
│   ├── blocked.rs           # NEW: blocked view
│   ├── status.rs            # NEW: status update
│   └── internal.rs          # Existing archive logic
└── doctor.rs                # Add cycle detection
```

---

## Relationship to Other Specs

| Spec | Relationship |
|------|--------------|
| **patina-platform** | This is Step 0. Build native, extract to plugin later. |
| **system-introspection** | Will use new schema (DataContract for spec commands) |
| **cli-reorganization** | Will use new schema, places spec in `infra/` group |
| **scrape-layer-unify** | Unblocked by this — can start now |
| **beads-patterns** | Source of design inspiration |

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | draft | Created from beads analysis. Key insight: specs should be work items with ready queue, not documents. |
| 2026-02-05 | draft | Superseded by patina-platform — work tracking to be WASM plugin. |
| 2026-02-05 | ready | **UN-SUPERSEDED.** Build native now, extract to plugin later. Deep dive into beads confirmed design. Added trait-based approach for future plugin extraction. Removed blocker on patina-platform. |
| 2026-02-05 | active | Phases 1-4 complete. Added auto-release: spec completion triggers version bump based on type (fix→patch, feat→minor). "Spec is the milestone" — no batching, no target planning. See [[version-rules-system]], [[spec-is-milestone]]. |
