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
- src/spec.rs
- src/commands/spec/mod.rs
- src/commands/spec/internal.rs
- src/commands/session/
beliefs:
- spec-first
- session-git-integration
- stale-context-is-hostile-context
- process-checkpoints-over-tooling
- git-is-the-knowledge-substrate
sessions:
- 20260222-054702
- 20260223-084803
- 20260223-092355
---

# feat: Workflow Rigor — Pause, Block, Split, Resume

> The spec system works. The middle doesn't. When you're building a spec
> and something diverts you — the approach is wrong, a prerequisite is
> missing, a new idea emerges, life happens — there's no system to park
> the work, track why you left, or guide you back. This spec adds the
> exit path, the return path, and the queue pressure to keep everything
> moving forward.

## Problem

### The Mystery Middle

Enterprise spec systems (Rust RFCs, AWS ADRs, structured RFCs) all share
the same gap: `active` is a black hole. It covers "just started" through
"almost done" through "stalled for 3 months." None of them handle what
happens mid-implementation when work diverts.

Patina's spec system has the same gap. Today's state machine:

```
draft → ready → active → complete (release + archive)
                              ↘
                         abandoned (archive)
```

Five states, one direction, no way back from active except forward to
complete or down to abandoned. The real workflow looks like this:

```
1. Reason about what to build (user + LLM)         → draft
2. Settle on what to build (review, lock)           → ready
3. Start building (user + LLM)                      → active
4. SOMETHING HAPPENS:
   a. Approach is wrong (won't work)                → ???
   b. Missing prerequisite (need spec-Y first)      → ???
   c. New idea emerges (shiny thing pulls you away) → ???
   d. Life happens (walk away for a week)           → ???
5. Return to work                                   → ???
6. Finish                                           → complete | abandoned | ???
```

Step 4 is where every system falls down. Step 6 has an unnamed third
option: half-done, partially right, needs to ship what works and draft
what doesn't.

### What Exists Today (Infrastructure Is 60% Built)

The spec system already has blocking infrastructure that this spec was
originally written without knowing about:

**Already in `src/spec.rs` (`SpecFrontmatter`):**
- `blocked_by: Vec<String>` — first-class field, serde round-trips
- `blocks: Vec<String>` — first-class field

**Already in `src/commands/scrape/layer/mod.rs`:**
- `spec_deps` table: `(spec_id, depends_on)` — populated from `blocked_by`
- Scrape reads YAML, inserts into `spec_deps` on every run

**Already in `src/commands/spec/internal.rs`:**
- `get_ready_specs()` — JOINs `spec_deps`, filters out blocked specs
- `get_blocked_specs()` — finds specs with incomplete blockers
- `show_ready_specs()` / `show_blocked_specs()` — human + JSON output
- `VALID_STATUSES: ["draft", "ready", "active", "complete", "abandoned"]`
- `update_spec_status()` — updates YAML + DB, auto-archives on complete/abandoned
- Full archive system: git tag + git rm + commit + recovery path

**What's missing:**
- No mutation commands (`spec block`, `spec unblock`)
- No `paused` or `blocked` status values
- No `spec pause`, `spec resume`, `spec split`, `spec next` commands
- No git tags for state transitions (only archive tags today)
- No queue pressure (paused specs can hide forever)
- Session hardening (session list, stale warning, atomic flip)

## Solution

### State Machine

```
draft ──→ ready ──→ active ──→ complete (release + archive + tag)
                      │
                      ├──→ paused (reason required, WIP commit + tag)
                      │      │
                      │      ├──→ active (resume, tag)
                      │      ├──→ abandoned (archive + tag)
                      │      └──→ split
                      │             ├── done half → complete (release)
                      │             └── remaining → new draft (split_from: parent)
                      │
                      ├──→ blocked (blocked_by required, tag)
                      │      │
                      │      ├──→ active (when blockers complete)
                      │      └──→ abandoned (archive + tag)
                      │
                      └──→ abandoned (archive + tag)
```

**New statuses:** `paused`, `blocked`
**New operations:** `pause`, `resume`, `block`, `unblock`, `split`, `next`

### Paused vs Blocked

Both stop work. Different reasons, different rules:

| | Paused | Blocked |
|---|---|---|
| **Why** | User chose to stop (wrong approach, new idea, life) | Can't continue (needs prerequisite spec) |
| **Who decides to resume** | User | System (when blocker completes) |
| **`blocked_by` field** | Optional (may reference a spec, may not) | Required |
| **Queue behavior** | Shows with escalating age pressure | Shows with blocker status |
| **Auto-resume** | No — user must explicitly resume | `spec next` suggests when blocker done |

### Git as the Backbone

Git tags mark every state transition, not just death. Tags are annotated
with reasons. This makes the spec timeline durable, diffable, and
recoverable.

**Tag conventions:**
```
spec/<id>-start           ← when spec goes active
spec/<id>-paused-<N>      ← each pause (annotated with reason)
spec/<id>-resumed-<N>     ← each resume
spec/<id>-blocked-<N>     ← each block (annotated with blocker + reason)
spec/<id>-v<N>-complete   ← split: done half shipped
spec/<id>                 ← final archive (exists today)
```

**WIP commits on pause:** Don't stash. Commit the work-in-progress:
```
WIP: spec-X paused — discovered keychain won't work over SSH
```
Tag the commit. The WIP state is durable, diffable, and the LLM can
read it to regain context on resume.

**Diff as context recovery on resume:**
```bash
# What did I accomplish before pausing?
git diff spec/X-start..spec/X-paused-1

# What changed in the codebase while I was away?
git diff spec/X-paused-1..HEAD

# What files did this spec touch?
git log --name-only spec/X-start..spec/X-paused-1
```

`spec resume` runs these automatically and presents the context to the
LLM. No manual archaeology.

### Spec Split

The half-done problem: some work is shippable, some isn't. Split forces
a decision — what's actually done vs what needs more thought.

**`patina spec split <id> --at <description>`:**
1. Tag current state: `spec/<id>-v1-complete`
2. Complete the original spec (version bump, archive, tag)
3. Create a new draft spec with:
   - `split_from: <parent-id>` in frontmatter
   - Reference to parent tag for provenance
   - Body: remaining work, copied from parent's unfinished phases
4. Git commit: `spec: split <id> — ship done work, draft remainder`

**Why split matters:** It creates tension that prevents the LLM from
shipping MVPs. You can't mark everything complete — you have to draw
the line between "actually done" and "needs more work." The done half
gets a real release. The undone half goes back to draft where it has to
earn its way through the lifecycle again.

### Queue Pressure

Paused and blocked specs can't hide. The queue system applies pressure:

**In `spec list`:**
```
ID                    STATUS              AGE
spec-workflow-rigor   active              -
spec-git-tag-system   paused (12d)        ⚠ stale — resume, split, or abandon
spec-knowledge-evo    blocked             waiting on spec-workflow-rigor
```

**In `spec next`:**
- Active specs first (current work)
- Paused specs with age warnings ("paused 12 days — decision needed")
- Blocked specs with blocker status ("blocker spec-Y is now complete — resume?")
- Drafts available to promote

**In `/session-start`:**
```
Spec landscape:
  Active:  spec-workflow-rigor
  Paused:  spec-git-tag-system (12d) ⚠ — resume, split, or abandon?
  Blocked: spec-knowledge-evo (waiting on spec-workflow-rigor)
  Drafts:  2 available

Recommended: continue spec-workflow-rigor (active)
```

### New SpecFrontmatter Fields

Extend `src/spec.rs` `SpecFrontmatter`:

```rust
// Existing (already in struct):
pub blocked_by: Vec<String>,
pub blocks: Vec<String>,

// New fields:
pub paused_reason: Option<String>,     // why paused (required on pause)
pub paused_date: Option<String>,       // when paused (auto-set)
pub blocked_reason: Option<String>,    // why blocked
pub blocked_date: Option<String>,      // when blocked (auto-set)
pub split_from: Option<String>,        // parent spec ID (set by split)
```

## Implementation

### Phase 1: State Machine — `paused` + `blocked` statuses

**Goal:** Add `paused` and `blocked` to `VALID_STATUSES` and wire up
the basic commands.

**Changes to `src/commands/spec/internal.rs`:**
- Add `"paused"` and `"blocked"` to `VALID_STATUSES`
- `update_spec_status()`: validate transitions (can't go from `draft`
  directly to `paused`; must be `active` first)

**Changes to `src/spec.rs`:**
- Add `paused_reason`, `paused_date`, `blocked_reason`, `blocked_date`,
  `split_from` to `SpecFrontmatter` (all optional, skip_serializing_if)

**New command: `patina spec pause <id> --reason "..."`:**
1. Validate spec is `active`
2. Create WIP commit if uncommitted changes exist
3. Update YAML: status → `paused`, set `paused_reason` and `paused_date`
4. Create annotated tag: `spec/<id>-paused-<N>`
5. Git commit: `spec: pause <id> — <reason>`
6. Log in active session

**New command: `patina spec resume <id>`:**
1. Validate spec is `paused` or `blocked`
2. If `blocked`: check all blockers complete (error if not, `--force` to override)
3. Update YAML: status → `active`, clear pause/block fields
4. Create annotated tag: `spec/<id>-resumed-<N>`
5. Git commit: `spec: resume <id>`
6. Show context diffs:
   - `git diff spec/<id>-paused-<N>..HEAD` — what changed while away
   - `git diff spec/<id>-start..spec/<id>-paused-<N>` — what you accomplished
7. Log in active session

**New command: `patina spec block <id> --by <blocker> --reason "..."`:**
1. Validate spec is `active`
2. Update YAML: status → `blocked`, set `blocked_by`, `blocked_reason`, `blocked_date`
3. Create annotated tag: `spec/<id>-blocked-<N>`
4. Git commit: `spec: block <id> (waiting on <blocker>)`
5. Log in active session

**Exit criteria:**
- [ ] `paused` and `blocked` are valid statuses
- [ ] `spec pause` creates WIP commit + tag + updates YAML
- [ ] `spec resume` checks blockers, shows context diffs, restores active
- [ ] `spec block` sets blocked_by + creates tag
- [ ] Tags follow `spec/<id>-paused-N` / `spec/<id>-blocked-N` convention
- [ ] Invalid transitions rejected (draft → paused, paused → complete)

### Phase 2: Spec Split

**Goal:** Ship done work, draft remaining work as new spec.

**New command: `patina spec split <id>`:**
1. Validate spec is `active` or `paused`
2. Prompt: "Describe what's done" (used for release commit)
3. Tag current state: `spec/<id>-v<N>-complete`
4. Complete original spec (normal release flow: version bump, archive, tag)
5. Create new spec directory: `layer/surface/build/feat/<new-id>/SPEC.md`
   - Frontmatter includes `split_from: <parent-id>`
   - Status: `draft`
   - Body: user-provided description of remaining work
6. Git commit: `spec: split <id> — ship v<N>, draft remainder as <new-id>`

**Exit criteria:**
- [ ] `spec split` completes original spec with release
- [ ] New draft spec created with `split_from` provenance
- [ ] Parent archived with tag `spec/<id>-v<N>-complete`
- [ ] New spec references parent tag for recovery
- [ ] `git show spec/<id>:...` recovers original spec content

### Phase 3: Queue System — `spec next`

**Goal:** The return path. Guide user + LLM to the right work.

**New command: `patina spec next`:**
1. Query all specs (filesystem + DB merge, like `get_all_specs()`)
2. Rank by:
   - Active specs first (current work)
   - Blocked specs whose blockers are now complete ("ready to resume")
   - Paused specs with age (escalating urgency)
   - Impact: blocks N other specs (from `spec_deps`)
   - Drafts ready to promote
3. Output: recommended spec with reasoning

**Enhance `spec ready`:**
- Show impact: "blocks 2 other specs"
- Show paused specs with age warnings
- Show blocked specs whose blockers completed

**Enhance `spec list`:**
- Show age for paused/blocked specs
- Flag stale paused specs (>7 days)

**Exit criteria:**
- [ ] `spec next` recommends a spec with reasoning
- [ ] `spec ready` shows impact and age warnings
- [ ] `spec list` shows age for paused/blocked specs
- [ ] Paused specs >7 days flagged as stale

### Phase 4: Session Hardening

**Goal:** Fix stale session accumulation and partial-end bugs.

**4a. `patina session list` command:**
New subcommand querying `layer/sessions/` + `.patina/local/active-session.md`.
Shows active, stale (>24h), and recent completed sessions.

```
$ patina session list
ACTIVE  20260131-093100  Session System & Adapter Parity (21d stale)
RECENT  20260218-225007  Secrets Keychain Policy (completed, 2d ago)
```

**4b. Stale session warning in `session start`:**
If previous session is >24h old, print warning showing what's being archived.

**4c. Atomic status flip in `session end`:**
Flip `status: active → completed` as the first mutation in `end_session()`,
before computing metrics or archiving. If later steps fail, the session is
at least marked done.

**4d. Richer CLI output for skills:**
CLI returns structured summary to stdout that the skill can use directly.
Fewer LLM interpretation steps. Skill markdown stays the same — just
relies less on file reads.

**Exit criteria:**
- [ ] `patina session list` shows active/stale/recent sessions
- [ ] `session start` warns when archiving a session >24h old
- [ ] `session end` flips status before archiving (atomic-first)
- [ ] Session CLI commands return structured summary to stdout

### Phase 5: Session Integration

**Goal:** Wire spec queue into session workflow.

**Update `/session-start`:**
- Run `patina spec next`, show spec landscape
- Suggest recommended spec with reasoning
- Show paused specs with age warnings

**Update `/session-update`:**
- Detect spec status changes since session start
- Show blocking/unblocking events
- Warn about paused specs aging

**Update `/session-end`:**
- If spec completed, show what it unblocks
- If spec paused, confirm reason was captured
- Suggest next spec to work on

**Exit criteria:**
- [ ] `/session-start` shows spec landscape with recommendations
- [ ] `/session-update` tracks spec status changes
- [ ] `/session-end` suggests next spec and confirms pause reasons

## Testing

### Manual Test Cases

**Test 1: Pause and resume**
```bash
patina spec status my-spec active
patina spec pause my-spec --reason "Discovered need for auth first"
# → WIP commit, tag spec/my-spec-paused-1, status: paused

patina spec list
# → my-spec shows "paused (0d)"

patina spec resume my-spec
# → Shows context diffs, status: active, tag spec/my-spec-resumed-1
```

**Test 2: Block and unblock**
```bash
patina spec block my-spec --by auth-spec --reason "Need auth first"
# → status: blocked, blocked_by: [auth-spec], tag spec/my-spec-blocked-1

patina spec resume my-spec
# → Error: Still blocked by auth-spec (draft)

patina spec status auth-spec complete
patina spec resume my-spec
# → Success: status active, context diffs shown
```

**Test 3: Split**
```bash
patina spec split my-spec
# → Prompts for what's done
# → Completes my-spec (release + archive)
# → Creates my-spec-v2 as draft with split_from: my-spec
# → Tag: spec/my-spec-v1-complete

git show spec/my-spec:layer/surface/build/feat/my-spec/SPEC.md
# → Original spec recovered
```

**Test 4: Queue pressure**
```bash
patina spec pause old-spec --reason "Exploring alternatives"
# ... 12 days pass ...
patina spec next
# → "old-spec has been paused for 12 days. Resume, split, or abandon?"
```

**Test 5: Context recovery on resume**
```bash
patina spec resume my-spec
# Output:
# Since you paused (spec/my-spec-paused-1):
#   Your work: 5 files changed, 120 insertions
#   Codebase changes: 23 commits, 8 files you touched were modified
#   Reason you paused: "Discovered need for auth first"
```

## Open Questions

1. Should `patina session list` also query archived sessions in
   `layer/sessions/`, or only active + `.patina/local/`?
2. Should `patina doctor` check for stale sessions and paused specs?
3. What's the right staleness threshold for paused specs — 7 days? 14 days?
4. Should `spec split` auto-generate the new spec ID (e.g., `<id>-v2`)
   or prompt the user for a name?
5. Should `spec pause` require a WIP commit, or allow pausing with a
   clean tree (no uncommitted work)?

## Deferred (needs own spec if pursued)

- **Session state machine** — Formal states: `created → active → paused →
  ended`. Transitions validated in Rust. Would prevent stale sessions
  structurally but changes the fundamental model (file-as-state →
  state-as-data). Phase 4 fixes may make it unnecessary — try the cheap
  fix before the expensive redesign.

## Non-Goals

- No automatic dependency resolution (Temporal-style orchestration)
- No workflow language (LangGraph-style state machines)
- No complex priority algorithms
- No multi-project coordination (that's Mother's job)
- No automatic unblocking (human decides via `spec resume`)
- No pre-commit hook for auto git metric capture (abandoned — marginal
  value, adds hook complexity)
- No eventlog consumer (measurement concern — belongs with
  measurement-coverage, not workflow rigor)
- No belief lifecycle changes (belongs with spec-knowledge-evolution)

## Success Metrics

**The wander-and-return pattern works:**
```
Before:
  Start spec-X → discover need Y → both show "active" →
  confusion → stale specs → lost context → start over

After:
  Start spec-X → discover need Y →
  `spec block X --by Y` (exit path, tagged, reason captured) →
  work on Y → Y complete →
  `spec resume X` (return path, context diffs, guided) →
  finish X or `spec split X` (ship what's done, draft the rest)
```

**Queue keeps things moving:**
- "Which spec should I work on?" → `spec next`
- "What's blocked on what?" → `spec blocked`
- "This has been paused too long" → `spec list` with age warnings
- "Is this session stale?" → `session list`

**Git preserves everything:**
- Every state transition tagged — full timeline recoverable
- WIP commits on pause — diffable context for resume
- Archives recoverable — `git show spec/<id>:...`
- Split provenance — `split_from` traces lineage

## Provenance

This spec consolidates two earlier specs:
- **spec-blocking-queue** (Feb 22): Spec dependency tracking and queue
  visibility. Born from the keychain SSH cascade in session 20260222-054702.
- **session-hardening** (Feb 20): Session visibility and bug fixes.
  Born from discovering two stale sessions during adapter parity work.

Both abandoned and archived with git tags after consolidation.
Recover originals: `git show spec/<id>:layer/surface/build/feat/<id>/SPEC.md`

Informed by enterprise patterns: Rust RFCs, AWS ADRs, structured RFCs.
All share the same gap in the middle — Patina's spec system addresses it
with `paused`, `blocked`, `split`, and git-backed state transitions.

## Related Work

**Builds on:**
- [[spec-driven-design]]: Specs guide LLM work
- [[git-tags-as-knowledge-refs]]: Git tags preserve state
- [[git-is-the-knowledge-substrate]]: Git as durable memory
- [[spec-first]]: Design before implement

**Distinct from:**
- [[spec-knowledge-evolution]]: Belief/spec schema redesign (epistemic layer)
- [[measurement-coverage]]: Measurement system (observability layer)

## Key Files

```
src/spec.rs                          — SpecFrontmatter struct (add new fields)
src/commands/spec/mod.rs             — public API, clap subcommands (add new commands)
src/commands/spec/internal.rs        — all logic: VALID_STATUSES, state transitions,
                                       ready/blocked queries, archive system
src/commands/session/mod.rs          — session public API
src/commands/session/internal.rs     — session lifecycle logic
resources/claude/session-*.md        — skill instructions
```
