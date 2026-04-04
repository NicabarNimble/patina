---
type: feat
id: persona-lake-mvp1
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-135124-249836000
blocked_by:
  - mother-duckdb-ducklake-federation
beliefs:
  - "[[projects-are-sovereign-mother-coordinates]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[five-boundaries-no-overlap]]"
  - "[[core-verbs-standalone-mother-additive]]"
related:
  - mother/src/
  - src/mother/
  - children/
  - layer/surface/build/feat/mother-duckdb-ducklake-federation/SPEC.md
  - layer/surface/build/feat/child-construction-canon/SPEC.md
exit_criteria:
  - id: pl1-mother-surface
    text: "Mother provides minimal federation surface for this slice: status + bounded read query over one persona lake namespace."
    checked: false
  - id: pl2-one-persona-one-source
    text: "One persona lane (`persona-ops`) and one source type are wired end-to-end (choose exactly one SaaS source OR local folder source for MVP)."
    checked: false
  - id: pl3-keepable-children
    text: "Children are split into keepable roles: source-fetcher (ingress), normalizer (shape), and canon validators/writers (`schema-enforcer`, `dedup-filter`, `record-writer`)."
    checked: false
  - id: pl4-stable-contract
    text: "A stable record envelope is enforced across children (required provenance keys include persona_id, source_id, and ingestion timestamp)."
    checked: false
  - id: pl5-namespace-discipline
    text: "Lake/table paths follow a deterministic namespace (`persona/<persona_id>/<source>/<table>`) and can be re-run idempotently."
    checked: false
  - id: pl6-reuse-proof
    text: "The same child pipeline runs for a second persona by configuration only (no child code changes), proving reuse."
    checked: false
  - id: pl7-parallel-load-proof
    text: "Parallel child execution (at least 5 concurrent tasks) completes without duplicate drift after dedup and without capability-boundary violations."
    checked: false
  - id: pl8-proof
    text: "Proof commands pass: `patina spec check persona-lake-mvp1 --json`, `cargo check --workspace -q`, and one end-to-end persona-lake smoke run script."
    checked: false
---

# feat: Persona Lake MVP1

## Problem

We need a practical way to validate Mother-owned DuckDB/DuckLake substrate with
canon child composition without expanding scope into full multiproject or
cross-mother federation. Today, architecture is directionally aligned but we
need a thin, keepable vertical slice.

## Goal

Deliver one minimal persona-lake slice that proves:

1. Mother owns lake/query substrate,
2. children provide scalable compute roles,
3. child components are reusable across personas by configuration.

## Non-Goals

- Cross-mother shared-lake federation.
- Full SaaS connector matrix in MVP1.
- Full DuckLake maintenance suite (all compaction/cleanup modes).
- Runtime kind/engine consolidation work.

## Scope

### In scope

- One persona lane: `persona-ops`.
- One source lane (pick one only):
  - SaaS source (e.g. Gmail OR Dropbox), or
  - local folder source.
- End-to-end path:
  - `source-fetcher` -> `normalizer` -> `schema-enforcer` -> `dedup-filter` -> `record-writer`.
- Mother status/query interface for this lane.

### Out of scope

- New abstraction families for this slice.
- Source-specific one-off logic in shared children unless gated by config.

## Contracts

### Child envelope (minimum required fields)

- `persona_id`
- `source_id`
- `ingested_at`
- `record_type`
- `payload`

Exact typed schema can evolve, but these keys are mandatory for MVP1
interoperability and lineage.

### Namespace

Persisted outputs follow:

- `persona/<persona_id>/<source>/<table>`

This namespace is canonical for MVP1 and should remain stable across reruns.

## Verification

```bash
patina spec check persona-lake-mvp1 --json
cargo check --workspace -q
# project-local smoke script path to be defined in implementation PR
```

## Build Readiness

Ready once `mother-duckdb-ducklake-federation` lands. This spec is intentionally
small and reusable-first to avoid overbuilding while producing durable child
components.
