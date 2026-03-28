---
type: feat
id: claude-code-session-child-system
status: draft
created: 2026-03-27
sessions:
  origin: 20260327-021039-379187000
beliefs:
  - "[[patina-is-knowledge-layer]]"
  - "[[dont-build-what-exists]]"
  - "[[eventlog-is-truth]]"
related:
  - mother/src/broker/sources.rs
  - mother/src/state.rs
  - src/mother/broker/mod.rs
  - src/child/toy_host/lake.rs
  - sdk/patina-sdk/
  - children/
  - layer/sessions/
  - layer/surface/epistemic/
exit_criteria:
  - id: ccss1-append-only-ingest
    text: "A child can ingest Claude Code JSONL logs into an append-only lake table with deterministic dedupe and checkpoint resume."
    checked: false
  - id: ccss2-provenance-complete
    text: "Every ingested row includes source provenance (interface, source file, session id, byte offset or line index, content hash, ingested_at)."
    checked: false
  - id: ccss3-transform-blocks
    text: "A second child transforms raw log rows into normalized session blocks (goal, decisions, evidence, outcomes, open items) with trace-back to source rows."
    checked: false
  - id: ccss4-no-secret-leakage
    text: "Transform pipeline applies explicit redaction policy for secrets/tokens before block emit and is covered by tests."
    checked: false
  - id: ccss5-idempotent-reruns
    text: "Re-running ingest and transform is idempotent: no duplicate lake records or duplicate blocks for unchanged input."
    checked: false
  - id: ccss6-cross-interface-ready
    text: "Schema supports multiple interfaces (claude/opencode/gemini) even if phase 1 only enables Claude source adapter."
    checked: false
  - id: ccss7-queryable-output
    text: "Generated blocks are queryable through existing Patina retrieval surfaces and can be linked into session artifacts without manual copy/paste."
    checked: false
  - id: ccss8-gates-green
    text: "Workspace checks and tests pass, including ingest/transform unit tests and one end-to-end fixture test."
    checked: false
---
# feat: claude code session child system

## Problem

Claude Code already stores dense, high-fidelity conversation logs in JSONL form under `~/.claude/projects/...`, including user prompts, assistant output, tool calls, and progress events. Those logs are useful as raw telemetry, but they are not project-memory artifacts: they are noisy, vendor-scoped, difficult to query semantically, and not directly aligned to Patina's session/belief workflow.

Today, high-value session context is manually summarized into `layer/sessions/*.md`. That keeps quality high but does not scale as interaction volume grows.

## Goal

Build a two-child system that treats CLI logs as ingest telemetry and emits Patina-native, provenance-preserving session data blocks that can feed retrieval and session artifacts.

In short:

1. Child A ingests append-only logs into lake storage.
2. Child B transforms raw rows into normalized, queryable session blocks.
3. Mother mediates checkpoints, retries, and capability boundaries.

## Non-Goals

- Replacing all existing session markdown artifacts in phase 1.
- Making Claude internals a hard dependency for Patina core.
- Solving all interfaces at once; phase 1 starts with Claude source adapter but keeps multi-interface schema.
- Building a perfect universal summarizer; phase 1 prioritizes deterministic extraction and provenance over prose quality.

## Target Shape

### Child A: `claude-log-ingest` (connect/store child)

- Reads JSONL files from configured Claude project paths.
- Appends normalized raw events into a lake append-only lane (Parquet-compatible downstream/export target).
- Uses checkpoint toy for resume per source file/session.
- Uses deterministic dedupe key `(source_path, offset_or_line, hash)`.

### Child B: `session-block-transform` (transform/store child)

- Reads raw event rows from lake.
- Extracts structured blocks:
  - session metadata
  - goals
  - decisions
  - evidence snippets
  - outcomes/handoffs/open items
- Emits block rows with source lineage (list of contributing raw row ids).
- Applies secret redaction before emit.

### Mother orchestration

- Schedules ingest child first, transform child second.
- Tracks task state/retries via existing task/state tables.
- Maintains bounded capability grants via `[needs].toys`.

## Data Model (phase 1)

### Raw lake table (append-only)

`session_raw_events`

- `interface` (`claude`)
- `project_key`
- `session_id`
- `source_path`
- `event_index`
- `event_type`
- `role`
- `timestamp`
- `payload_json`
- `payload_hash`
- `ingested_at`
- `dedupe_key`

### Block table

`session_blocks`

- `block_id`
- `interface`
- `project_key`
- `session_id`
- `block_kind` (`goal|decision|evidence|outcome|handoff|context`)
- `title`
- `body`
- `confidence`
- `source_event_refs` (array/json)
- `redaction_flags`
- `created_at`

## Solution

### Phase A - schema + source adapter lock

- Define raw and block schemas.
- Implement Claude JSONL adapter parser with strict validation + tolerant fallback.
- Define provenance and dedupe contract.

### Phase B - ingest child

- Build `claude-log-ingest` child with toys: `log`, `state`, `checkpoint`, `lake`, `connector` (file source adapter).
- Add checkpointed append and idempotent rerun behavior.
- Add fixture tests with representative Claude session logs.

### Phase C - transform child

- Build `session-block-transform` child with toys: `log`, `state`, `checkpoint`, `lake`, `measure`.
- Add deterministic extraction rules for phase 1 block kinds.
- Add redaction pass and lineage attachment.

### Phase D - Patina integration

- Wire Mother task orchestration for ingest -> transform order.
- Expose minimal query surface to inspect blocks.
- Add optional session-artifact helper that suggests block-backed summary material.

## Guardrails

1. Append-only truth: raw event table is immutable; corrections happen in derived layers.
2. Provenance never dropped: every block must reference source raw events.
3. No secret amplification: transform must redact before emitting curated block text.
4. Idempotent rerun requirement: same inputs, same outputs, no duplication.
5. Interface neutrality by schema: Claude-specific parser, interface-neutral stored shape.

## Verification

```bash
cargo check --workspace -q
cargo test -q --workspace
patina spec check claude-code-session-child-system --json
```

Behavior checks (post-implementation):

```bash
# Run ingest against fixture source
patina mother run claude-log-ingest --json

# Run transform
patina mother run session-block-transform --json

# Validate block output exists with lineage
<query command for session_blocks includes source_event_refs>
```

## Build Readiness

Ready for implementation slicing. The architecture aligns with existing Mother task/checkpoint/lake patterns and current SDK toy model.
