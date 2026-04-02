# Design: child construction canon

## Why This Design

The project has stable primitives (Mother orchestration, capability-granted toys, WASM children via Component Model). The missing piece is a registry of reusable children that compose into diverse systems — data pipelines, knowledge federation, P2P communication — using the same rules and the same children.

Children are not bespoke parts of bespoke pipelines. They are general-purpose legos. A `record-writer` child is the same child whether it's writing text records, belief exports, or chat history. The objective determines which children compose and how — not what the children are.

## Build Target

1. Lock hard rules (1-8) for Mother/child/toy behavior.
2. Build a registry of reusable children through three orthogonal MVPs.
3. Prove reuse: MVP 2 reuses children from MVP 1, MVP 3 reuses children from both.
4. Extract SDK guidance from the experience of building and composing children.
5. Demonstrate deterministic assembly: an agent selects children for a stated objective.

## Resolved Decisions

- Children are reusable children, not objective-specific parts.
- A child provides one general capability, configured by manifest for specific use.
- Objectives are composed from registry children via event streams.
- Focused children are the default — one capability, narrow toybox.
- Component Model composition handles tight-coupling edge cases.
- Mother remains final authority for grants and orchestration.
- Children never self-escalate capability.
- Each layer observes the layer below at the boundary, never inside.
- WASI standards are used directly; custom toys cover only the delta.
- Measurement is two-tier: Mother-guaranteed automatic, child-declared optional.
- The target state is deterministic assembly by agents from the registry.
- Encrypt at storage layer, not parquet layer (for maximum tool compatibility).
- DuckLake for registry now, designed for Iceberg/Delta portability.

## Children Registry

### Core children (MVP 1: folder-text-to-parquet)

1. `file-system-monitor` — watch folder, emit file events
2. `content-extractor` — blob → structured records with provenance
3. `schema-enforcer` — validate records against declared schema
4. `dedup-filter` — reject duplicates by content hash
5. `record-writer` — batch records → parquet files
6. `lakehouse-registry` — manage tables over parquet (DuckLake)

### Federation children (MVP 2: multiproject-belief-share)

7. `event-router` — subscribe, apply rules, republish
8. `encryption-envelope` — field-level encrypt/decrypt
9. `query-responder` — answer queries against lake data

### P2P children (MVP 3: e2ee-multimother-chat)

10. `message-relay` — relay messages between Mothers
11. `notification-emitter` — alerts based on event patterns

## Child Specs

| Child Spec | Domain | Blocks built | Blocks reused |
|---|---|---|---|
| `folder-text-to-parquet` | data pipeline | 1-6 | — |
| `multiproject-belief-share` | knowledge federation | 7-9 | 3,4,5,6 from MVP 1 |
| `e2ee-multimother-chat` | P2P communication | 10-11 | 5,6,7,8 from MVPs 1+2 |

If children compose across all three domains, the registry is general. If they break, we learn where and fix them.

## Canon Surfaces

### 1) Hard Rule Surface

Eight normative rules: authority boundary, explicit grants, least-privilege, append-only truth, idempotent reruns, provenance, observation at boundary, WASI foundation.

### 2) Children Registry Surface

Reusable children with declared capabilities, toy requirements, event interfaces. Blocks are the unit of composition. The registry grows with each MVP.

### 3) Objective Recipe Surface

Composition format: which children, which configuration, which event streams, which acceptance gates. Domain-specific fields (trust model, encryption, checkpoint) added when relevant.

### 4) Measurement Surface

Two tiers: Mother-guaranteed (automatic) and child-declared (via `measure` toy with manifest-declared metrics and cardinality bounds).

## Governance Integration

New objectives should include:

1. Child selection from registry (or justification for new child).
2. Composition diagram (event stream wiring).
3. Hard-rule compliance checklist.
4. Measurement contract with tier annotations.
5. Acceptance gates.

## Direct Code Targets

- `children/` — registry child implementations
- `sdk/patina-sdk/` — SDK docs and authoring examples
- `wit/` — interface definitions (WASI + Patina deltas)
- child template guidance under `children/template/`

## Verification Plan

```bash
patina spec check child-construction-canon --json
cargo check --workspace -q
cargo test -q --workspace
```

## Build Readiness

Phase A (canon lock) is ready. Phase B (core registry via folder-text-to-parquet) is the next action. The registry will be validated by building children, composition will be validated by reuse across MVPs.
