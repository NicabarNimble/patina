---
type: feat
id: session-hardening
status: draft
created: 2026-02-20
sessions:
  origin: null
related:
- src/commands/session/internal.rs
- src/commands/session/mod.rs
- resources/claude/session-start.md
- resources/claude/session-end.md
- src/eventlog.rs
beliefs:
- session-git-integration
- stale-context-is-hostile-context
- process-checkpoints-over-tooling
---

# feat: Session System Hardening

> The session system works well. Don't redesign it — harden the edges.
> Fix stale session accumulation, partial-end bugs, and give the eventlog
> its first consumer.

## Problem

Sessions accumulate in `active` status when the user walks away without
`/session-end`. Two stale sessions discovered on 2026-02-20: one from Dec 9
2025 (actually completed but status never flipped), one from Jan 31 2026
(started, never used). The only cleanup path is `session start` archiving
the previous session — but that only fires when you start a *new* one.

Secondary: the eventlog dual-write (`session.started`, `session.update`,
`session.observation`, `session.ended`) has no reader. It's infrastructure
without consumers.

## What NOT to Change

- **Skill-as-prompt pattern** — markdown instructions telling Claude how to
  behave are the core value. Don't replace with pure CLI.
- **Dual-write architecture** — markdown for LLM collaboration, eventlog for
  structured queries. Both serve distinct purposes.
- **Git tag bracketing** — `session-{id}-{adapter}-start/end` is simple and
  enables replay.
- **Work classification heuristics** — imperfect but useful. Refine later.

## Phases

### Phase 1: Visibility & Bug Fixes (low risk, additive only)

**1a. `patina session list` command**

New subcommand showing active, stale (>24h), and recent completed sessions.
Query `layer/sessions/` + `.patina/local/active-session.md`. No changes to
existing commands.

```
$ patina session list
ACTIVE  20260131-093100  Session System & Adapter Parity (21d stale)
RECENT  20260218-225007  Secrets Keychain Policy (completed, 2d ago)
RECENT  20260218-192625  Setup Claude UX (completed, 2d ago)
```

**1b. Stale session warning in `session start`**

Currently: silently archives previous session. Proposed: if previous session
is >24h old, print a warning showing what's being archived. Nothing lost
silently.

**1c. Atomic status flip in `session end`**

The Dec 9 bug: session had classification and end tags but `status: active`
persisted. Fix: flip `status: active → completed` as the *first* mutation in
`end_session()`, before computing metrics or archiving. If later steps fail,
the session is at least marked done.

**Exit criteria:**
- [ ] `patina session list` shows active/stale/recent sessions
- [ ] `session start` warns when archiving a session >24h old
- [ ] `session end` flips status before archiving (atomic-first)
- [ ] Both stale sessions from 2026-02-20 cleaned up

### Phase 2: Strengthen the Contract (medium risk)

**2a. Richer CLI output for skills**

Currently: skill markdown tells Claude to run CLI, then separately read the
output file. Proposed: CLI returns a structured summary to stdout that the
skill can use directly. Fewer LLM interpretation steps, less chance of
skipping. Skill markdown stays the same — just relies less on file reads.

**2b. Eventlog consumer in `patina-review`**

Give the eventlog its first reader. `/patina-review` queries structured
events to surface: session frequency, work type distribution, belief capture
rate, average session duration. Makes the dual-write investment pay off.

**2c. Auto git metric capture (opt-in hook)**

Pre-commit hook that appends basic git context to active session file.
Manual `/session-update` stays for narrative — hook captures facts
automatically. Opt-in via `patina session hooks install`, not default.

**Exit criteria:**
- [ ] Session CLI commands return structured summary to stdout
- [ ] `patina-review` queries eventlog for session analytics
- [ ] Optional pre-commit hook captures git metrics to active session

### Phase 3: Future Consideration (needs own spec if pursued)

**3a. Session state machine**

Formal states: `created → active → paused → ended`. Transitions validated
in Rust. Would prevent stale sessions structurally but changes the
fundamental model (file-as-state → state-as-data). Park this — the Phase 1
fixes may make it unnecessary.

## Open Questions

1. Should `session list` also query archived sessions in `layer/sessions/`,
   or only active + `.patina/local/`?
2. Should `patina doctor` check for stale sessions as part of health checks?
3. Is pre-commit hook the right trigger for auto-capture, or a background
   file watcher?

## Key Files

```
src/commands/session/mod.rs       — public API, clap subcommands
src/commands/session/internal.rs  — all lifecycle logic
resources/claude/session-*.md     — skill instructions
src/adapters/templates.rs         — template embedding
src/eventlog.rs                   — event infrastructure
src/git/operations.rs             — git tag/branch helpers
```
