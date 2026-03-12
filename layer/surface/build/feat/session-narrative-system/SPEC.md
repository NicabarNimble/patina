---
type: feat
id: session-narrative-system
status: complete
created: 2026-03-11
related:
- agentic-surface-architecture
- patina-ai-interface-layer
- persona-federation
- continuous-operation
- spec-subsystem-plugin
beliefs:
- session-capture
- spec-driven-design
- safety-boundaries
- dependable-rust
- unix-philosophy
- mother-is-connection-and-continuity
- session-git-trail-is-sacred
sessions:
  origin: 20260310-230611
exit_criteria:
- id: many-active-sessions
  text: Mother-backed live session state supports many active sessions per project rather than a single active-session singleton
  checked: true
- id: durable-artifacts-preserved
  text: Durable session artifacts remain in layer/sessions with same-or-better git tags, commit capture, and reviewability as the current system
  checked: true
- id: collision-safe-identities
  text: Session IDs are collision-safe for concurrent human and agent starts while remaining ordered and readable
  checked: true
- id: participants-and-handoffs
  text: Sessions capture participants, interfaces, persona context, and handoff relationships explicitly
  checked: true
- id: no-singleton-dependence
  text: The new path no longer depends on .patina/local/active-session.md as the only live coordination primitive
  checked: true
- id: semantic-structure
  text: Session artifacts gain enough structured narrative to support later extraction into semantic datablocks and belief evidence
  checked: true
---
# feat: Session Narrative System

> Replace the single-active-session design with a truthful multi-session narrative system: many active sessions per project, Mother-backed live session state, durable git-linked artifacts in layer/sessions, and strong handoff/provenance semantics for human and agent work.

## Problem

The current session system is valuable but structurally too narrow for
the next Patina model.

What it does well today:

- deep git integration
- human-readable markdown artifacts
- low-friction start/update/note/end workflow
- durable records in `layer/sessions/`

What it cannot do well:

- more than one active session per project
- truthful multi-interface or swarm-style concurrent work
- session attachment independent of a single local file
- explicit handoffs between human and agent work
- richer narrative semantics beyond a lightweight activity log

The current implementation encodes this limit directly:

- one `.patina/local/active-session.md`
- one `.patina/local/last-session.md`
- timestamp-only IDs with second-level granularity

That was a good first system. It is not enough for a multiplayer,
agentic, Mother-brokered future.

## Solution

### 1. Split live session state from durable session artifacts

The new system has two layers:

- **live session state** managed by Mother
- **session artifacts** committed in `layer/sessions/`

The live layer coordinates reality. The artifact layer tells the
reviewable story.

**Explicit implementation target:**

- live session state should initially live in `~/.patina/mother/runtime.db`
  beside the existing Mother runtime tables, not in a new one-off
  project-local singleton file
- session artifacts remain project-local in `layer/sessions/`
- the session scraper in `src/commands/scrape/layer/sessions.rs` is part
  of the contract and must continue to parse both existing and new
  artifacts

### 2. Let many sessions be active at once

Patina should support many active sessions per project:

- one human in OpenCode
- one human in Gemini
- a web interface actor
- one or more autonomous agents

These are not fake sub-sections of one singleton session. They are real
parallel sessions with explicit relationships.

### 3. Preserve the git trail as a first-class invariant

The git/session trail is sacred and must survive the redesign:

- session start/end tags remain
- commit range and changed-file reporting remain
- session records stay human-reviewable
- artifacts still live in `layer/sessions/`

The new system may improve the trail, but it must not weaken it.

### 4. Add explicit participant and handoff structure

A session should record:

- participants
- interface type
- persona context
- parent/handoff relationships
- linked specs
- notable decisions/evidence/outcomes

This turns sessions into controlled narrative, not just process logs.

### 5. Make IDs collision-safe and runtime-safe

Use human-orderable IDs with uniqueness beyond second-level timestamps.
The file-visible ID can stay time-oriented, but the runtime system needs
collision safety for concurrent starts.

**Decision:** use two identifiers:

- `runtime_id`: UUID/opaque unique key for Mother-managed live state
- `file_id`: ordered archive ID for markdown and git tags,
  `YYYYMMDD-HHMMSS-<suffix>`

The suffix should be short, deterministic enough for readability, and
safe for concurrent starts in the same second.

### 6. Prepare session artifacts for semantic afterlife

Session artifacts are future raw material for:

- handoff context
- datablock extraction
- belief evidence
- provenance queries about why work happened

So the markdown shape should gain stronger narrative structure without
becoming onerous to capture.

## Implementation Sequence

### Commit 1: `refactor(session): extract session document model from singleton flow`

Separate the markdown/git artifact model from the current
single-active-session mechanics.

**File targets:**

- `src/commands/session/internal.rs` — shrink toward compatibility
  wrapper behavior
- `src/commands/scrape/layer/sessions.rs` — preserve artifact contract
- new `src/session/` module with narrow public API and private internals

### Commit 2: `feat(session): add Mother-backed live session registry`

Introduce Mother-owned live session state with many active sessions,
participants, and leases.

**File targets:**

- `src/mother/state.rs` — add session tables in `runtime.db`
- `src/mother/mod.rs` — export narrow live-session types only
- new `src/session/internal/live.rs`

### Commit 3: `feat(session): project live state to durable git-linked artifacts`

Keep `layer/sessions/` and git tags as the durable review path.

**File targets:**

- new `src/session/internal/artifact.rs`
- new `src/session/internal/projection.rs`
- `src/commands/session/internal.rs`

### Commit 4: `feat(session): add participant, handoff, and persona structure`

Make session narrative richer and more useful for review.

**File targets:**

- `src/commands/scrape/layer/sessions.rs` — extend parsed YAML/body
  fields without breaking legacy parse
- `src/commands/spec/internal/create.rs` — stop assuming only one local
  active session source
- `src/commands/scry/internal/logging.rs` — move away from direct
  singleton lookup

### Commit 5: `feat(session): support compatibility and new interfaces together`

Allow the existing compatibility path and `patina ai` path to coexist
truthfully during migration.

**Compatibility rules:**

- `patina session start/update/note/end` must continue to work
- `.patina/local/active-session.md` may remain as compatibility state for
  the old path, but `patina ai` must not depend on it as source of truth
- the old session commands should project into the new session runtime,
  not fork the artifact model

## Exact Build Contract

Another agent should implement this spec in this order:

1. Introduce `src/session/mod.rs` with a small public API:
   - create/attach live session
   - append observation/update
   - end session
   - project artifact
2. Move document parsing/rendering and git envelope logic under
   `src/session/internal/`
3. Add session tables to `~/.patina/mother/runtime.db`
4. Keep `src/commands/session/mod.rs` as thin CLI routing
5. Update singleton readers in spec/scry/session code to use the new API
   where appropriate
6. Extend the session scraper last, once the artifact shape is stable

## Verification

- unit tests for ID generation and collision safety
- unit tests for artifact rendering/parsing compatibility
- unit tests for new runtime tables in `src/mother/state.rs`
- regression tests for `src/commands/scrape/layer/sessions.rs`
- command-level tests or scripted verification that:
  - old `patina session` still works
  - parallel sessions can coexist
  - archived artifacts land in `layer/sessions/`
  - git tags are created with the new file ID

## Exit Criteria

1. Mother-backed live session state supports many active sessions per
   project rather than a single active-session singleton
2. Durable session artifacts remain in `layer/sessions` with
   same-or-better git tags, commit capture, and reviewability as the
   current system
3. Session IDs are collision-safe for concurrent human and agent starts
   while remaining ordered and readable
4. Sessions capture participants, interfaces, persona context, and
   handoff relationships explicitly
5. The new path no longer depends on `.patina/local/active-session.md`
   as the only live coordination primitive
6. Session artifacts gain enough structured narrative to support later
   extraction into semantic datablocks and belief evidence

## Non-Goals

- replacing the current session trail with opaque runtime state
- inventing fake historical capture for work that already happened
- turning sessions into the execution engine for children
- making interfaces own durable narrative truth
