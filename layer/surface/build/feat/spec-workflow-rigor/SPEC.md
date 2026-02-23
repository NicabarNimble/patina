---
type: feat
id: spec-workflow-rigor
status: draft
created: 2026-02-23
priority: high
consolidates:
- spec-blocking-queue
- session-hardening
related:
- layer/core/spec-driven-design.md
- src/commands/spec/
- src/commands/session/
beliefs:
- spec-first
- session-git-integration
- stale-context-is-hostile-context
- process-checkpoints-over-tooling
sessions:
- 20260222-054702
- 20260223-084803
- 20260223-092355
---

# feat: Workflow Rigor — Spec Blocking, Queue Visibility, and Session Hardening

> Track what's blocked, show what's ready, harden the session edges.
> Consolidates spec-blocking-queue (Feb 22) and session-hardening (Feb 20)
> into one workflow spec. Both address the same gap: we have durable state
> (git, sessions, beliefs) but lack tracking rigor.

## Problem

### 1. Spec Dependencies Are Implicit

When working on spec-X, you discover spec-Y is needed first. Both show
"active." No way to mark the dependency, query what's ready, or guide the
LLM to the right work. The "oh shit" cascade from session 20260222-054702:

```
spec-secrets-keychain-ssh (complete <- wrong!)
spec-keychain-macos26-regression (active <- impossible!)
spec-secrets-dual-storage (draft <- real work)
spec-launcher-auth (active <- actually blocked!)
```

No link, no query, no history, no guidance.

### 2. Sessions Go Stale Silently

Sessions accumulate in `active` status when the user walks away without
`/session-end`. Two stale sessions discovered on 2026-02-20: one from
Dec 9 2025 (completed but status never flipped), one from Jan 31 2026
(started, never used). The only cleanup path is `session start` archiving
the previous session — but that only fires when you start a new one.

### Root Cause

Both problems stem from the same gap: **we have durable state but lack
tracking rigor.** State exists but isn't queryable. Dependencies exist
but aren't explicit. The queue exists in our heads but isn't visible.

## What NOT to Change

- **Skill-as-prompt pattern** — markdown instructions telling the LLM how
  to behave are the core value. Don't replace with pure CLI.
- **Dual-write architecture** — markdown for LLM collaboration, eventlog
  for structured queries. Both serve distinct purposes.
- **Git tag bracketing** — `session-{id}-{adapter}-start/end` is simple
  and enables replay.
- **Work classification heuristics** — imperfect but useful. Refine later.

## Solution

### Spec Blocking: Explicit Dependencies in YAML

Add blocking metadata to spec frontmatter:

```yaml
---
id: spec-launcher-auth
status: blocked
blocked_by:
  - spec-secrets-dual-storage
blocked_date: 2026-02-22
blocked_reason: |
  Keychain SSH fails with -25308. Need encrypted file secrets first.
depends_on:
  - spec-git-tag-system  # soft dependency (nice-to-have)
---
```

Key principles:
1. **Explicit over implicit** — blocking is metadata, not just notes
2. **Queryable** — can find all blocked specs
3. **Traceable** — history shows when/why blocked
4. **Durable** — survives in git, sessions, database

### Session Hardening: Visibility and Bug Fixes

- `patina session list` — show active, stale (>24h), recent sessions
- Stale session warning in `session start` — don't archive silently
- Atomic status flip in `session end` — flip status before computing
  metrics, so partial failures don't leave zombie sessions

## Implementation

### Phase 1: Spec Blocking — YAML + Query (low risk, additive)

**Goal:** Start tracking blocking manually, query it.

**1a. New YAML fields (no code changes):**
```yaml
blocked_by: [spec-id]
blocked_date: YYYY-MM-DD
blocked_reason: "..."
```

**1b. Update `src/commands/spec/blocked.rs`:**
- Parse YAML files for `blocked_by` field
- Show specs with blocking info and blocker status

**Exit criteria:**
- [ ] Can manually add blocking metadata to spec YAML
- [ ] `patina spec blocked` parses and displays blocked specs
- [ ] Blocker status shown (is the blocker complete yet?)

### Phase 2: Spec Blocking — Commands (automate the YAML)

**Goal:** `block` and `unblock` commands so humans/LLMs don't edit YAML.

**New files:**
- `src/commands/spec/block.rs`
- `src/commands/spec/unblock.rs`

**`patina spec block <id> --by <blocker-id> --reason "..."`:**
1. Update spec YAML: status → blocked, add blocked_by/date/reason
2. Git commit: `spec: block <id> (waiting on <blocker>)`
3. Log in active session if exists

**`patina spec unblock <id>`:**
1. Check if all blockers are complete (error if not, `--force` to override)
2. Update spec YAML: status → active, remove blocking fields
3. Git commit: `spec: unblock <id> (<blocker> complete)`
4. Log in active session

**Exit criteria:**
- [ ] `patina spec block <id> --by <blocker> --reason "..."` updates YAML + commits
- [ ] `patina spec unblock <id>` checks blocker status before unblocking
- [ ] `--force` flag bypasses blocker check
- [ ] Session log records blocking changes

### Phase 3: Session Hardening (low risk, additive)

**Goal:** Fix stale session accumulation and partial-end bugs.

**3a. `patina session list` command:**
New subcommand querying `layer/sessions/` + `.patina/local/active-session.md`.
Shows active, stale (>24h), and recent completed sessions.

```
$ patina session list
ACTIVE  20260131-093100  Session System & Adapter Parity (21d stale)
RECENT  20260218-225007  Secrets Keychain Policy (completed, 2d ago)
```

**3b. Stale session warning in `session start`:**
If previous session is >24h old, print warning showing what's being archived.

**3c. Atomic status flip in `session end`:**
Flip `status: active → completed` as the first mutation in `end_session()`,
before computing metrics or archiving. If later steps fail, the session is
at least marked done.

**Exit criteria:**
- [ ] `patina session list` shows active/stale/recent sessions
- [ ] `session start` warns when archiving a session >24h old
- [ ] `session end` flips status before archiving (atomic-first)

### Phase 4: Database & Queue Queries

**Goal:** Fast queries, priority recommendations.

**Database schema (extend specs table):**
```sql
ALTER TABLE specs ADD COLUMN blocked_date TEXT;
ALTER TABLE specs ADD COLUMN blocked_reason TEXT;

CREATE TABLE spec_blocks (
    blocked_spec TEXT NOT NULL,
    blocker_spec TEXT NOT NULL,
    since TEXT NOT NULL,
    reason TEXT,
    PRIMARY KEY (blocked_spec, blocker_spec),
    FOREIGN KEY (blocked_spec) REFERENCES specs(id),
    FOREIGN KEY (blocker_spec) REFERENCES specs(id)
);
```

**Enhanced `patina spec ready`:**
- Query database for unblocked specs
- Show priority based on blocker count ("blocks 2 other specs")

**New `patina spec next`:**
- Recommend spec based on status, impact (blocks N), momentum (recent sessions)

**Exit criteria:**
- [ ] Database stores blocking relationships
- [ ] `patina spec ready` shows priority (high if blocks others)
- [ ] `patina spec next` recommends what to work on
- [ ] Queries are fast (<100ms for 100 specs)

### Phase 5: Session Integration

**Goal:** Seamless workflow tying specs and sessions together.

**Update `/session-start`:**
- Query `patina spec next`, suggest recommended spec
- Show spec landscape: ready, blocked, recently unblocked

**Update `/session-update`:**
- Detect spec status changes since session start
- Show blocking/unblocking events
- Suggest unblock if blocker completed

**Update `/session-end`:**
- If spec completed, show what it unblocks
- Suggest next spec to work on

**Exit criteria:**
- [ ] `/session-start` suggests spec to work on
- [ ] `/session-update` tracks blocking changes
- [ ] `/session-end` suggests unblocked specs

## Testing

### Manual Test Cases

**Test 1: Block a spec**
```bash
patina spec block spec-A --by spec-B --reason "Need B first"
patina spec blocked
# → spec-A listed, blocker: spec-B
```

**Test 2: Unblock when blocker complete**
```bash
patina spec unblock spec-A
# → Error: Still blocked by spec-B (draft)
patina spec status spec-B complete
patina spec unblock spec-A
# → Success: spec-A active
```

**Test 3: Queue query**
```bash
patina spec block spec-A --by spec-B
patina spec block spec-C --by spec-B
patina spec ready
# → Shows: spec-B (blocks 2), spec-D
# → Hides: spec-A, spec-C (blocked)
```

**Test 4: Session list**
```bash
patina session list
# → Shows active sessions, flags stale (>24h)
```

## Non-Goals

- No automatic dependency resolution (Temporal-style orchestration)
- No workflow language (LangGraph-style state machines)
- No complex priority algorithms
- No multi-project coordination (that's Mother's job)
- No automatic unblocking (human decides when to unblock)
- No eventlog consumer in patina-review (premature — needs its own spec)
- No pre-commit hook for auto git metric capture (premature)

## Success Metrics

**Before:**
- "Which spec should I work on?" → grep through session notes
- "What's blocked on what?" → implicit, in your head
- "Is this session stale?" → discover accidentally

**After:**
- "Which spec should I work on?" → `patina spec next`
- "What's blocked on what?" → `patina spec blocked`
- "Is this session stale?" → `patina session list` (flagged automatically)
- State survives sessions (durable), LLM guided to correct spec

## Provenance

This spec consolidates two earlier specs:
- **spec-blocking-queue** (Feb 22): Spec dependency tracking and queue visibility.
  Born from the keychain SSH cascade in session 20260222-054702.
- **session-hardening** (Feb 20): Session visibility and bug fixes.
  Born from discovering two stale sessions during adapter parity work.

Both abandoned and archived with git tags after consolidation.
Recover originals: `git show spec/<id>:layer/surface/build/feat/<id>/SPEC.md`

## Related Work

**Builds on:**
- [[spec-driven-design]]: Specs guide LLM work
- [[git-tags-as-knowledge-refs]]: Git tags preserve state
- [[spec-first]]: Design before implement

**Distinct from:**
- [[spec-knowledge-evolution]]: Schema redesign, different risk profile
- [[measurement-coverage]]: Measurement system, blocked by evolve verb

## Key Files

```
src/commands/spec/blocked.rs     — existing (needs enhancement)
src/commands/spec/block.rs       — new (Phase 2)
src/commands/spec/unblock.rs     — new (Phase 2)
src/commands/session/mod.rs      — public API, clap subcommands
src/commands/session/internal.rs — lifecycle logic
resources/claude/session-*.md    — skill instructions
src/eventlog.rs                  — event infrastructure
```
