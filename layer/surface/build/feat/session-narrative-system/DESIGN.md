# Design: Session Narrative System

## Why This Exists

The current session system got the important thing right early:

- git tags bracket work
- markdown artifacts are readable
- capture is fast enough to actually happen

But it encoded "one local active session" as the coordination model.
That is what now blocks parallel interfaces, swarms, and truthful
handoffs.

This design preserves the artifact quality while changing the live
coordination model underneath it.

## Core Split

### 1. Live session state

Mother-managed runtime state:

- many active sessions
- participant seats / presence
- leases
- persona attachment
- interface attachment

**Storage decision:** use `~/.patina/mother/runtime.db`
(`src/mother/state.rs`) for the first implementation. Do not create a
second independent runtime database for sessions.

### 2. Session artifact

Git-backed markdown in `layer/sessions/`:

- readable by humans
- linked to tags and commits
- durable across restarts and machines
- source material for later semantic extraction

The live state is not the review artifact. It is the coordination layer
that makes the artifact truthful in a multiplayer world.

The artifact parser in `src/commands/scrape/layer/sessions.rs` is part
of the compatibility surface. New artifacts must extend, not break, that
contract.

## Session Shape

Suggested top-level concepts:

```rust
struct LiveSession {
    runtime_id: SessionRuntimeId,
    file_id: SessionFileId,
    title: String,
    persona_uid: Option<String>,
    participants: Vec<ParticipantSeat>,
    status: SessionStatus,
    git: GitEnvelope,
}
```

Suggested initial runtime tables in `runtime.db`:

```sql
CREATE TABLE mother_sessions (
    runtime_id TEXT PRIMARY KEY,
    project_uid TEXT NOT NULL,
    file_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    persona_uid TEXT,
    status TEXT NOT NULL,
    branch TEXT,
    start_tag TEXT,
    end_tag TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE mother_session_participants (
    session_runtime_id TEXT NOT NULL,
    participant_id TEXT NOT NULL,
    role TEXT NOT NULL,
    interface_kind TEXT,
    adapter_name TEXT,
    display_name TEXT,
    joined_at TEXT NOT NULL,
    left_at TEXT,
    PRIMARY KEY (session_runtime_id, participant_id, joined_at)
);

CREATE TABLE mother_session_handoffs (
    id INTEGER PRIMARY KEY,
    from_runtime_id TEXT NOT NULL,
    to_runtime_id TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL
);
```

Start with these three tables. Do not overdesign a session event store
until the artifact projection is working.

The durable artifact should remain markdown-first, but its frontmatter
can grow to include:

- participants
- interfaces
- persona
- parent_session
- handoff_from / handoff_to
- start/end git tags

And the body should reserve stronger sections for:

- goals
- activity
- decisions
- evidence
- handoff
- outcome

## Identity Strategy

Timestamp ordering is useful but not sufficient. The system should use:

- a human-orderable file ID
- a collision-safe runtime ID

This avoids clobbering in parallel starts while keeping the archive easy
to scan.

Concrete decision:

- `runtime_id`: UUIDv4 or equivalent opaque identifier
- `file_id`: `YYYYMMDD-HHMMSS-<base32_4>`
- git tags continue to use `file_id`:
  - `session-<file_id>-<adapter>-start`
  - `session-<file_id>-<adapter>-end`

## Compatibility Strategy

The compatibility path should continue to work during migration.

That means:

- old `patina session` flow can remain as a thin compatibility wrapper
- new interfaces should not depend on `.patina/local/active-session.md`
  as the sole source of truth
- archived artifacts and git tags remain the shared durable outputs

Compatibility shims that must be updated during build:

- `src/commands/spec/internal/create.rs`
- `src/commands/scry/internal/logging.rs`
- any direct readers of `.patina/local/active-session.md`

## Semantic Strategy

Sessions are not merely logs. They are controlled narrative.

That means the design should support future extraction of:

- decisions
- tradeoffs
- why-context
- evidence links
- handoff state

without requiring a giant schema or making capture burdensome.

Recommended durable markdown frontmatter additions:

- `participants`
- `persona`
- `interfaces`
- `parent_session`
- `handoff_from`
- `handoff_to`

Recommended durable body sections:

- `## Goals`
- `## Activity Log`
- `## Decisions`
- `## Evidence`
- `## Handoff`
- `## Outcome`

The old sections can continue to parse, but this is the target shape.

## Rust Design Rules

- keep the public session API narrow and stable per [[dependable-rust]]
- isolate live-store internals from artifact rendering internals
- prefer distinct modules for live state, artifact projection, and git
  capture rather than one giant session manager
- preserve low-friction capture per [[session-capture]]

## Exact File Targets

Introduce these modules explicitly:

- new `src/session/mod.rs` — narrow public API
- new `src/session/internal/live.rs` — Mother-backed runtime logic
- new `src/session/internal/artifact.rs` — markdown/frontmatter model
- new `src/session/internal/projection.rs` — runtime -> artifact bridge
- new `src/session/internal/ids.rs` — `runtime_id` and `file_id`
  generation
- keep `src/commands/session/mod.rs` as routing
- shrink `src/commands/session/internal.rs` toward compatibility glue

## Smallest Safe Sequence

1. Extract the session document rendering/parsing model from the current
   singleton command implementation.
2. Introduce Mother-backed live session storage and seat management.
3. Project live session truth into the durable markdown/git trail.
4. Add participant, persona, and handoff structure.
5. Update `patina ai` to use the new live session path.

6. Update the scraper only after the artifact shape is stable enough to
   commit.

## Key Files

- `src/commands/session/internal.rs` — current singleton implementation
- `src/commands/session/mod.rs` — current public command surface
- `src/mother/mod.rs` — Mother seam where live session state should
  attach
- `src/mother/state.rs` — concrete `runtime.db` schema target
- `src/commands/scrape/layer/sessions.rs` — artifact compatibility
  parser
- `layer/core/values/session-capture.md` — low-friction invariant
- `layer/core/values/spec-driven-design.md` — provenance chain invariant

## Open Questions

- Should session artifacts be updated incrementally during a live
  session, or only finalized strongly on end/handoff boundaries?
- What is the minimum participant model that supports humans, interface
  actors, and autonomous agents without overdesign?
- Should handoff links be represented entirely in session frontmatter or
  partly in body sections for readability?
