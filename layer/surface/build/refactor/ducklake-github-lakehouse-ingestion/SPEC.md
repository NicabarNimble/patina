---
type: refactor
id: ducklake-github-lakehouse-ingestion
status: active
created: 2026-03-12
updated: 2026-03-17
sessions:
  origin: 20260312-160150
related:
- layer/surface/build/refactor/ducklake-knowledge-child-cutover/SPEC.md
- layer/surface/build/refactor/ducklake-github-lakehouse-ingestion/DESIGN.md
beliefs:
- children-have-agency-toys-are-capabilities
- connector-toy-is-indivisible-authority
- patina-is-combination-of-knowledge-and-action
exit_criteria:
- id: legacy-ducklake-runtime-identity-is-removed
  text: DuckLake keeps a knowledge-child-only runtime path baseline and `Destination::Lake` stays knowledge-child only
  checked: true
- id: github-scope-contract-is-explicit-and-complete
  text: Scope contract explicitly defines full issues/PR ingestion surface (issues, issue comments, issue events, pulls, pull comments, reviews, review comments, optional commits) with retention/backfill and incremental key rules
  checked: false
- id: two-phase-ingestion-with-bounded-fanout-is-implemented
  text: Knowledge-child ingestion uses phase A list pagination plus phase B bounded fanout for child entities with adaptive rate-limit backoff
  checked: false
- id: watermarks-and-idempotent-upserts-are-enforced
  text: Watermarks and stable entity keys provide idempotent reruns with no silent duplicates
  checked: false
- id: parquet-layout-and-duckdb-metadata-are-shipped
  text: Lake writes encrypted parquet partitions (org/repo/entity/date) and tracks manifests/checkpoints/runs/schema in DuckDB metadata
  checked: false
- id: quality-controls-cover-reconciliation-and-late-arrivals
  text: Reconciliation counts, late-arrival handling, dead-letter flow, and parity samples against GitHub API totals are implemented and tested
  checked: false
- id: operational-controls-and-telemetry-are-complete
  text: Bounded concurrency, retry-class policy, and rich measure telemetry (duration, calls, bytes, retries, lag) are emitted for ingestion runs
  checked: false
- id: bronze-silver-gold-data-blocks-are-exposed
  text: Bronze raw snapshots, silver normalized entities, and gold analytics/agent data blocks are produced for downstream apps/agents
  checked: false
---
# refactor: DuckLake GitHub Lakehouse Ingestion

> Replace migration-era ingestion behavior with an authoritative knowledge-child
> pipeline that captures full issues/PR data into encrypted parquet-backed lake
> blocks with DuckDB metadata and production-grade telemetry.

## Problem

DuckLake runtime cutover and knowledge-child-only baseline are complete, but ingestion
still behaves as a migration-stage implementation:

- parity with legacy semantics is incomplete for full PR/issue enrichment
- storage remains JSON-in-DuckDB rows rather than parquet lake partitions
- telemetry exists but does not yet provide production-grade ingestion
  observability for throughput, retries, lag, and quality reconciliation

Client requirements now prioritize complete GitHub issue/PR capture into
lakehouse-style outputs that downstream transforms and agent/apps can consume.

## Goal

Ship a full DuckLake knowledge-child ingestion system for GitHub issue/PR data
with deterministic continuity, comprehensive scope coverage, encrypted parquet
outputs, DuckDB metadata management, and explicit operational telemetry.

## Status

Active.

Boundary and naming stabilization lanes are complete. This spec is now the
authoritative ingestion correctness lane and is ready for implementation slices.

## Resolved Decisions (Locked For Build)

1. Pull commits are feature-flagged and off by default.
2. Reconciliation uses bounded tolerance (default: max 2% or 25 records,
   whichever is greater) and logs threshold breaches.
3. Gold exposes physical outputs plus minimal stable query views for downstream
   analytics/agent workloads.
4. Ingestion remains GitHub-specific in this spec; provider abstraction is
   explicitly deferred.
5. Two-phase ingestion is mandatory: list pagination first, bounded fanout for
   child entities second.

## Contract

### Data Scope Contract (authoritative)

Default contract for each repo binding:

1. Issues list (`state=all`)
2. Issue comments
3. Issue events
4. Pull requests list (`state=all`)
5. Pull comments
6. Pull reviews
7. Pull review comments
8. Pull commits (optional feature flag)

Contract also defines:

- retention and backfill windows
- incremental keys and watermark behavior
- per-entity stable identity keys for upsert

### Default Policy Values

- Retention: full-history by default, bounded by configurable backfill window
  per entity type.
- Backfill window: 90 days on first bind for child entities; full listing for
  top-level issues/pulls.
- Watermark key: `updated_at` per repo/entity stream.
- Entity identity key: `repo_id + entity_type + provider_id`.
- Reconciliation threshold: `max(2% of source count, 25 records)`.

### Data Contract Appendix (Determinism + Continuity)

- Cursor identity is tuple-based per repo/entity stream:
  `watermark = (updated_at, provider_id)`.
- Progression is monotonic lexicographic on `(updated_at, provider_id)`.
- Resume queries are time-inclusive on `updated_at`; local filtering drops rows with
  tuples `<=` last committed watermark tuple to prevent duplicate logical writes.
- Tie-break ordering uses stable `provider_id` whenever `updated_at` is equal.
- Replay window policy: every run re-reads a bounded trailing window for
  late-arrival correction (default 24h for child entities), then deduplicates by
  stable identity key plus tuple ordering.
- Tombstone policy:
  - Bronze is append-only and never hard-deletes prior snapshots.
  - Silver records soft-delete state (`is_deleted`, `deleted_at`) when an entity
    is confirmed missing/removed by reconciliation rules.
  - Gold reflects silver soft-delete semantics and excludes tombstoned rows from
    default analytic views unless explicitly requested.

## Implementation Plan

1. Lock explicit contract-driven endpoint planner.
2. Implement two-phase ingestion with bounded fanout and adaptive backoff.
3. Implement idempotent upsert materialization keyed by stable identifiers.
4. Write bronze/silver/gold outputs as encrypted parquet partitions.
5. Maintain metadata/checkpoints/manifests in DuckDB.
6. Add reconciliation, late-arrival, dead-letter, and parity verification suite.
7. Emit measure telemetry sufficient for operator-level run diagnostics.

## Slice Execution Plan

### Slice A - Scope and ingestion planner

- Lock endpoint planner for issues/issues-comments/issues-events/pulls/
  pull-comments/reviews/review-comments with optional commits flag.
- Implement two-phase pipeline scaffolding and cursor progression contract.

### Slice B - Materialization and continuity

- Implement idempotent upserts with stable keys and watermark enforcement.
- Write encrypted parquet partitions and DuckDB metadata/manifests/checkpoints.

### Slice C - Quality and operability

- Implement reconciliation, late-arrival, dead-letter, and parity samples.
- Emit bounded-concurrency, retry-class, and run diagnostics telemetry.
- Expose bronze/silver/gold outputs and minimal gold query views.

## Non-Goals

- Generic multi-provider connector abstraction in this spec.
- UI vocabulary or naming cleanup.
- Broad Mother federation rollout beyond ingestion correctness and telemetry.

## Verification

The spec is only marked complete when:

- `patina spec check ducklake-github-lakehouse-ingestion --json` passes
- parity and quality test suites demonstrate no regression versus expected scope
- telemetry captures run-level and endpoint-level operational signals

### Exit Criteria Gate Map

Each criterion below can only be checked true after the mapped command(s) exist
and pass.

1. `github-scope-contract-is-explicit-and-complete`
   - `cargo test -q -p patina-ai ducklake_scope_contract_covers_required_entities`
2. `two-phase-ingestion-with-bounded-fanout-is-implemented`
   - `cargo test -q -p patina-ai ducklake_two_phase_ingestion_with_bounded_fanout`
3. `watermarks-and-idempotent-upserts-are-enforced`
   - `cargo test -q -p patina-ai ducklake_idempotent_upsert_and_watermark_progression`
4. `parquet-layout-and-duckdb-metadata-are-shipped`
   - `cargo test -q -p patina-ai ducklake_parquet_partition_layout_and_metadata_manifest`
5. `quality-controls-cover-reconciliation-and-late-arrivals`
   - `cargo test -q -p patina-ai ducklake_reconciliation_late_arrival_and_dead_letter_flow`
6. `operational-controls-and-telemetry-are-complete`
   - `cargo test -q -p patina-ai ducklake_ingestion_operational_telemetry_emitted`
7. `bronze-silver-gold-data-blocks-are-exposed`
   - `cargo test -q -p patina-ai ducklake_bronze_silver_gold_outputs_available`

Cross-cutting determinism gate (required before marking spec complete):

- `cargo test -q -p patina-ai ducklake_github_ingestion_fixture_replay_is_deterministic`

## Build Readiness

- [x] Status and doctrine alignment are current.
- [x] Design doc exists and is linked.
- [x] Data scope contract is explicit.
- [x] Core defaults (watermark/key/retention/tolerance) are locked.
- [x] Slice order is defined.
- [x] Exit criteria have command-level gate mapping.
