---
type: feat
id: voice-lake-mvp1
status: draft
created: 2026-04-02
revised: 2026-04-09
sessions:
  origin: 20260402-135124-249836000
  revised: 20260409-070410-485377000
blocked_by: []
beliefs:
  - "[[projects-are-sovereign-mother-coordinates]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[events-are-local-beliefs-federate]]"
  - "[[beliefs-live-at-two-levels]]"
related:
  - refactor/voice-rename
  - refactor/child-typed-composition
  - feat/pando-execution-mvp
  - feat/multiproject-belief-share
  - feat/child-construction-canon
exit_criteria:
  - id: vl1-voice-scoped-query
    text: "Mother federation supports voice-scoped queries: `patina mother federation query --voice <id>` filters results to one voice's namespace."
    checked: false
  - id: vl2-one-voice-one-source
    text: "One voice lane and one local folder source are wired end-to-end through the 6 canon children (file-system-monitor → content-extractor → schema-enforcer → dedup-filter → record-writer → lakehouse-catalog)."
    checked: false
  - id: vl3-pando-config-injection
    text: "Pando manifest supports a `[config]` section with key-value pairs (voice_id, source_id, output_root) that Mother passes to children at instantiation."
    checked: false
  - id: vl4-namespace-discipline
    text: "Persisted output follows `voice/<voice_id>/<source_id>/` path prefix. record-writer and lakehouse-catalog respect the output_root from pando config."
    checked: false
  - id: vl5-reuse-proof
    text: "The same pando (folder-text-to-parquet) runs for a second voice by changing only `[config].voice_id` — no child code changes."
    checked: false
  - id: vl6-federation-attach
    text: "Mother attaches voice lake output (parquet/catalog) to federation.duckdb so it's queryable via `patina mother federation query`."
    checked: false
  - id: vl7-proof
    text: "Proof commands pass: `cargo check --workspace -q`, `cargo nextest run`, and one end-to-end smoke script that ingests a folder, queries the lake, and verifies row count."
    checked: false
---

# feat: Voice Lake MVP1

## Problem

Mother's DuckDB/DuckLake federation substrate is operational (v0.49.0) and all 6
canon children are typed WIT components (Apr 8). But there's no vertical slice
proving they work together under a voice identity — the pipeline can ingest files
but has no concept of WHO the data belongs to or WHERE it should be namespaced.

We need a thin, keepable slice that connects: voice identity → child pipeline →
namespaced output → queryable lake. This is the foundation for
multiproject-belief-share (MVP2).

## Goal

Deliver one minimal voice-lake slice that proves:

1. Mother owns lake/query substrate and can scope queries by voice,
2. the 6 canon children compose end-to-end via typed WIT under a pando,
3. child pipelines are reusable across voices by configuration only.

## Non-Goals

- Cross-mother federation or P2P sync (that's multiproject-belief-share).
- SaaS source connectors (local folder only for MVP).
- Voice keypair/crypto signing (that's belief-system-hardening).
- Parallel child execution (Mother is sequential today; parallelism is a separate spec).
- Changing the record-envelope WIT type — voice/source scoping lives in namespace/routing, not in the record.
- DuckLake compaction, maintenance, or schema evolution.

## Scope

### In scope

- One voice: `default` (the existing voice, formerly persona).
- One source: local folder (reuses file-system-monitor as-is).
- End-to-end typed pipeline via existing pando `folder-text-to-parquet`:
  `file-system-monitor → content-extractor → schema-enforcer → dedup-filter → record-writer → lakehouse-catalog`
- Pando config injection: `[config]` section in pando.toml with voice_id, source_id, output_root.
- Mother passes config to children at composition time (runtime injection).
- Namespaced output: `voice/<voice_id>/<source_id>/` path prefix for parquet and catalog.
- Voice-scoped federation query: `--voice <id>` flag filters to one voice's tables.

### Out of scope

- New children. All 6 canon children are reused without modification.
- New WIT types. record-envelope stays as-is.
- Source-specific logic gated by child code (config-driven only).

## Architecture

### Voice scoping without changing record-envelope

The record-envelope WIT type describes file content (source-path, content-hash, etc.).
Voice identity is orthogonal — it's about WHO owns the pipeline run, not WHAT's in the records.

Voice scoping is achieved through:
1. **Pando config** — `[config].voice_id` and `[config].source_id` set at invocation
2. **Output path prefix** — Mother constructs `output_root = voice/{voice_id}/{source_id}/`
   and injects it as a filesystem preopen for record-writer and lakehouse-catalog
3. **Federation namespace** — Mother registers the voice's output tables under a
   `v_{voice_id}` alias in federation.duckdb

This means:
- Children don't know about voices — they write to their preopen directory
- Mother owns the namespace — it decides where output lands
- Federation can filter by voice — the alias is the scoping mechanism

### Pando config injection

New `[config]` section in pando.toml:

```toml
[pando]
name = "folder-text-to-parquet"

[config]
voice_id = "default"
source_id = "local-docs"
output_root = "voice/default/local-docs"

[[children]]
name = "file-system-monitor"
# ... existing children
```

Mother reads `[config]` at composition time and:
- Sets filesystem preopens using output_root
- Passes config values as environment or init parameters to children that need them
- Uses voice_id for federation table aliasing

### Federation voice scoping

Current: `ATTACH patina.db AS p_{project_uid}`
New: Additionally register voice output as queryable tables:
- `patina mother federation query --voice default "SELECT * FROM records LIMIT 10"`
- Filters to tables under `v_default` alias

## Contracts

### Record envelope (unchanged)

The existing `record-envelope` WIT type is the contract. No new fields.
Voice/source identity is ambient context from pando config, not record content.

### Namespace

Persisted outputs follow:

- `voice/<voice_id>/<source_id>/records/` — parquet files
- `voice/<voice_id>/<source_id>/catalog/` — catalog entries

This namespace is canonical for MVP1 and must be idempotent across reruns.

## Implementation Order

1. Pando config section — parser + validation in `mother/src/pando.rs`
2. Config-driven filesystem preopens — Mother injects output_root at composition
3. Namespace output paths — record-writer and lakehouse-catalog respect preopen root
4. Federation voice alias — ATTACH voice output in federation.duckdb
5. CLI flag — `--voice <id>` on federation query command
6. End-to-end smoke test

## Resolved Decisions

- **record-envelope unchanged** — voice scoping is a routing/namespace concern, not a record schema concern. This avoids breaking all 6 children's WIT.
- **No new children** — this is a composition and infrastructure spec, not a child-building spec.
- **Parallel execution descoped** — Mother is sequential. Parallelism is valuable but orthogonal to voice-lake proof.
- **Local folder only** — SaaS sources are connector children that don't exist yet. file-system-monitor is the proven source.
- **voice-rename is a blocker** — vocabulary must be consistent before building voice infrastructure.

## Verification

```bash
cargo check --workspace -q
cargo nextest run
# End-to-end smoke:
patina mother start
ls ~/.patina/mother/voice/default/           # Voice identity dir exists
# Ingest a test folder through folder-text-to-parquet pando
ls voice/default/local-docs/records/         # Namespaced output exists
patina mother federation query --voice default "SELECT count(*) FROM records"
# Verify row count matches input files
```

Note: There is no `patina voice` CLI command yet. Voice identity is managed
through Mother protocol and paths. A `patina voice` CLI surface may come later
but is not in scope for this spec.

## Build Readiness

Blocked by `feat/pando-execution-mvp`. The pipeline must actually run end-to-end
before we can layer voice scoping on top. voice-rename is complete (Apr 9).
Once pando-execution-mvp lands, the work here is: pando config injection,
namespace paths, and federation voice aliasing.

## Revision Log

- 2026-04-02: Created as persona-lake-mvp1 with 8 exit criteria.
- 2026-04-09: Revised to voice-lake-mvp1. Aligned with typed composition reality (Apr 8), federation v0.49.0, voice-rename spec. Removed source-fetcher/normalizer (don't exist), added actual 6 canon children. Changed record-envelope contract (voice scoping is namespace, not record field). Descoped parallel execution. Reduced to 7 focused exit criteria. Added pando config injection as core mechanism.
- 2026-04-09: Changed blocker from voice-rename (complete) to pando-execution-mvp. Reality check found composition validation works but execution doesn't — pipeline has never run end-to-end. Must close execution gap before layering voice scoping.
