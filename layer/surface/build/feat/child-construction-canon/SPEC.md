---
type: feat
id: child-construction-canon
status: active
created: 2026-03-27
sessions:
  origin: 20260327-021039-379187000
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[patina-is-knowledge-layer]]"
  - "[[eventlog-is-truth]]"
  - "[[observation-at-the-boundary]]"
  - "[[wasi-is-foundation-not-option]]"
related:
  - sdk/patina-sdk/
  - children/
  - mother/src/
  - src/child/
child_specs:
  - folder-text-to-parquet
  - multiproject-belief-share
  - e2ee-multimother-chat
exit_criteria:
  - id: ccc1-hard-rules-locked
    text: "Hard rules 1-8 are documented as normative canon."
    checked: false
  - id: ccc2-core-children-built
    text: "6 core reusable children built and proven in MVP 1 (file-system-monitor, content-extractor, schema-enforcer, dedup-filter, record-writer, lakehouse-catalog)."
    checked: false
  - id: ccc3-reuse-proven
    text: "MVP 2 reuses at least 4 children from MVP 1 without modification. Reuse failures documented and children adjusted."
    checked: false
  - id: ccc4-cross-domain-proven
    text: "MVP 3 reuses children from both prior MVPs. Registry proven across pipeline, federation, and P2P domains."
    checked: false
  - id: ccc5-measurement-contract
    text: "Two-tier measurement validated across all three MVPs: Mother-guaranteed automatic, child-declared via `measure`."
    checked: false
  - id: ccc6-sdk-extracted
    text: "SDK guidance extracted from three MVPs: child authoring, toy selection, composition patterns, registry documentation."
    checked: false
  - id: ccc7-deterministic-assembly
    text: "An agent composes children for a stated objective using the recipe format. Demonstrated on at least one objective not in the three MVPs."
    checked: false
---
# feat: child construction canon

## Problem

Patina has three architectural primitives: Mother (orchestration + authority), children (WASM compute), and toys (sandbox interfaces). But there is no registry of reusable children, no composition model, and no guidance for how an agent or user can assemble children for an objective.

Building bespoke children per use case doesn't scale. The system needs general-purpose reusable children that compose into diverse systems — data pipelines, knowledge federation, P2P communication — using the same rules and the same children.

## Goal

Build a registry of reusable children, proven across three orthogonal domains, that compose into working systems for any stated objective.

Three tiers of people use this system:

1. **Users** — run Mother on their machine. Toys come with Mother upgrades. Children come from the registry (Patina-maintained or third-party). Users describe what they need; an LLM selects children from the registry, generates a composition manifest, and Mother orchestrates. Users never write code.

2. **Developers** — build new children using the SDK. Any language that compiles to WASM via the Component Model (Rust, Python via componentize-py, JavaScript via jco). Publish children to the registry. The SDK is the primary developer surface.

3. **Platform (Patina core)** — maintain Mother, toys, and SDK. Toy changes are platform-level: propose → spec → implement → stabilize → lock. Aligned with WASI governance. Custom Patina toys follow WASI conventions so they can be proposed upstream to the Component Model ecosystem.

## Non-Goals

- Building bespoke children per objective.
- Forcing one composition for every objective.
- Adding runtime capability escalation or dynamic toy discovery.

## Only Three Things

The architecture has exactly three concepts. Nothing else.

**Mother** — authority boundary, orchestration, observation.
- Grants toys to children
- Orchestrates lifecycle (tick, start, stop)
- Mediates all external access (credentials, storage, network)
- Observes children at the boundary (metrics, telemetry)
- Routes events between children (event bus)
- Upgrades bring new toys

**Children** — WASM components that do compute.
- Use only granted toys
- Never self-escalate
- Communicate through events (publish via `wasi:messaging`, subscribe via `patina:events-stream`)
- Declare all needs in manifest
- Configured by manifest for specific use (same child, different config = different behavior)
- Built by developers, composed by users/agents, orchestrated by Mother

**Toys** — controlled openings in the WASM sandbox wall.
- The ONLY way a child touches the outside world
- Granted by Mother, declared in manifest
- WASI standard where available, Patina delta where not
- New toys require platform-level approval (like WASI proposals)
- Toy litmus test: "Why can't the child do this from pure WASM compute?" If it can, it's child code, not a toy.

## Hard Rules (normative)

1. Mother is authority boundary.
   - Children propose or execute; Mother authorizes and orchestrates.

2. Toys are explicit grants.
   - Children never self-grant or dynamically escalate capabilities.
   - Authority is declared in manifest via `[needs].toys` and optional scopes.

3. Least-privilege toyboxes.
   - Prefer narrower grants and split children where it reduces privilege.

4. Append-only truth for telemetry lanes.
   - Raw ingest layers are immutable; corrections happen in derived layers.

5. Idempotent reruns and checkpoint recovery.
   - Objective flows must be safe under retry and restart.

6. Provenance retention.
   - Derived outputs must retain trace-back to source records.

7. Observation at the boundary.
   - Each layer observes the layer below it at the interface, never inside it.
   - Mother measures children (lifecycle, toy call latency, throughput, errors).
   - Children measure toys (response shape, success/failure, volume).
   - Infrastructure telemetry is Mother's responsibility; children are observable without implementing measurement internals.

8. WASI is foundation, not option.
   - Toys use standard WASI interfaces where they exist; custom interfaces cover only the delta.
   - Where WASI covers most of a need, compose on top of the standard — do not reinvent the covered portion.
   - Where WASI covers a fraction, still use that fraction as foundation and build the rest alongside it.
   - Custom interfaces follow WASI conventions so they can contribute upstream to the Component Model ecosystem.

## Children Registry

Reusable children, each providing one general capability, configured by manifest, composable via events.

### Core children (built in MVP 1)

| Child | Capability | Toys |
|---|---|---|
| `file-system-monitor` | Watch folder for changes, emit file-found events | `wasi:filesystem`, `wasi:keyvalue`, `wasi:messaging/producer`, `wasi:logging` |
| `content-extractor` | Blob → structured records with provenance | `wasi:filesystem`, `patina:events-stream`, `wasi:messaging/producer`, `wasi:logging` |
| `schema-enforcer` | Validate records against declared schema | `patina:events-stream`, `wasi:messaging/producer`, `wasi:logging` |
| `dedup-filter` | Reject duplicate records by content hash | `patina:events-stream`, `wasi:keyvalue`, `wasi:messaging/producer`, `wasi:logging` |
| `record-writer` | Batch records → parquet files with partitioning | `patina:events-stream`, `wasi:filesystem`, `wasi:keyvalue`, `wasi:logging`, `patina:measure` |
| `lakehouse-catalog` | Manage tables over parquet (schema, evolution, registration) | `wasi:sql`, `wasi:keyvalue`, `wasi:logging` |

### Federation children (built in MVP 2)

| Child | Capability | Toys |
|---|---|---|
| `event-router` | Subscribe to events, apply rules, republish | `patina:events-stream`, `wasi:messaging/producer`, `wasi:keyvalue`, `wasi:logging` |
| `encryption-envelope` | Field-level encrypt/decrypt for records | `patina:events-stream`, `wasi:messaging/producer`, `patina:crypto`, `wasi:logging` |
| `query-responder` | Answer queries against lake data | `wasi:sql`, `patina:events-stream`, `wasi:messaging/producer`, `wasi:logging` |

### P2P children (built in MVP 3)

| Child | Capability | Toys |
|---|---|---|
| `message-relay` | Relay messages between peers/Mothers | `patina:events-stream`, `wasi:messaging/producer`, `wasi:http`, `patina:connect`, `wasi:logging` |
| `notification-emitter` | Send alerts/notifications based on event patterns | `patina:events-stream`, `wasi:http`, `patina:connect`, `wasi:logging` |

### Reuse across MVPs

| Child | MVP 1: folder-text-to-parquet | MVP 2: multiproject-belief-share | MVP 3: e2ee-multimother-chat |
|---|---|---|---|
| file-system-monitor | **build** | | |
| content-extractor | **build** | | |
| schema-enforcer | **build** | reuse | reuse |
| dedup-filter | **build** | reuse | |
| record-writer | **build** | reuse | reuse |
| lakehouse-catalog | **build** | reuse | |
| event-router | | **build** | reuse |
| encryption-envelope | | **build** | reuse |
| query-responder | | **build** | |
| message-relay | | | **build** |
| notification-emitter | | | **build** |

## Toy Lifecycle

Toys are the platform API. New toys are rare and intentional.

**Approval process (aligned with WASI governance):**
1. **Propose** — developer identifies a capability that fails the toy litmus test (can't be done from pure WASM compute)
2. **Spec** — WIT definition, security analysis, hard-rule compliance
3. **Implement** — Mother host implementation, SDK integration
4. **Stabilize** — used by multiple children, no interface changes for N releases
5. **Lock** — stable API, breaking changes require major version

**Upstream contribution:** When a Patina toy stabilizes and the broader Component Model ecosystem needs it, propose it as a WASI standard. `patina:events-stream`, `patina:measure`, and `patina:connect` are candidates.

**Known toy needed:** `patina:crypto` — field-level encryption where Mother holds keys, child calls encrypt/decrypt. Required for `encryption-envelope` child. Credentials (keys) never cross the WASM wall.

## Component Model Ecosystem

Patina children are WASM components. They use WIT interfaces. They compile to `wasm32-wasip2`.

**What this enables:**
- **Language freedom** — Rust, Python (componentize-py), JavaScript (jco), Go, C. Developer picks their language.
- **Ecosystem portability** — children using only WASI imports can run in other WASM hosts (wasmCloud, Spin).
- **Inbound portability** — WASM components from other ecosystems can run in Mother if they import compatible interfaces.
- **Upstream contribution** — Patina's custom toys, composition model, and observation patterns can inform Component Model standards.

## Objective Recipe Format

Every objective recipe defines:

- `objective_id`
- `user_intent`
- `children` (list of children from registry with configuration and manifest overrides)
- `composition` (how children connect — event stream wiring)
- `measurement_contract` (two tiers: Mother-guaranteed + child-declared)
- `acceptance_gates` (which tier observes each gate)
- `failure_recovery`

Domain-specific fields (when relevant):

- `input_sources` / `expected_outputs` (data objectives)
- `checkpoint_plan` / `dedupe_strategy` (stateful objectives)
- `trust_model` (multi-Mother or P2P objectives)
- `encryption_requirements` (privacy-sensitive objectives)

## Unknowns (resolved during build, not before)

These are assumptions the design depends on that have not been proven. They will be encountered and resolved during construction — not tested in isolation first. When an unknown is hit, the resolution is documented in the session and the spec is adapted with user approval.

### Technical unknowns

| Unknown | When we'll hit it | What breaks if wrong |
|---|---|---|
| parquet-rs compiles to `wasm32-wasip2` | Building record-writer in MVP 1 | Parquet writing moves to Mother-side via a toy, or alternative serialization format |
| Two children compose via events with acceptable latency | First split from composite to focused children in MVP 1 | Composition model needs rethinking — batching, direct calls, or fewer children |
| `wasi:filesystem` write works with scoped preopens | Building record-writer in MVP 1 | Output writes move to a storage toy instead of direct filesystem |
| Connect binding injection works end-to-end | First child that makes authenticated HTTP calls | Debug the `WasiHttpView` implementation — it was built but lightly tested |
| `patina:crypto` toy design is viable | Building encryption-envelope in MVP 2 | Encryption moves to Mother-side or storage-layer only |

### Model unknowns

| Unknown | When we'll hit it | What breaks if wrong |
|---|---|---|
| Children are reusable across objectives without modification | MVP 2 tries to reuse 4 children from MVP 1 | Children need configuration generalization or the "one child, one capability" model is too rigid |
| Event payload contracts are stable enough for cross-child composition | Any multi-child composition | Need typed event schemas, a schema registry, or tighter coupling |
| Mother-tier automatic observation produces useful metrics | MVP 1 when we check acceptance gates | Observation-at-the-boundary belief needs revision — may need explicit instrumentation |
| DuckLake catalog maps cleanly to Iceberg for portability | Future lakehouse migration (not in three MVPs) | Portability claim is weaker than stated — may require data migration |

### Aspirational claims (not validated by three MVPs)

These are in the spec as goals but won't be proven during the three MVPs. They're honest about being aspirational:

- LLM-driven deterministic assembly from registry
- Language freedom (only Rust tested)
- Ecosystem portability to wasmCloud/Spin
- Upstream contribution to WASI standards
- Three tiers of people (users/developers/platform)

## Adaptation Protocol

When an unknown is hit during build:

1. **Document what happened** — in the session artifact, with evidence.
2. **Propose adaptation** — what changes in the spec, the design, or the approach.
3. **User approves** — no spec change without explicit user approval.
4. **Update spec** — the spec reflects reality, not the original design.
5. **Capture as belief if structural** — if the resolution reveals a principle, capture it.

Specs are living documents. They start as hypotheses and converge on truth through building. The build is the proof.

## Solution Phases

### Phase A — canon lock

- Publish hard rules (1-8) as normative canon.
- Publish children registry structure and recipe format.

### Phase B — build core children (MVP 1)

- `folder-text-to-parquet` child spec builds 6 core reusable children.
- Each child proven end-to-end with real data and measurements.

### Phase C — prove reuse (MVP 2)

- `multiproject-belief-share` child spec reuses 4 children from MVP 1, builds 3 federation children.
- Reuse failures documented and children adjusted.

### Phase D — prove cross-domain (MVP 3)

- `e2ee-multimother-chat` child spec reuses children from both prior MVPs, builds 2 P2P children.
- Registry proven across pipeline, federation, and P2P domains.

### Phase E — SDK extraction

- SDK guidance extracted from all three MVPs.
- Child authoring guide, composition patterns, registry documentation.
- Deterministic assembly demonstrated: agent composes children for a new objective.

## Verification

```bash
patina spec check child-construction-canon --json
cargo check --workspace -q
cargo test -q --workspace
```

## Build Readiness

Phase A (canon lock) is ready. Phase B (core children via folder-text-to-parquet) is the next action.
