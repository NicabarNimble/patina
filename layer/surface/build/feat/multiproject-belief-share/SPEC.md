---
type: feat
id: multiproject-belief-share
status: draft
created: 2026-03-27
parent: child-construction-canon
sessions:
  origin: 20260327-104954-066673000
blocked_by:
  - folder-text-to-parquet
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[patina-is-knowledge-layer]]"
  - "[[observation-at-the-boundary]]"
  - "[[wasi-is-foundation-not-option]]"
related:
  - sdk/patina-sdk/
  - children/
  - layer/surface/epistemic/
  - layer/surface/build/feat/child-construction-canon/
exit_criteria:
  - id: mbs1-core-children-reused
    text: "At least 4 children from MVP 1 reused without modification: record-writer, schema-enforcer, dedup-filter, lakehouse-catalog."
    checked: false
  - id: mbs2-reuse-failures-documented
    text: "Any child that required modification has the failure documented and the child adjusted."
    checked: false
  - id: mbs3-federation-children-built
    text: "3 new federation children built: event-router, encryption-envelope, query-responder."
    checked: false
  - id: mbs4-trust-model
    text: "Trust model defined and implemented — which beliefs a project accepts, filtering by source/confidence/facet."
    checked: false
  - id: mbs5-provenance-survives
    text: "Belief provenance (evidence, sessions, attack/support links) survives cross-project transfer intact."
    checked: false
  - id: mbs6-conflict-resolution
    text: "Competing beliefs from different projects surfaced and resolved (or coexist with provenance)."
    checked: false
  - id: mbs7-recipe-validated
    text: "Objective recipe filled in with concrete values including trust_model field."
    checked: false
---
# feat: multiproject-belief-share

## Problem

Epistemic beliefs are project-local. Insights from one Patina project cannot inform another. This MVP proves the registry model by reusing core children from MVP 1 and building 3 new federation children.

## Goal

Share epistemic beliefs across independent Patina projects with provenance and trust controls. In doing so, prove that core children are genuinely reusable and build federation children for the registry.

The critical test: **can record-writer, schema-enforcer, dedup-filter, and lakehouse-catalog be reused from MVP 1 without modification?** If yes, the registry model works. If no, we learn where and fix the children.

## Non-Goals

- Real-time belief synchronization in this phase.
- Cross-project session sharing (beliefs only).
- Merging entire knowledge bases automatically.

## Blocks Reused from MVP 1

| Child | Used for | Expected modification |
|---|---|---|
| `record-writer` | Persist shared beliefs as parquet records | None — records are records |
| `schema-enforcer` | Validate incoming beliefs against belief schema | None — schema is configurable |
| `dedup-filter` | Reject duplicate belief imports by content hash | None — dedup by hash is generic |
| `lakehouse-catalog` | Manage belief tables, schema evolution | None — tables are tables |

## New Children Built

### 7. `event-router`

**Capability:** Subscribe to events, apply routing rules, republish to different streams.

**Toys:** `patina:events-stream`, `wasi:messaging/producer`, `wasi:keyvalue` (routing rules), `wasi:logging`

**How it works:** Subscribes to one or more event streams. Applies configurable rules (filter by type, transform payload, route to target stream). Publishes matching events to configured output streams. Rules declared in manifest.

**Reuse:** any multi-child composition, notification pipelines, audit trails, chat message routing.

### 8. `encryption-envelope`

**Capability:** Field-level encrypt/decrypt for records in transit.

**Toys:** `patina:events-stream`, `wasi:messaging/producer`, `wasi:logging`

**How it works:** Subscribes to events containing records. Encrypts configured fields before republishing for cross-project transfer. Decrypts on import side. Key management is Mother's responsibility — child receives encrypt/decrypt capability via toy grant, never sees raw keys.

**Reuse:** any cross-boundary data transfer, chat messages, sensitive record sharing.

### 9. `query-responder`

**Capability:** Answer structured queries against lake data and publish results.

**Toys:** `wasi:sql`, `patina:events-stream`, `wasi:messaging/producer`, `wasi:logging`

**How it works:** Subscribes to `query.request` events. Executes parameterized SQL against lakehouse catalog. Publishes `query.result` events. Manifest configures which tables are queryable and what access scope the child has.

**Reuse:** any system that needs to read from the lake — dashboards, analysis, cross-project queries.

## Composition

```
[source project]
event-router (filter exportable beliefs)
    → [belief.export] →
encryption-envelope (encrypt sensitive fields)
    → [belief.encrypted] →
    ... transport (iroh, file, API — TBD) ...
    → [belief.received] →
[target project]
encryption-envelope (decrypt)
    → [belief.decrypted] →
schema-enforcer (validate belief schema)
    → [belief.validated] →
dedup-filter (reject duplicates)
    → [belief.ready] →
record-writer (persist to parquet)
    → [file.written] →
lakehouse-catalog (register in belief table)
```

## Unknowns (resolved during build)

| Unknown | When we'll hit it | What breaks if wrong |
|---|---|---|
| Children from MVP 1 reuse without modification | First attempt to compose MVP 1 children with belief data | The "reusable children" thesis — requires child generalization or accepting some bespoke work |
| `patina:crypto` toy design works for field-level encryption | Building encryption-envelope child | Encryption moves to storage-layer only, or Mother-side processing |
| Event payload contracts from MVP 1 work for belief records | First belief flowing through schema-enforcer → dedup-filter → record-writer | Need to generalize event schemas or add belief-specific adapters |
| Trust filtering is solvable with event-router rules | Implementing trust model | May need a dedicated trust-filter child — another registry entry |

## Open Design Questions

- Transport between projects: iroh, file exchange, API? Resolved during build — the children don't care how beliefs arrive, they process events.
- Trust filtering: event-router rules or a dedicated trust-filter child? Resolved during build.
- Conflict resolution: separate child or query-responder + human review? Resolved during build.

## Acceptance Gates

- 4+ children reused from MVP 1 without modification. *(registry validation)*
- Provenance completeness survives cross-project transfer. *(integration test)*
- Duplicate beliefs rejected on re-import. *(dedup-filter test)*
- Schema evolution works for belief format changes. *(lakehouse-catalog test)*

## Verification

```bash
patina spec check multiproject-belief-share --json
cargo check --workspace -q
cargo test -q --workspace
```

## Build Readiness

Blocked by `folder-text-to-parquet`. The core children must exist before this MVP can prove reuse.
