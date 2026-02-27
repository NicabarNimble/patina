---
type: fix
id: data-db-split-fixes
status: active
created: 2026-02-26
sessions:
  origin: 20260226-152857
related:
- data-db-split
- data-architecture-v2
beliefs:
- if-its-patina-its-git
- events-are-autobiography-not-telemetry
exit_criteria:
- id: jsonl-replica-exists
  text: '`layer/events.jsonl` is produced by export and contains all events from events.db'
  checked: true
- id: export-runs-on-session-end
  text: new events are appended to `layer/events.jsonl` on session end
  checked: true
- id: import-rebuilds-events-db
  text: '`patina events import` rebuilds events.db from JSONL — count matches'
  checked: true
- id: doctor-reports-replica-staleness
  text: '`patina doctor` compares max seq in events.db vs JSONL and reports gap'
  checked: true
- id: write-failures-are-loud
  text: failed writes to events.db produce visible warnings via `emit_or_warn()` helper
  checked: true
- id: doctor-checks-events-db-integrity
  text: '`patina doctor` checks events.db existence and basic integrity (`PRAGMA quick_check`)'
  checked: true
- id: broken-fk-removed
  text: forge materialized view FK declarations removed or annotated
  checked: true
- id: dead-syntax-cleaned
  text: bare block in eval.rs removed
  checked: true
- id: migration-is-idempotent
  text: '`ensure_events_db()` migration uses INSERT OR IGNORE — safe under concurrent execution'
  checked: true
- id: init-once-per-process
  text: '`ensure_events_db()` runs once per process via `OnceLock`, not on every call'
  checked: true
---
# fix: Data DB Split Fixes — Durability, Safety, and Cleanup

> data-db-split shipped the event store without its durability half. events.db
> is irreplaceable data with a single point of failure (one file, one disk).
> Write failures are silently swallowed. Postmortem of data-db-split, session
> 20260226-152857.

## Problem

data-db-split (v0.33.1) correctly separated runtime events from rebuildable
projections. But it shipped with gaps across durability, safety, and
resource management that undermine the "irreplaceable" promise:

1. **No durability beyond crash safety.** (Schickling) events.db has WAL +
   synchronous=FULL (survives process crashes) but nothing survives `rm`, disk
   failure, or a fresh clone. The Schickling pattern is SQLite-as-runtime +
   sync-log-as-replica — we built the runtime half and deferred the replica half.

2. **Silent degradation.** (Gjengset) Write failures to events.db are swallowed
   with `let _ =` across 7+ call sites. If events.db corrupts, the system keeps
   running but stops recording. The user has no signal that their autobiography
   stopped being written. `let _ =` is for intentionally discarding values you've
   thought about — using it to swallow write failures to your source of truth is
   negligence, not intentional discard.

3. **TOCTOU race in initialization.** (Gjengset) `ensure_events_db()` checks
   `events_path.exists()` then creates the database. Two concurrent processes
   (CLI + MCP server) could both pass the existence check. Schema creation is
   idempotent (`CREATE TABLE IF NOT EXISTS`) but the migration isn't — both
   could copy the same 96 events, producing duplicates.

4. **Per-call overhead in hot path.** (Gjengset) `open_events_db()` calls
   `ensure_events_db()` on every invocation — a filesystem `exists()` syscall
   per write. For CLI (one emit per command): negligible. For MCP server
   (dozens of requests per session): wasteful. Should initialize once per
   process.

5. **Connection-per-emit.** (Gjengset) `measure::emit()` opens events.db,
   sets PRAGMA synchronous=FULL, inserts one row, drops the connection. Every
   single call. Fine for CLI's one-shot pattern but won't scale as Area 2
   adds more emitters. MCP server path is particularly wasteful.

6. **Schema lies.** forge_issues and forge_prs declare `FOREIGN KEY (event_seq)
   REFERENCES eventlog(seq)` but event_seq now points to events.db seqs, not
   patina.db's eventlog. Dead syntax in eval.rs from the refactor.

## Root Cause

Area 1 was scoped as "separate the databases." The durability story (DESIGN.md
OQ#3) was designed but treated as a separate concern instead of part of the
event store's foundation. The silent-write pattern was inherited from the
pre-split code where events were disposable (they lived in patina.db and got
rebuilt). After the split, events.db is irreplaceable — the error handling
should have changed with the lifecycle rules. The resource management patterns
(per-call initialization, connection-per-emit) were carried over from the
single-database CLI pattern without rethinking for the dual-database +
long-running MCP server context.

## Fix

### Step 1: JSONL Replica — `layer/events.jsonl`

The durability half of the Schickling pattern.

**Export (`patina events export`):**
- Read all events from events.db where `seq > last_exported_seq`, ordered by seq
- Append new events to `layer/events.jsonl`
- Each line is one event: `{"seq":N,"event_type":"...","timestamp":"...","source_id":"...","source_file":"...","data":{...}}`
- Track last exported seq via `scrape_meta` table (key: `last_exported_seq`)

**Atomicity: at-least-once, JSONL-first.** The ordering matters:
1. Begin read transaction on events.db (snapshot isolation)
2. SELECT events where `seq > last_exported_seq` ORDER BY seq
3. Append to JSONL file, fsync
4. UPDATE `scrape_meta` SET value = max_seq (commit)

If crash between step 3 and 4: marker wasn't updated, next export re-appends
the same events, JSONL has duplicates, import deduplicates on seq. This is
safe — at-least-once beats at-most-once for durability. Never update the
marker before JSONL hits disk.

**Auto-export on session end:**
- `patina session end` calls export after archiving
- Best-effort: if export fails, warn and continue. Session archival is the
  critical path, export is not. Doctor catches staleness on next run.
- New events since last export get appended and committed with the session

**Import (`patina events import <path>`):**
- Parse JSONL line by line (serde deserialization, typed structs)
- `INSERT OR IGNORE` into events.db using seq as PRIMARY KEY — same seq
  means same event, skip. This works because import is disaster recovery
  on the same project (restoring events.db from its own replica), not
  cross-machine merge.
- Report: imported N events, skipped M duplicates
- Import does NOT update `last_exported_seq` marker — it's a recovery
  operation, not an export. Next export will see the restored events and
  update the marker naturally.

**Scale:** ~3,500 events/year x ~200 bytes = ~700KB/year of JSONL. Git handles
line-oriented text well. After 10 years: ~7MB.

### Step 2: Loud Write Failures + `emit_or_warn()` Helper

Centralize the error-handling policy in a helper instead of fixing each
call site independently. Gjengset principle: if you're going to handle an
error the same way in 7+ places, extract the policy.

**New helper in `src/measure.rs`:**
```rust
/// Emit a measurement event, warning on failure instead of silently dropping.
/// Use this at call sites where the operation should succeed but isn't worth
/// crashing for. The warning makes degradation visible.
pub fn emit_or_warn(verb: &str, tool: &str, mode: &str, metrics: &serde_json::Value) {
    if let Err(e) = emit(verb, tool, mode, metrics) {
        eprintln!("patina: warning: failed to record event: {e}");
    }
}
```

**Update all 7 `let _ = measure::emit(...)` call sites** to use
`measure::emit_or_warn(...)`. The `emit()` function stays as `Result<()>`
for callers that want to propagate (forge already does).

**Scry CLI logging** — same pattern for CLI path. Add eprintln before
returning None on failure.

**Scry MCP logging** — currently uses `.ok()?` (silent). Leave as-is for
now. The MCP server has no warning channel — `eprintln!` during MCP goes
to process stderr which may not be visible. MCP warning infrastructure is
a separate concern, addressed in [[spec-mcp-server-hardening]].

**Forge inserts** — already propagate errors via `?`. No change needed.

The principle: write failures are warnings, not panics. The tool still works.
But the user SEES that events aren't being recorded. Silent degradation becomes
visible degradation. The policy is in one place (`emit_or_warn`), not spread
across 7 call sites. MCP-specific warning surfacing deferred to
[[spec-mcp-server-hardening]].

### Step 3: Fix Initialization — TOCTOU + OnceLock

**Make migration idempotent (fix TOCTOU):**

The migration in `ensure_events_db()` copies runtime events from patina.db.
If two processes race, both could insert the same events. Fix: use
`INSERT OR IGNORE` with a dedup key, or add a unique constraint on
`(event_type, timestamp, source_id)` to the events.db eventlog table.

This also makes `patina events import` (Step 1) naturally safe — the same
dedup key prevents double-import.

**Run initialization once per process (`OnceLock`):**

Replace the per-call `events_path.exists()` check with a process-level
initialization gate:

```rust
use std::sync::OnceLock;

static EVENTS_INIT: OnceLock<()> = OnceLock::new();

pub fn ensure_events_db() -> Result<()> {
    EVENTS_INIT.get_or_try_init(|| {
        // ... existing creation + migration logic
        Ok(())
    })?;
    Ok(())
}
```

One `exists()` check per process lifetime instead of per call. CLI: no visible
difference. MCP server: eliminates syscall-per-request overhead.

### Step 4: Doctor Health Checks

Two new doctor checks:

**events.db integrity:**
- File exists at `.patina/local/data/events.db`
- `PRAGMA quick_check` passes (fast, sufficient for routine checks —
  `integrity_check` is thorough but slow on large databases)
- `SELECT COUNT(*) FROM eventlog` succeeds
- Warning if count is 0 (empty event store)

**JSONL replica staleness:**
- `layer/events.jsonl` exists
- Compare highest seq in JSONL (last line's `seq` field) vs highest seq in
  events.db (`SELECT MAX(seq) FROM eventlog`). Seq comparison is deterministic
  and immune to blank lines, unlike line counting.
- Report gap: "events.db max seq: N, JSONL max seq: M, gap: N-M events"
- Warning if gap > 0, actionable: "run `patina events export`"

### Step 5: Schema Cleanup

**Remove broken FK declarations:**
- `src/commands/scrape/forge/mod.rs` — remove `FOREIGN KEY (event_seq)
  REFERENCES eventlog(seq)` from forge_issues and forge_prs CREATE TABLE.
  Keep the `event_seq INTEGER` column (it's useful as a cross-db reference),
  just drop the unenforceable constraint.

**Remove dead syntax:**
- `src/commands/eval/mod.rs:1553` — remove bare `{ }` block wrapping
  `measure::emit()` call. Leftover from `if let Ok(conn) = ...` refactor.

## Non-Goals

- **New event types.** This spec fixes the event store's durability and safety.
  Wiring new emissions is Area 2's job (data-emission-completeness).
- **JSONL as primary store.** JSONL is a replica, not a replacement for
  events.db. SQLite remains the runtime. JSONL travels with git for
  machine-loss recovery.
- **Automated backup scheduling.** Export runs on session end. No cron,
  no daemon, no background process.
- **Event compaction or rotation.** Per DESIGN.md OQ#2: no compaction, ever.
  The numbers don't justify it.
- **Event type enum.** Gjengset's instinct is right — moving from stringly-typed
  verbs + `VALID_VERBS` to a real enum eliminates a class of runtime bugs, and
  Rust's exhaustive matching makes it obvious when you forget to handle a new
  verb. But Area 2 is actively adding verbs (the registry isn't stable as of
  Feb 2026). Touching the enum every time you add a verb is busywork — no
  compilers are broken today because runtime validation still guards mistakes.
  **Trigger:** convert when the registry cadence slows and every verb ships
  with a known owner and test plan. That's when the enum gives lasting leverage.
- **ATTACH newtype wrapper.** The safety benefit is real — a dedicated
  `MeasureConnection` constructor that always ATTACHes would let the compiler
  enforce "all queries after this point are safe to run against both DBs." But
  only the measure path performs cross-DB reads today. Adding a dedicated type
  introduces overhead (new module, trait plumbing) with zero reuse. **Trigger:**
  when a second cross-DB consumer appears — that's when duplication and risk
  multiply. At that point, introduce the wrapper, migrate measure onto it, and
  on-board the new consumer simultaneously. **Interim:** a plain
  `fn attach_events(conn: &Connection)` helper extracts the ATTACH boilerplate
  without type machinery. Could land in Step 6 cleanup or wait for Area 2.
- **MCP warning channel.** The MCP server has no way to surface warnings to
  clients alongside successful responses. `eprintln!` during MCP may not be
  visible. Addressed in [[spec-mcp-server-hardening]], not here.
- **Connection reuse.** Connection-per-emit is fine for CLI. MCP server
  connection reuse (thread-local, RefCell guards, lifetime management)
  belongs in [[spec-mcp-server-hardening]] where it can be designed alongside
  the broader MCP observability work.
- **Connection pooling.** Full connection pooling (r2d2, deadpool) is
  overengineering for a local-first CLI tool with SQLite.
