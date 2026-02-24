---
type: feat
id: spec-workflow-rigor
status: active
created: 2026-02-23
sessions:
- 20260222-054702
- 20260223-084803
- 20260223-092355
- 20260223-120524
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
- mutation-completes-query
- active-is-a-black-hole
- specs-orthogonal-to-sessions
- plugins-are-three-prong-bundles
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
- No mutation commands — query side exists (ready, blocked, list) but
  mutation side doesn't (pause, resume, block, complete, abandon)
- No `paused` or `blocked` status values
- `spec status` does too many jobs (setter, release trigger, archive
  trigger, escape hatch) — violates unix-philosophy
- No git tags for state transitions (only archive tags today)
- No queue pressure (paused specs can hide forever)
- No `spec create` — specs are hand-created, no templates
- Session hardening (session list, stale warning, atomic flip)
- No MCP tools for spec operations
- No unified `/spec` skill for LLM discovery

## Solution

### Architecture: Three-Layer Capability

Every spec operation is exposed through three layers. This is the
[[plugins-are-three-prong-bundles]] pattern:

```
┌─────────────────────────────────────────────┐
│  Adapter Skill (/spec)                      │  ← WHEN to act (LLM judgment)
│  Single skill describes full capability.    │
│  LLM reads once, knows what's available.    │
├─────────────────────────────────────────────┤
│  MCP Tools (JSON-RPC typed parameters)      │  ← HOW to call (interface)
│  Same operations as CLI, structured I/O.    │
│  LLM calls these directly.                 │
├─────────────────────────────────────────────┤
│  CLI Commands (Rust, deterministic)         │  ← WHAT happens (execution)
│  Explicit params, --json output.            │
│  Machine-first, no inference.               │
└─────────────────────────────────────────────┘
```

The CLI is the single implementation. MCP tools call the same Rust
functions. The skill teaches the LLM when to use which tool. All three
layers ship together.

**Design for plugin extraction:** Keep all spec logic behind the
`commands/spec/mod.rs` interface (dependable-rust pattern). The public
API shape today becomes the WIT contract when spec moves to a WASM
plugin. Don't scatter spec logic across other modules.

### Command Decomposition

Decompose `spec status` into single-purpose commands. Each command does
one thing ([[unix-philosophy]]). This completes the mutation side that
the query commands are missing ([[mutation-completes-query]]).

**Query commands** (read-only):

| Command | Do X |
|---|---|
| `spec list` | Show all specs with filters |
| `spec ready` | Show actionable specs (unblocked, ready/active) |
| `spec blocked` | Show blocked specs with blocker status |
| `spec next` | Recommend next spec to work on |

**Mutation commands** (each does exactly one thing):

| Command | Transition | Side effects |
|---|---|---|
| `spec create` | → draft | Scaffold from template + git commit |
| `spec promote` | draft→ready→active | Advance one step, no side effects |
| `spec pause` | active→paused | WIP commit + tag + reason |
| `spec resume` | paused/blocked→active | Context diffs + tag |
| `spec block` | active→blocked | Tag + blocked_by + spec_deps |
| `spec complete` | active→complete | Release + archive + tag |
| `spec abandon` | any→abandoned | Archive + tag |
| `spec split` | active/paused→complete+draft | Release parent + scaffold child |

**What disappears:**
- `spec status` — replaced by `promote`, `complete`, `abandon`. No more
  one command doing 5 jobs. The escape hatch becomes
  `spec promote --force` for manual overrides.
- `spec archive` — absorbed into `complete` and `abandon` (they auto-
  archive). Keep `spec archive --stale` as a cleanup utility.

**All mutation commands support `--json`** for structured output that
MCP tools and adapter skills can parse.

### `/spec` Skill — Single Discovery Point

One skill describes the full capability. The LLM reads it once and
knows the entire surface area:

```
/spec — Manage spec lifecycle

MUTATIONS (change state):
  create   — scaffold new spec from conversation context
  promote  — advance: draft → ready → active
  pause    — park active work (reason required)
  resume   — restore paused/blocked work (shows context diffs)
  block    — mark blocked by another spec
  complete — ship it (release + archive)
  abandon  — kill it (archive)
  split    — ship done half, draft the rest

QUERIES (read-only):
  list     — all specs with filters (--status, --target, --json)
  ready    — what can be worked on now
  blocked  — what's stuck and why
  next     — recommended next spec

All commands support --json for structured output.
```

The skill includes guidance on WHEN to invoke each command (e.g.,
"when the user identifies a bug during spec work, offer to pause
the current spec and create a fix spec"). Drill-down to individual
command help via `patina spec <command> --help`.

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
**New commands:** `create`, `promote`, `pause`, `resume`, `block`,
`complete`, `abandon`, `split`, `next`
**Replaces:** `spec status` (decomposed into single-purpose commands)
**Removes:** `spec archive` (absorbed into `complete`/`abandon`)

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

**One paused spec at a time.** This is the core constraint. If you have
two paused specs, you're not pausing — you're avoiding. The queue isn't
working. Want to pause another? Resolve the existing one first: resume
it, split it, or abandon it.

Blocked is different — multiple blocked specs are fine because blocking
is a dependency, not a choice. But pausing is a choice, and the system
limits it.

```
$ patina spec pause another-spec --reason "New idea"
Error: spec-git-tag-system is already paused.
  Resume, split, or abandon it first:
    patina spec resume spec-git-tag-system
    patina spec split spec-git-tag-system
    patina spec status spec-git-tag-system abandoned
```

**In `spec list`:**
```
ID                    STATUS              AGE
spec-workflow-rigor   active              -
spec-git-tag-system   paused (12d)        ⚠ resolve before pausing another
spec-knowledge-evo    blocked             waiting on spec-workflow-rigor
```

**In `spec next`:**
- Active specs first (current work)
- Paused spec with age ("paused 12 days — resume, split, or abandon")
- Blocked specs with blocker status ("blocker spec-Y is now complete — resume?")
- Drafts available to promote

**In `/session-start`:**
```
Spec landscape:
  Active:  spec-workflow-rigor
  Paused:  spec-git-tag-system (12d) ⚠ — resolve before starting new work
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
pub paused_date: Option<String>,       // when paused (ISO 8601 UTC, D6)
pub paused_at_tag: Option<String>,     // tag ref for resume diffs (D4)
pub blocked_reason: Option<String>,    // why blocked
pub blocked_date: Option<String>,      // when blocked (ISO 8601 UTC, D6)
pub split_from: Option<String>,        // parent spec ID (set by split)
```

## Design Decisions

Resolved during external review. These are rules, not open questions.

### D1: WIP commit on pause — what if the tree is clean?

If the tree is clean, skip the WIP commit. Still create the tag. The tag
is the bookmark; the WIP commit is optional context. If you're mid-merge
or have unresolved conflicts, `spec pause` refuses — that's a
precondition, not an edge case.

**Preconditions for `spec pause`:**
- Spec must be `active`
- No unresolved merge conflicts
- No other spec already paused (one-paused-spec rule)

**If tagging or committing fails:** Roll back any YAML changes. The spec
stays `active`. User sees the error and retries.

### D2: Tag counter sequencing

Derive N by parsing existing tags: `git tag -l "spec/<id>-paused-*"`.
Count gives next N. Stateless — no counter in YAML, no DB state. Survives
rebases and manual tag deletions because tags are append-only in practice.
If a gap exists (paused-1, paused-3), that's fine — N is a sequence, not
a count.

### D3: `spec block` and the existing `blocked_by` / `spec_deps` relationship

`blocked_by` is already the field. `spec block` automates what you'd
hand-edit. The command writes YAML AND updates the DB inline (same pattern
as `update_spec_status()` which already does both). No scrape needed.
Multiple blockers are represented as a list — `blocked_by: [spec-A, spec-B]`.
`spec block` appends to the list; it doesn't overwrite.

### D4: Resume discovers the correct pause tag

Store the tag reference in YAML: `paused_at_tag: spec/X-paused-3`. Resume
reads it directly — no tag parsing, no ambiguity after multiple pause/resume
loops. The field is cleared on resume.

### D5: Split — new spec ID and path

Default new ID: `<parent-id>-v2` (or `-v3`, `-v4` if splits cascade).
User can override with `--id <custom-id>`. New spec lives in the standard
path: `layer/surface/build/feat/<new-id>/SPEC.md`. Version number in the
ID is a suffix for provenance, not semver.

### D6: Dates — format, timezone

ISO 8601, UTC. Same as every other date in the system (`created`, session
timestamps, `SpecFrontmatter.created`). Example: `2026-02-23`.

### D7: Session integration contract

The interface between session skills and queue data is defined during
Phase 5 implementation, based on the CLI output stabilized in Phases 1-4.
Specifying a JSON schema before the commands exist would be speculative.
Phase 5 wraps whatever the CLI produces.

## Implementation

### Phase 0: Spec Scaffolding — `spec create`

> **Carved out as its own spec.** `spec create` is the entry point to
> the entire lifecycle and needs its own spec covering: templates by
> type, LLM-driven parameter inference, the `/spec` skill definition,
> and MCP tool registration. This spec (`spec-workflow-rigor`) is
> `blocked_by: [spec-create]` once that spec exists.

**Why it matters:** Without `spec create`, the natural flow breaks.
Working on spec A → discover bug → `spec pause A` → ... now what?
You need `spec create fix the-bug` to scaffold the fix spec before you
can work on it or defer it. Creation is the entry point.

### Phase 1: Command Decomposition + State Machine

**Goal:** Decompose `spec status` into single-purpose commands. Add
`paused` and `blocked` statuses. Wire up the full mutation set.

**Changes to `src/commands/spec/mod.rs`:**
- Add new subcommands: `Promote`, `Pause`, `Resume`, `Block`,
  `Complete`, `Abandon` (and later `Split`, `Next`)
- Deprecate `Status` subcommand — print message redirecting to the
  new commands. Remove after one release cycle.
- Keep `Archive` only as `archive --stale` cleanup utility

**Changes to `src/commands/spec/internal.rs`:**
- Add `"paused"` and `"blocked"` to `VALID_STATUSES`
- Refactor `update_spec_status()` into focused internal functions that
  each new command calls
- Add transition validation: `paused`/`blocked` can only be set via
  their dedicated commands, not directly

**Changes to `src/spec.rs`:**
- Add `paused_reason`, `paused_date`, `blocked_reason`, `blocked_date`,
  `split_from` to `SpecFrontmatter` (all optional, skip_serializing_if)

**New git helpers in `src/git/operations.rs`:**
- `list_matching_tags(glob) -> Vec<String>` — for tag counter (D2)
- `create_tag_at(name, message, git_ref)` — tag a specific ref
- `has_merge_conflicts() -> bool` — check for `.git/MERGE_HEAD`

**New command: `patina spec promote <id>`:**
1. Validate current status allows promotion (draft→ready, ready→active)
2. Update YAML + DB (same pattern as current `update_spec_status()`)
3. If promoting to active: create tag `spec/<id>-start`
4. Git commit: `spec: promote <id> to <status>`
5. `--json` output

**New command: `patina spec complete <id>`:**
1. Validate spec is `active`
2. Delegate to `ReleaseStrategy` for version management
3. Auto-archive (tag + git rm + commit)
4. Git tag: `spec/<id>` (same as current archive flow)
5. `--json` output with release info

**New command: `patina spec abandon <id> [--reason "..."]`:**
1. Validate spec exists (any status except already abandoned)
2. Auto-archive (tag + git rm + commit)
3. Git tag: `spec/<id>` with reason annotation
4. `--json` output

**New command: `patina spec pause <id> --reason "..."`:**
1. Check no other spec is already paused (one-paused-spec rule)
2. Validate spec is `active`, no unresolved merge conflicts (D1)
3. Create WIP commit if uncommitted changes exist; skip if tree clean (D1)
4. Update YAML: status → `paused`, set `paused_reason`, `paused_date`,
   `paused_at_tag` (D4)
5. Derive tag N from existing tags (D2), create annotated tag:
   `spec/<id>-paused-<N>` with reason as message
6. Update DB inline (same as `update_spec_status()`)
7. Git commit: `spec: pause <id> — <reason>`
8. Log in active session
9. If any step fails after YAML mutation, roll back YAML (D1)

**New command: `patina spec resume <id>`:**
1. Validate spec is `paused` or `blocked`
2. If `blocked`: check all blockers complete (error if not, `--force`)
3. Read `paused_at_tag` from YAML for diff reference (D4)
4. Update YAML: status → `active`, clear pause/block fields
5. Derive tag N, create annotated tag: `spec/<id>-resumed-<N>`
6. Update DB inline
7. Git commit: `spec: resume <id>`
8. Show context diffs:
   - `git diff <paused_at_tag>..HEAD` — what changed while away
   - `git diff spec/<id>-start..<paused_at_tag>` — what you accomplished
9. Log in active session

**New command: `patina spec block <id> --by <blocker> --reason "..."`:**
1. Validate spec is `active`
2. Update YAML: status → `blocked`, append to `blocked_by` list (D3),
   set `blocked_reason`, `blocked_date`, `paused_at_tag`
3. Derive tag N, create annotated tag: `spec/<id>-blocked-<N>`
4. Update DB inline (patterns table + spec_deps insert) (D3)
5. Git commit: `spec: block <id> (waiting on <blocker>)`
6. Log in active session

**Exit criteria:**
- [x] `spec status` deprecated — prints redirect message
- [x] `spec promote` advances draft→ready→active, tags on active
- [x] `spec complete` triggers release + archive + tag
- [x] `spec abandon` archives + tags, accepts optional reason
- [x] `paused` and `blocked` are valid statuses
- [x] `spec pause` enforces one-paused-spec rule
- [x] `spec pause` creates WIP commit (if dirty) + tag + updates YAML + DB
- [x] `spec pause` with clean tree skips WIP commit, still tags
- [x] `spec resume` reads `paused_at_tag`, shows context diffs, restores active
- [x] `spec block` appends to `blocked_by` list + updates DB inline
- [x] Tags follow `spec/<id>-paused-N` / `spec/<id>-blocked-N` convention
- [x] Tag N derived from existing tags (D2)
- [x] Invalid transitions rejected (draft → paused, paused → complete)
- [x] YAML rollback on failure (D1) — `with_yaml_rollback()` used by both `pause` and `block`
- [x] All mutation commands support `--json` output

**Implementation deviations** (session [[session-20260223-162443]]):

1. ~~**`with_yaml_rollback()` not extracted as shared function.**~~ **Resolved** (session
   [[session-20260223-170149]]). Extracted `with_yaml_rollback()` as shared function.
   Both `pause_spec()` and `block_spec()` now use it for YAML rollback on failure.

2. **`complete_spec()` and `abandon_spec()` don't use `mutate_spec()`.** They
   do their own read/parse/write because `complete` needs the frontmatter for
   release bump detection and `abandon` needs to pass the file path to
   `archive_spec_inner()`. Same behavior, different code path. Acceptable —
   `mutate_spec()` is for the simpler commands.

3. ~~**Private `create_spec_tag()` not fully replaced.**~~ **Resolved** (session
   [[session-20260223-170149]]). Removed `create_spec_tag()`, all call sites now
   use `patina::git::create_tag_at()` directly.

4. **`--force` flag not added to `promote`.** Design mentioned
   `spec promote --force` for manual overrides. Not implemented — promote
   only does valid transitions, no override needed. Revisit if edge cases
   emerge.

5. **`paused_at_tag` reused for block tag ref.** `block_spec()` stores the
   block tag in `paused_at_tag` so `resume` can find it. Field name is
   misleading for blocked specs. Could rename to `state_change_tag` but
   that's a YAML schema change — defer.

6. **`blocked_by` list not cleared on resume.** Deliberate: preserves
   provenance (who blocked whom). `spec_deps` DB entries are cleared.
   YAML keeps historical record. Design didn't explicitly decide this.

### Phase 2: Spec Split

**Goal:** Ship done work, draft remaining work as new spec.

**New command: `patina spec split <id> [--id <new-id>]`:**
1. Validate spec is `active` or `paused`
2. Prompt: "Describe what's done" (used for release commit)
3. Tag current state: `spec/<id>-v<N>-complete`
4. Complete original spec (normal release flow: version bump, archive, tag)
5. New ID defaults to `<parent-id>-v2` (or `-v3` etc.), user can
   override with `--id` (D5)
6. Create new spec directory: `layer/surface/build/feat/<new-id>/SPEC.md`
   - Frontmatter includes `split_from: <parent-id>`
   - Status: `draft`
   - Body: user-provided description of remaining work
7. Git commit: `spec: split <id> — ship v<N>, draft remainder as <new-id>`

**Exit criteria:**
- [x] `spec split` completes original spec with release
- [x] New draft spec created with `split_from` provenance
- [x] Parent archived with tag `spec/<id>-v<N>-complete`
- [x] New spec references parent tag for recovery
- [x] `git show spec/<id>:...` recovers original spec content

**Implementation deviations** (session [[session-20260223-170149]]):

1. **No interactive prompt for "describe what's done".** Spec step 2 said
   "Prompt: describe what's done." Implemented as `--description` flag instead,
   with default text "Remaining work from split". CLI commands shouldn't block
   on interactive input — flags are composable.

2. **Flag name `--new-id` instead of `--id`.** Spec said `--id <new-id>`.
   Used `--new-id` because `<id>` is already the positional arg for the spec
   being split — `--id` would be ambiguous. Better UX, minor deviation.

3. **No `--major` flag on split.** `complete_spec()` accepts `--major` for
   1.0.0 bumps. `split_spec()` always uses `BumpType::from_spec_type()` —
   no major override. Edge case — fixable if needed.

4. **New spec gets a `## Recovery` section.** Spec didn't mention this.
   Added so the new draft contains the exact `git show` command to recover
   parent content. Strictly additive.

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
- Show one-paused-spec constraint status

**Exit criteria:**
- [x] `spec next` recommends a spec with reasoning
- [x] `spec ready` shows impact and paused/blocked status
- [x] `spec list` shows age for paused/blocked specs
- [x] Paused spec shown with "resolve before pausing another"

**Implementation deviations** (session [[session-20260223-170149]]):

1. **Impact is a tiebreaker, not its own rank tier.** Spec listed "Impact:
   blocks N other specs" as a separate ranking level. Implemented as a
   tiebreaker within each priority tier (active > unblocked > paused > ready
   > draft). High-impact specs surface first within their category — more
   useful than a flat impact-only tier.

2. **`spec ready` JSON output unchanged.** Enhanced view (paused section,
   unblocked section, impact counts) only appears in human output. JSON
   contract still returns only ready/active specs from `get_ready_specs()`.
   Deliberate — JSON consumers shouldn't get unexpected shape changes.

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

**4e. Fix development branch assumption:**
Session system (`session/internal.rs`) hardcodes `work` as the development
branch. Release system (`release/internal.rs`) expects `patina` branch.
They disagree. Fix: read the development branch from `.patina/config.toml`
(e.g., `[project] branch = "patina"`) and use it consistently in both
session start branch-switching and release safeguard checks.

**Exit criteria:**
- [x] `patina session list` shows active/stale/recent sessions
- [x] `session start` warns when archiving a session >24h old
- [x] `session end` flips status before archiving (atomic-first)
- [x] Session CLI commands return structured summary to stdout
- [x] Session and release agree on development branch (configurable)

**Implementation deviations** (session [[session-20260223-185307]]):

1. **`session list` output format differs from spec example.** Spec shows
   `(21d stale)` suffix on ACTIVE line and `(completed, 2d ago)` on RECENT.
   Implementation shows `(Nd ago)` for all entries, with `(Nd ago, never ended)`
   for STALE entries. ACTIVE line shows age since creation, not staleness.
   Deliberate — spec example conflated ACTIVE with STALE (showed a stale
   session on the ACTIVE line). Implementation separates the three categories
   cleanly: ACTIVE = current session, STALE = never ended, RECENT = archived.

2. **`session list` scans all 748 archived sessions.** Spec mentions querying
   `layer/sessions/` but doesn't address performance. Implementation reads
   all `.md` files sorted by filename descending, stops collecting RECENT
   after 5 but continues scanning for STALE (they could be anywhere).
   Acceptable — 748 files parse in <100ms, and stale detection requires
   full scan. Could optimize with session index later if count grows.

3. **4d underspecified — "richer CLI output" interpreted as additive stdout.**
   Spec said "CLI returns structured summary to stdout that the skill can use
   directly. Fewer LLM interpretation steps." Implementation adds: previous
   session reference printed in `session start` stdout, commit SHAs + changed
   file paths printed in `session update`, `[[commit-SHA]]` wikilinks in
   the update markdown section. But spec also said "Skill markdown stays the
   same — just relies less on file reads." The skill definitions in
   `resources/claude/session-*.md` were **not updated** to rely on stdout
   instead of file reads. Gap — skills still instruct the LLM to read
   `.patina/local/active-session.md` and `last-session.md`. The CLI is
   richer but the skill-side wiring is deferred. Natural fit for Phase 5
   (Session Integration) which rewrites the skill instructions anyway.

4. **4d focused on `start` and `update`, barely touched `end`.** Spec said
   all three commands get richer output. `session end` already had the
   richest output (session summary, metrics, tag range, beliefs, archive
   confirmation). No meaningful changes needed — it was already structured.
   Minor deviation from spec's "all three" framing.

5. **Atomic status flip uses `completed` intermediate state.** Spec says
   `status: active → completed` then archive flips to `archived`.
   Implementation does exactly this. Archive step handles both
   `completed → archived` and `active → archived` for robustness (covers
   edge case where atomic flip didn't run). Matches spec intent.

6. **`session list` spec example has no `--json` flag.** Spec says "New
   subcommand querying..." but doesn't mention JSON output. Implementation
   adds `--json` for consistency with all other Patina list/query commands
   (`spec list --json`, `spec ready --json`, etc.). Additive — follows
   project convention.

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
- [x] `/session-start` shows spec landscape with recommendations
- [x] `/session-update` tracks spec status changes
- [x] `/session-end` suggests next spec and confirms pause reasons

**Implementation deviations** (session [[session-20260223-201417]]):

1. **Wired into data functions, not CLI print functions.** Spec said "Run
   `patina spec next`, show spec landscape." Implementation calls
   `get_all_specs()`, `get_blocked_specs()`, and `load_dep_counts()` directly
   instead of the CLI-facing `next_spec()` / `show_ready_specs()` which print
   to stdout. Required re-exporting data functions from `spec/mod.rs` as
   `pub(crate)`. Cleaner — avoids mixing two commands' stdout output.

2. **Recommendation logic simplified from `next_spec`.** Full `next_spec()`
   has 6 priority tiers with impact tiebreaking. Session landscape uses a
   simpler 4-tier check: active > paused > ready (by impact) > draft.
   Sufficient for session context — users who want full ranking use
   `patina spec next` directly.

3. **`/session-end` does not confirm pause reasons.** Spec exit criterion says
   "confirms pause reasons." Since `spec pause` requires `--reason` (mandatory
   flag), a paused spec without a reason cannot exist. Checking for missing
   reasons is dead code. Removed the check — the constraint is enforced at
   mutation time, not at session end.

4. **Skill markdown updated to reference stdout, resolving Phase 4 D3.** Phase 4
   deviation D3 noted that skill instructions still told the LLM to read files
   for info already in stdout. Updated all three session skills to reference
   stdout for: branch/tag (start), commit SHAs/files (update), spec
   landscape/recommendations (start/end), and unblocked specs (end).

5. **`/session-update` doesn't show blocking/unblocking events.** Spec said
   "Show blocking/unblocking events." Implementation only shows which SPEC.md
   files were changed and warns about paused specs aging. No before/after
   status comparison — that would require snapshotting spec states at session
   start, adding state management complexity for minimal value. The
   changed-files approach is lighter and sufficient.

6. **Unblock detection is state-based, not event-based.** Spec said "If spec
   completed, show what it unblocks." Implementation queries
   `get_blocked_specs()` for specs whose blockers are all complete — it doesn't
   know if *this session* caused the unblock. Semantic difference that rarely
   matters in practice since the output is still correct ("these specs are now
   unblocked"). Event-based detection would require a session-start snapshot.

### Phase 6: MCP Tools + `/spec` Skill

**Goal:** Expose all spec commands as MCP tools and write the unified
`/spec` adapter skill. Completes the 3-prong bundle.

**6a. MCP tool registration:**
Register each spec command as an MCP tool in `src/mcp/`. Same Rust
functions the CLI calls — MCP is just a different entry point. Each
tool has typed parameters and returns structured JSON.

Tools: `spec_list`, `spec_ready`, `spec_blocked`, `spec_next`,
`spec_create`, `spec_promote`, `spec_pause`, `spec_resume`,
`spec_block`, `spec_complete`, `spec_abandon`, `spec_split`

**6b. `/spec` adapter skill:**
Single skill definition in `resources/claude/spec.md` (and equivalent
for other adapters). Describes the full capability with:
- Command menu (mutations + queries)
- When to invoke each command (LLM judgment guidance)
- How to fill parameters from conversation context
- How to present results to the user

**6c. Plugin manifest (design only):**
Document the shape of a future plugin manifest that bundles CLI
commands, MCP tools, and skill definition. Don't implement the plugin
runtime — just ensure the spec system's interfaces are clean enough
that extraction to WASM is possible later.

**Exit criteria:**
- [x] All spec commands available as MCP tools
- [x] `/spec` skill works with Claude adapter
- [x] LLM can discover, select, and invoke spec tools from conversation
- [x] Plugin interface shape documented (WIT contract sketch)

**Implementation deviations** (session [[session-20260223-220302]]):

1. **Tool naming uses dots not underscores.** Spec listed tools as `spec_list`,
   `spec_ready`, etc. Implementation uses `spec.list`, `spec.ready` — matching
   the existing `schemas.list`/`schemas.show` convention in the MCP server.
   Dots are idiomatic for MCP tool namespacing.

2. **`spec_create` not registered.** Spec listed 12 tools including `spec_create`.
   Phase 0 carved out `spec create` as its own spec — it doesn't exist yet.
   11 tools registered (4 query + 7 mutation), matching what the CLI provides.

3. **Query tools return serialized JSON text, not structured MCP content.**
   All MCP tools wrap results in `{"content": [{"type": "text", "text": ...}]}`
   per MCP protocol. Query results are `serde_json::to_string_pretty()` of the
   Rust types (SpecInfo, ReadySpec, BlockedSpec). Mutation results are the same
   `serde_json::Value` the CLI's `--json` path produces. The text wrapping is
   the established pattern (schemas.list, schemas.show do the same).

4. **`_value()` pattern applied to all mutations + next_spec.** Spec's approach
   (a) said "add _value() variants." Implementation extracts the JSON block and
   has the original function delegate to the _value() variant for both JSON and
   human output. Slightly more refactoring than "extract" — the original functions
   now parse from the Value for their human-readable printing. Cleaner separation.

5. **Skill is Claude-only, not multi-adapter.** Spec said "resources/claude/spec.md
   (and equivalent for other adapters)." Only Claude adapter skill created — no
   other adapters exist yet. Will scaffold others when adapters are added.

6. **Plugin manifest is inline in SPEC.md, not a separate design doc.** Spec said
   "document the shape" — a separate file would be over-engineering for a sketch
   that exists to prove the interface is clean. Inline keeps it co-located with
   the spec that motivated it.

**Plugin manifest design (6c):**

The spec system's current `mod.rs` public API maps cleanly to a WIT contract.
Each public function becomes a WIT function export. The types already derive
Serialize, so they cross the WASM boundary as JSON.

```wit
// Hypothetical spec-plugin WIT contract
package patina:spec@0.1.0;

interface queries {
    record spec-info {
        id: string,
        status: option<string>,
        target: option<string>,
        title: string,
        unscraped: bool,
    }

    record ready-spec {
        id: string,
        status: string,
        target: option<string>,
        title: string,
    }

    record blocker { id: string, status: string }
    record blocked-spec {
        id: string,
        status: string,
        target: option<string>,
        title: string,
        blocked-by: list<blocker>,
    }

    record recommendation {
        id: string,
        status: string,
        reason: string,
        priority: u32,
        impact: u32,
    }

    record list-filters {
        status: option<string>,
        target: option<string>,
    }

    list-specs: func(filters: list-filters) -> result<list<spec-info>, string>;
    ready-specs: func() -> result<list<ready-spec>, string>;
    blocked-specs: func() -> result<list<blocked-spec>, string>;
    next-spec: func() -> result<list<recommendation>, string>;
}

interface mutations {
    record mutation-result {
        command: string,
        spec-id: string,
        new-status: string,
        // Additional fields vary by command — JSON blob
        details: string,
    }

    promote: func(id: string) -> result<mutation-result, string>;
    complete: func(id: string, major: bool) -> result<mutation-result, string>;
    abandon: func(id: string, reason: option<string>) -> result<mutation-result, string>;
    pause: func(id: string, reason: string) -> result<mutation-result, string>;
    resume: func(id: string, force: bool) -> result<mutation-result, string>;
    block: func(id: string, by: string, reason: string) -> result<mutation-result, string>;
    split: func(id: string, new-id: option<string>, description: option<string>) -> result<mutation-result, string>;
}

world spec-plugin {
    export queries;
    export mutations;
}
```

**Key observations for extraction:**
- `spec/mod.rs` public API is already the WIT shape — each `pub fn` maps 1:1
- Types derive Serialize → cross WASM boundary as JSON (serde_json::Value)
- `internal.rs` stays behind the black-box boundary — only `mod.rs` exports matter
- The three prongs (CLI dispatches to mod.rs, MCP dispatches to mod.rs, skill
  describes mod.rs) all route through the same interface. Plugin extraction
  replaces the dispatch, not the logic.

### Phase 7: Completion Cleanup

**Goal:** Wire the last adapter seam, remove deprecated code, complete spec.

**7a. Wire `/spec` skill command:**
The `/spec` skill definition existed at `resources/claude/spec.md` since Phase 6
but wasn't deployed to `.claude/commands/spec.md`. Wired into both deployment
paths: `templates.rs` (used by `adapter refresh`) and `session_scripts.rs`
(used by `init_project`/`update_adapter_files`). Added compile-time embed,
template installation, and test assertions.

**7b. Remove `spec status` subcommand:**
Deprecated in Phase 1 (printed redirect messages). Removed:
- `SpecCommands::Status` variant from `mod.rs`
- `pub fn status()` from `mod.rs`
- `update_spec_status()` from `internal.rs` (160 lines)
- `VALID_STATUSES` constant (only consumer was removed function)
- Dispatch case in `main.rs`
- Tests for `Status` variant and `VALID_STATUSES`
- Updated stale `patina spec status <id> complete` references in
  `version/mod.rs` and `version/internal.rs`

**7c. Spec completion:**
Run `patina spec complete spec-workflow-rigor` to trigger release + archive.

**Exit criteria:**
- [x] `/spec` skill deployed via `adapter refresh` and `init_project`
- [x] `spec status` subcommand fully removed
- [x] Stale references to `spec status` updated
- [x] Spec completed with release + archive

**Implementation deviations:**

1. **Two deployment paths required wiring.** Expected only one file
   (`.claude/commands/spec.md`). Discovered the template system has two paths:
   `session_scripts.rs` (for init/update) and `templates.rs` (for adapter
   refresh). Both needed the spec.md embed. The templates.rs path is canonical
   for `adapter refresh` which is the primary deployment mechanism.

2. **`VALID_STATUSES` removed despite spec saying "Keep."** The prompt said
   "Keep VALID_STATUSES constant (still used by other code)." Compiler showed
   it was only used by `update_spec_status()` and a self-referential test —
   no other consumers. Dead code removed. The valid status list is enforced
   by individual mutation commands' transition validation, not a central
   constant.

## Testing

### Manual Test Cases

**Test 1: Promote through lifecycle**
```bash
patina spec promote my-spec          # draft → ready
patina spec promote my-spec          # ready → active (tags spec/my-spec-start)
patina spec promote my-spec          # Error: already active, use complete/pause/block
```

**Test 2: Pause and resume**
```bash
patina spec pause my-spec --reason "Discovered need for auth first"
# → WIP commit, tag spec/my-spec-paused-1, status: paused

patina spec list
# → my-spec shows "paused (0d)"

patina spec resume my-spec
# → Shows context diffs, status: active, tag spec/my-spec-resumed-1
```

**Test 3: Block and unblock**
```bash
patina spec block my-spec --by auth-spec --reason "Need auth first"
# → status: blocked, blocked_by: [auth-spec], tag spec/my-spec-blocked-1

patina spec resume my-spec
# → Error: Still blocked by auth-spec (draft)

patina spec complete auth-spec
patina spec resume my-spec
# → Success: status active, context diffs shown
```

**Test 4: Complete and abandon**
```bash
patina spec complete my-spec
# → Release (version bump) + archive (tag + git rm) + commit

patina spec abandon other-spec --reason "Superseded by new approach"
# → Archive (tag + git rm) + commit, no release
```

**Test 5: Split**
```bash
patina spec split my-spec
# → Prompts for what's done
# → Completes my-spec (release + archive)
# → Creates my-spec-v2 as draft with split_from: my-spec
# → Tag: spec/my-spec-v1-complete

git show spec/my-spec:layer/surface/build/feat/my-spec/SPEC.md
# → Original spec recovered
```

**Test 6: One-paused-spec constraint**
```bash
patina spec pause spec-A --reason "Exploring alternatives"
# → Success: spec-A paused

patina spec pause spec-B --reason "New idea"
# → Error: spec-A is already paused.
#   Resume, split, or abandon it first.

patina spec abandon spec-A --reason "No longer needed"
patina spec pause spec-B --reason "New idea"
# → Success: spec-B paused (spec-A resolved)
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

1. ~~Should `patina session list` also query archived sessions in
   `layer/sessions/`, or only active + `.patina/local/`?~~ → **Yes, query
   both.** Implementation scans `layer/sessions/` for STALE (status: active,
   never ended) and RECENT (last 5 archived), plus `.patina/local/active-session.md`
   for ACTIVE. All three categories are useful: ACTIVE shows current work,
   STALE surfaces forgotten sessions, RECENT provides continuity context.
2. Should `patina doctor` check for stale sessions and paused specs?

## Resolved Questions

3. ~~Staleness threshold for paused specs~~ → **No day-based threshold.**
   Pressure comes from the one-paused-spec constraint: you can't pause
   another until you resolve the existing one. The queue itself is the
   pressure, not a timer.
4. ~~Split auto-generate ID or prompt?~~ → **Default `<id>-v2`, override
   with `--id`.** (D5)
5. ~~WIP commit required on pause?~~ → **Optional.** If tree is clean,
   skip WIP commit, still create tag. Tag is the bookmark. (D1)

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
- "I want to pause this and start something else" → must resolve existing pause first
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
# Spec system (CLI layer)
src/spec.rs                          — SpecFrontmatter struct (add new fields)
src/commands/spec/mod.rs             — public API, clap subcommands (decompose into commands)
src/commands/spec/internal.rs        — all logic: transitions, queries, archive
src/git/operations.rs                — git helpers (add tag listing, ref-based tagging)
src/release/                         — ReleaseStrategy (used by spec complete)

# MCP layer (Phase 6)
src/mcp/                             — MCP tool registration for spec commands

# Skill layer (Phase 6)
resources/claude/spec.md             — /spec skill definition (unified discovery)

# Session system (Phase 4-5)
src/commands/session/mod.rs          — session public API
src/commands/session/internal.rs     — session lifecycle logic
resources/claude/session-*.md        — session skill instructions

# Future: plugin extraction
src/commands/spec/mod.rs             — public API becomes WIT contract shape
```
