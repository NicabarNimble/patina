---
type: refactor
id: ducklake-github-lakehouse-ingestion
status: ready
created: 2026-03-12
sessions:
  origin: 20260312-160150
related:
- layer/surface/build/refactor/ducklake-knowledge-child-cutover/SPEC.md
beliefs:
- children-have-agency-toys-are-capabilities
- connector-toy-is-indivisible-authority
- patina-is-combination-of-knowledge-and-action
exit_criteria:
- id: legacy-ducklake-runtime-identity-is-removed
  text: Native legacy DuckLake runtime identity and dual-route execution paths are removed; `Destination::Lake` is knowledge-child only
  checked: false
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

The current DuckLake cutover has improved runtime authority boundaries, but it
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

## Implementation Plan

1. Remove legacy runtime identity and dual-route logic.
2. Implement explicit contract-driven endpoint planner.
3. Implement two-phase ingestion with bounded fanout and adaptive backoff.
4. Implement idempotent upsert materialization keyed by stable identifiers.
5. Write bronze/silver/gold outputs as encrypted parquet partitions.
6. Maintain metadata/checkpoints/manifests in DuckDB.
7. Add reconciliation, late-arrival, dead-letter, and parity verification suite.
8. Emit measure telemetry sufficient for operator-level run diagnostics.

## Non-Goals

- Generic multi-provider connector abstraction in this spec.
- UI vocabulary or naming cleanup.
- Broad Mother federation rollout beyond ingestion correctness and telemetry.

## Verification

The spec is only marked complete when:

- `patina spec check ducklake-github-lakehouse-ingestion --json` passes
- parity and quality test suites demonstrate no regression versus expected scope
- telemetry captures run-level and endpoint-level operational signals
