# Design: claude code session child system

## Why This Design

This design separates raw capture from knowledge shaping:

- Raw logs are telemetry (complete but noisy).
- Session blocks are curated memory (queryable and reusable).

Using two children preserves Patina boundaries:

- Child A handles source-specific ingestion mechanics.
- Child B handles semantic shaping and redaction.
- Mother owns orchestration, retries, and grants.

That keeps extraction logic composable and avoids coupling ingestion mechanics to summarization policy.

## Build Target

1. Add `claude-log-ingest` child to ingest JSONL events into append-only lake rows.
2. Add `session-block-transform` child to generate normalized session blocks with lineage.
3. Add Mother orchestration path to run ingest -> transform deterministically.
4. Ensure output blocks are consumable by retrieval/session workflows.

## Resolved Decisions

- Keep raw event rows immutable.
- Perform secret redaction in transform before block emission.
- Use deterministic dedupe and checkpointing to support safe reruns.
- Keep schema interface-neutral even when source parser is Claude-specific.
- Treat generated blocks as derived data, not replacement of ground-truth raw logs.

## Child Roles and Toy Grants

### `claude-log-ingest`

Expected toys:

- `log`
- `state`
- `checkpoint`
- `lake`
- `connector` (file-backed source adapter)

Responsibilities:

- Discover source files under configured Claude project path(s).
- Parse JSONL into normalized raw event rows.
- Write append-only rows to `session_raw_events`.
- Persist per-source checkpoint.

### `session-block-transform`

Expected toys:

- `log`
- `state`
- `checkpoint`
- `lake`
- `measure`

Responsibilities:

- Read unprocessed raw events.
- Build deterministic blocks from event windows.
- Apply redaction policy.
- Write blocks to `session_blocks` with `source_event_refs`.

## Dataflow

1. Source adapter reads `.jsonl` events from Claude project logs.
2. Ingest child normalizes rows and appends to raw table.
3. Mother marks ingest task success and triggers transform task.
4. Transform child emits block rows + metrics.
5. Retrieval/session tooling queries blocks for high-signal context.

## Failure and Retry Semantics

- Ingest failures are recoverable from checkpoint.
- Transform failures are recoverable from last processed event watermark.
- Duplicate writes are prevented by stable dedupe keys.
- Dead-letter behavior follows existing Mother task retry policy.

## Security and Privacy

- Source files are read-only inputs.
- Raw rows keep full payload for provenance.
- Derived blocks must redact token-like values and known secret fields.
- Redaction should set `redaction_flags` for auditability.

## Commits

1. `feat(spec): add claude-code-session-child-system spec and design`
2. `feat(children): add claude-log-ingest child skeleton and schemas`
3. `feat(children): add session-block-transform child skeleton`
4. `feat(children): implement checkpointed append-only ingest`
5. `feat(children): implement block extraction + redaction + lineage`
6. `feat(mother): wire ingest-transform orchestration path`
7. `test(e2e): add fixture pipeline test for claude logs to blocks`

## Direct Code Targets

- `children/claude-log-ingest/`
- `children/session-block-transform/`
- `mother/src/broker/sources.rs`
- `mother/src/state.rs`
- `src/mother/broker/mod.rs`
- `src/child/toy_host/lake.rs`
- `sdk/patina-sdk/` (if new helper trait/macros are needed)

## Verification Plan

Core gates:

```bash
cargo check --workspace -q
cargo test -q --workspace
```

Behavior checks:

```bash
patina spec check claude-code-session-child-system --json
patina mother run claude-log-ingest --json
patina mother run session-block-transform --json
```

Assertions:

- Raw table grows append-only with no duplicates on rerun.
- Block table rows include source lineage.
- Redaction test fixture proves no leaked token values in block text.

## Build Readiness

Ready for phased implementation. Existing Mother task/checkpoint primitives are sufficient for phase 1.
