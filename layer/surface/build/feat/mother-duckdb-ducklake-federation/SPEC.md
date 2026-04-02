---
type: feat
id: mother-duckdb-ducklake-federation
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-135124-249836000
blocked_by:
  - ducklake-retirement
beliefs:
  - "[[projects-are-sovereign-mother-coordinates]]"
  - "[[standards-are-storage-coordination-sits-above]]"
  - "[[core-verbs-standalone-mother-additive]]"
  - "[[five-boundaries-no-overlap]]"
related:
  - mother/src/
  - src/mother/
  - src/paths.rs
  - layer/surface/build/feat/multiproject-belief-share/SPEC.md
  - layer/surface/build/feat/child-construction-canon/SPEC.md
exit_criteria:
  - id: mdf1-mother-federation-home
    text: "Mother owns one federation catalog at ~/.patina/mother/federation.duckdb and manages lifecycle (open, lock, refresh, close)."
    checked: false
  - id: mdf2-project-sqlite-sovereignty
    text: "Per-project SQLite remains source of truth; federation is additive query coordination only. No core writes are redirected to DuckDB."
    checked: false
  - id: mdf3-attach-registry
    text: "Mother builds an attach registry for known projects and exposes deterministic project_uid -> attached alias mapping for federation queries."
    checked: false
  - id: mdf4-ducklake-extension-policy
    text: "DuckLake extension loading is host-managed and optional: if available, Mother enables it; if unavailable, Mother keeps non-DuckLake federation paths working with explicit diagnostics."
    checked: false
  - id: mdf5-federation-query-surface
    text: "Mother exposes a read-focused federation query surface for cross-project use (status, refresh, query) with safe parameterization and bounded result size defaults."
    checked: false
  - id: mdf6-multiproject-unblock
    text: "The multiproject-belief-share spec is updated to depend on this federation substrate before building query-responder and cross-project lake reads."
    checked: false
  - id: mdf7-observation-contract
    text: "Mother emits federation telemetry (refresh duration, attach count, query latency, failures) via existing observation conventions."
    checked: false
  - id: mdf8-failure-boundaries
    text: "Failures in federation (attach/load/query) degrade gracefully and do not break standalone local protocol verbs for a single project."
    checked: false
  - id: mdf9-proof
    text: "Proof commands pass: `patina spec check mother-duckdb-ducklake-federation --json`, `cargo check --workspace -q`, and targeted federation tests."
    checked: false
---

# feat: Mother DuckDB + DuckLake Federation

## Problem

Patina's per-project SQLite model is the right ownership boundary, but there is
no built federation substrate in Mother for cross-project analytic queries.
The earlier greenfield data-platform direction explicitly queued DuckDB
federation as future work, and MVP 2 (`multiproject-belief-share`) needs this
substrate for query-time cross-project reads.

## Goal

Build a Mother-owned DuckDB federation layer (with optional DuckLake extension)
that sits above project SQLite stores, without replacing project sovereignty.

## Non-Goals

- Replacing per-project SQLite as source of truth.
- Reintroducing `patina-ducklake` child behavior as storage authority.
- Tying federation availability to baseline local protocol execution.
- Building full distributed transport in this spec (handled by downstream specs).

## Architecture

### Storage roles

- **SQLite (project-scoped):** authoritative local truth for project-owned data.
- **DuckDB (Mother-scoped):** federation query engine over many project stores.
- **DuckLake extension (optional):** Mother-side capability for lakehouse-style
  catalogs/tables when available on host.

### Boundary rules

- Mother owns the DuckDB process/file and extension loading policy.
- Children remain engine-agnostic; they consume toys/contracts, not backend
  engine details.
- Federation failures cannot break standalone local project workflows.

## Minimal Surface

Mother provides three operations:

1. **status** — federation readiness and attached projects
2. **refresh** — re-scan/re-attach known project stores
3. **query** — bounded, read-focused cross-project query execution

CLI/API naming is implementation detail; capability shape is normative.

## Why This Unblocks Canon

`multiproject-belief-share` introduces `query-responder` and cross-project
belief workflows. Those need a stable Mother federation substrate. This spec
builds that substrate first so MVP 2 children can stay reusable and avoid
embedding database-engine logic.

## Phases

### Phase A — federation substrate

- Add Mother federation catalog and lifecycle at
  `~/.patina/mother/federation.duckdb`.
- Build deterministic project attach registry from known project UIDs.

### Phase B — extension and query path

- Implement optional DuckLake extension loading policy.
- Implement bounded read query path (status/refresh/query).

### Phase C — canon integration

- Update `multiproject-belief-share` dependencies and assumptions.
- Add telemetry and failure-boundary tests.

## Verification

```bash
patina spec check mother-duckdb-ducklake-federation --json
cargo check --workspace -q
cargo test -q --workspace mother::federation
```

## Build Readiness

Ready to start. This is foundational work for MVP 2 federation children and is
independent from `child-rename` and `engine-consolidate` sequencing.
