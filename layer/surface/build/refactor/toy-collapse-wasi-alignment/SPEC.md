---
type: refactor
id: toy-collapse-wasi-alignment
status: draft
created: 2026-03-26
sessions:
  origin: 20260325-150227-161735000
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[four-roles-no-overlap]]"
  - "[[children-are-wasm]]"
  - "[[root-communicates-identity]]"
related:
  - wit/toys/
  - wit/worlds/
  - sdk/patina-sdk/
  - sdk/patina-sdk-core/
  - sdk/patina-sdk-data/
  - sdk/patina-sdk-agent/
  - src/child/toy_host/
  - src/child/internal/
  - children/
exit_criteria:
  - id: tca1-eight-toys
    text: "Exactly 8 toy WIT interfaces exist in `wit/toys/`: http, fs, log, state, store, events, task, peer. All other toy .wit files are deleted."
    checked: false
  - id: tca2-wasi-aligned
    text: "http, fs, log, state toy interfaces adopt WASI function shapes (wasi:http, wasi:filesystem, wasi:logging, wasi:keyvalue) where applicable."
    checked: false
  - id: tca3-domain-logic-moved
    text: "Domain logic from retired toys (github, lake, connector, belief, graph, session, etc.) is migrated to SDK helper libraries or child code. No domain-specific types in toy WIT interfaces."
    checked: false
  - id: tca4-connections-in-manifest
    text: "`child.toml` supports `[needs.connections]` for named bindings. Mother resolves connection names to toy + credential + config at runtime."
    checked: false
  - id: tca5-sdk-one-crate
    text: "SDK is one crate (`patina-sdk`) with feature flags per toy. Tier sub-crates (`patina-sdk-core`, `patina-sdk-data`, `patina-sdk-agent`) are retired or absorbed."
    checked: false
  - id: tca6-host-mediates-credentials
    text: "All credential injection happens on the host side of the WASM wall. No credential data appears in toy WIT interfaces. Connection-name handles are the only thing children receive."
    checked: false
  - id: tca7-children-migrated
    text: "All in-tree children (ducklake, belief-verifier, spec-manager, session-writer, doctor, lake-manager) build and run using the collapsed toy set."
    checked: false
  - id: tca8-builds-pass
    text: "`cargo check --workspace`, `cargo test -q`, and all children compile and pass tests."
    checked: false
---
# refactor: Collapse toys to primitives and align with WASI/Cloudflare binding model

> Reduce 22 toys to 8 data-access primitives. Adopt WASI interface shapes where standards exist. Move domain logic from toys to children and SDK libraries. Align with Cloudflare Workers binding model for capability grants.

## Problem

Patina has 22 toy WIT interfaces. Many are not true primitives — they're domain-specific APIs baked into the host:

- `github` is an entire GitHub REST client (7 functions, 6 record types) when it's really just `http` + credentials
- `lake` is DuckDB operations (7 functions) when it's really just `store` + a collection name
- `connector` is a sync orchestration protocol when it's really just `http` + credentials + cursor state
- `belief`, `graph`, `query` are identical function signatures (`query` + `mutate`) pointing at different stores
- `checkpoint` is `state` with a stream-scoped key
- `session` is `fs` + `git` + Patina session knowledge
- `measure`, `emit` are structured event emission — subsets of a generic `events` primitive

This violates the toy litmus test: "Why can't the child do this itself from pure WASM compute?" A child CAN construct GitHub API requests, format DuckDB SQL, and serialize belief queries — it just needs host-provided capabilities to reach the network, storage, and event bus. The domain knowledge belongs in the child, not the host.

Cloudflare Workers validates this architecture: they run the world's largest edge compute platform with ~8 primitive binding types (KV, R2, D1, Queues, Service Bindings, Durable Objects, fetch, Secrets). There is no "GitHub binding" — you use `fetch()` with secrets.

WASI validates it independently: `wasi:http`, `wasi:filesystem`, `wasi:keyvalue`, `wasi:logging` are generic primitives. There is no `wasi:github`.

## Session Discoveries (20260325-150227-161735000)

This spec emerged from a chain of discoveries during a single session, not from a planned design exercise:

### 1. Filesystem layout review surfaced the real architecture

Reviewing the greenfield spec arc (5 specs, 4 days, ~2100 lines deleted) and auditing the post-consolidation workspace layout led to the question: "what would a greenfield layout look like?" Answering that forced us to think about crate boundaries, which forced us to think about what `patina-core` should actually contain.

### 2. The belief system is the core of Patina

The core verbs — scrape, scry, assay, oxidize, context — all serve the belief system. Mother/children/toys exist to **extend** the belief system's reach into any domain. But children aren't servants of the belief system — they're autonomous data movers and transformers with bounded agency. They operate within a platform whose core is the belief layer, and their work generates evidence that can flow into it.

### 3. Children have a tighter scope than "anything WASM can do"

In a world of 1000s of children, they're not general-purpose compute. They're **data movers** — ingesting, transforming, and routing data through the platform. This means the toy set should be small and data-oriented. Children don't need GPU compute or audio processing. They need to reach data sources, transform data, and put it somewhere.

### 4. A github toy vs a google workspace toy are just http toys with different creds

This was the breakthrough observation. The 22 toys aren't 22 primitive capabilities. Many are the same capability (HTTP, store, events) with different credentials or domain knowledge baked in. A `github` toy is just `http` + GitHub PAT + knowledge of the GitHub API. That domain knowledge belongs in the child or an SDK library, not in the host interface.

### 5. "Scope" was trying to be three things at once

When we tried to add scoping to the collapsed toys, "scope" was carrying too much: the capability (what you can do), the credential (how you authenticate), and the target (where you're reaching). Separating these led to the **connection** concept — a named binding that Mother resolves, like Cloudflare Workers' `wrangler.toml` bindings.

### 6. Credentials should never cross the WASM wall

The WASM sandbox gives us a real isolation boundary. If credentials stay on Mother's side, a child physically cannot exfiltrate them. Mother injects auth headers into HTTP requests, resolves store paths, manages secrets — the child operates through opaque handles. This is capability-based security, the same model as Cloudflare Workers, Deno, and browser sandboxes.

### 7. We independently arrived at a proven architecture

Comparing with Cloudflare Workers revealed almost 1:1 mapping: our toybox = their bindings, our child.toml = their wrangler.toml, our Mother = their Workers runtime, our connection handles = their opaque binding objects. The WASM component model's import/export mechanism is the same pattern formalized at the type-system level. We took the long road to understand the road, but the destination is well-proven.

### 8. WASI alignment is free if we collapse correctly

4 of our 8 collapsed toys map directly to existing or proposed WASI interfaces (http, filesystem, keyvalue, logging). Our 4 Patina-specific toys (store, events, task, peer) fill gaps the WASI ecosystem hasn't standardized yet. If we design them cleanly — domain-agnostic, with implementation experience — they're natural candidates for WASI proposals. We don't need to plan for that; just building good interfaces makes it possible.

### 9. The toybox concept unifies everything

"Toy" expands to mean anything Mother provides to a child — capabilities, connections, resources. "Toybox" is the complete sealed grant payload. `child.toml` `[needs]` is the request. Mother is the authority that turns requests into grants. This simplifies the mental model: read `child.toml`, you know everything the child can do. Like reading `wrangler.toml`.

## Goal

8 toy primitives. 4 adopt WASI interface shapes. 4 are Patina-specific, designed to be clean enough to propose upstream. Domain logic moves to children and SDK helper libraries. The SDK simplifies from 4 crates to 1.

## Non-Goals

- Do NOT change Mother's lifecycle management or child kind system.
- Do NOT change the belief system or core verbs (scrape, scry, assay, oxidize, context).
- Do NOT adopt WASI interfaces wholesale where they don't fit Patina's needs — align with shapes, don't force-fit.
- Do NOT build a WASI registry or package publishing pipeline.
- Do NOT extract children to separate repos (separate effort).

## Current State

### 22 Toy WIT Interfaces

| Toy | Functions | What it actually is |
|-----|-----------|-------------------|
| `log` | 1 | Primitive — keep |
| `state` | 4 | Primitive — keep, align with wasi:keyvalue |
| `http` | 2 | Primitive — keep, align with wasi:http |
| `layer-fs` | 6 | Primitive — keep as `fs`, align with wasi:filesystem |
| `events` | 3 | Primitive — keep |
| `task` | 1 | Primitive — keep |
| `peer` | 2 | Primitive — keep |
| `git` | 6 | Borderline — keep (real host capability) |
| `github` | 7 | Domain logic — collapse into `http` + SDK helper |
| `connector` | 4 | Domain logic — collapse into `http` + SDK helper |
| `ingress` | 2 | Domain logic — collapse into `http` + SDK helper |
| `lake` | 7 | Domain logic — collapse into `store` |
| `belief` | 2 | Domain logic — collapse into `store` |
| `graph` | 2 | Domain logic — collapse into `store` |
| `query` | 1 | Domain logic — collapse into `store` |
| `emit` | 1 | Domain logic — collapse into `events` |
| `measure` | 1 | Domain logic — collapse into `events` |
| `checkpoint` | 2 | Domain logic — collapse into `state` |
| `session` | 8 | Domain logic — collapse into `fs` + `git` + SDK helper |
| `schema` | 0 | Support types — absorb into relevant toys |
| `types` | 0 | Support types — absorb into relevant toys |
| `layer` | varies | Overlaps with `fs` — collapse |

### SDK Structure (4 crates)

- `patina-sdk` — umbrella re-export
- `patina-sdk-core` — core toys (log, state, types, task, events, peer)
- `patina-sdk-data` — data toys (lake, checkpoint, measure, github, connector)
- `patina-sdk-agent` — agent toys (query, emit, session)

### child.toml (current)

```toml
[needs]
toys = ["log", "state", "lake", "connector", "checkpoint", "events"]
```

No connection concept. Toy names bake in domain assumptions.

## Target State

### 8 Toy WIT Interfaces

| Toy | Origin | Connection-aware | Description |
|-----|--------|-----------------|-------------|
| `http` | WASI-aligned | Yes — connection name maps to endpoint + credentials | Outbound HTTP requests |
| `fs` | WASI-aligned | No — Mother scopes paths | File read/write/list within granted paths |
| `log` | WASI-aligned | No | Structured logging output |
| `state` | WASI-aligned | No | Key-value persistence for child working memory |
| `store` | Patina-built | Yes — connection name maps to data store | Structured data query/mutate |
| `events` | Patina-built | No — stream names are the scope | Pub/sub with offset tracking and ack |
| `task` | Patina-built | No | Deferred work scheduling |
| `peer` | Patina-built | Yes — child name is the target | Child-to-child communication via Mother |

### SDK Structure (1 crate)

```toml
[dependencies]
patina-sdk = { version = "1.0", features = ["knowledge-child", "http", "store", "events", "log"] }
```

Feature flags per toy. Convenience helpers for domain logic (github, lake, session) as library modules, not toys.

### child.toml (target)

```toml
name = "ducklake"
kind = "knowledge-child"

[needs]
toys = ["http", "store", "events", "log"]

[needs.connections]
github = { toy = "http" }
gitlab = { toy = "http" }
ducklake = { toy = "store" }

[provides]
child = "ducklake"

[relationships]
emits = ["data-ingested"]
listens = ["sync-requested"]
```

## Solution

### Phase 1: Design the 8 toy WIT interfaces

Write the 8 new `.wit` files. For WASI-aligned toys, study the WASI interface shapes and adopt where they fit. For Patina-specific toys, design clean, domain-agnostic interfaces.

Key design principle: toys take a `connection` parameter (string handle) where they need to reach external resources. Mother resolves the handle to real infrastructure. The child never sees credentials.

### Phase 2: Build connection-aware child.toml parsing

Update `child.toml` schema to support `[needs.connections]`. Update Mother's manifest parsing to resolve connections at child load time. This is the toybox assembly path.

### Phase 3: Update toy host implementations

Rewrite `src/child/toy_host/` to implement the 8 new interfaces. The toy host becomes connection-aware: it checks the toybox for connection grants and injects credentials on the host side.

### Phase 4: Create SDK helper libraries

Move domain logic from retired toys into SDK helper modules:

- `sdk::helpers::github` — constructs GitHub API requests using `http` toy
- `sdk::helpers::lake` — constructs DuckDB operations using `store` toy
- `sdk::helpers::session` — manages session artifacts using `fs` + events
- etc.

These are library code, not toys. They use toys under the hood.

### Phase 5: Migrate children

Update all in-tree children to use collapsed toys + SDK helpers:

- `ducklake` → `http` + `store` + `events` (was: lake, connector, checkpoint, events)
- `belief-verifier` → `store` + `events` (was: belief, events, checkpoint)
- `spec-manager` → `fs` + `store` (was: layer-fs, belief, session)
- `session-writer` → `fs` + `events` (was: layer-fs, session, events)
- `doctor` → `store` + `log` (was: knowledge-child stub)
- `lake-manager` → `store` + `http` (was: lake, connector)

### Phase 6: Collapse SDK crates

Absorb `patina-sdk-core`, `patina-sdk-data`, `patina-sdk-agent` into `patina-sdk`. One crate, feature flags per toy, helper modules for domain logic.

### Phase 7: Clean up

Delete retired toy WIT files. Delete retired toy host implementations. Update all documentation. Remove `wit/worlds/` mirror copies (addressed by `wit-contract-single-source` spec).

## Implementation Order

Phases 1-2 can proceed in parallel (WIT design + manifest parsing).
Phase 3 depends on Phase 1 (toy host implements new WIT).
Phase 4 depends on Phase 1 (helpers use new toy bindings).
Phase 5 depends on Phases 3-4 (children use new host + helpers).
Phase 6 depends on Phase 5 (SDK consolidation after children migrate).
Phase 7 depends on Phase 6 (cleanup after everything works).

## Resolved Decisions

- **Toy litmus test**: "Why can't the child do this itself from pure WASM compute?" If it can, it's SDK/library, not a toy.
- **Credentials never cross the WASM wall.** Children operate through connection-name handles. Mother injects credentials on the host side.
- **Connections are named bindings**, like Cloudflare Workers' `wrangler.toml` bindings. The child says "github." Mother knows what that means.
- **Domain logic belongs in children and SDK helpers, not toys.** A `github::list_issues()` helper uses the `http` toy internally — it's library code, not a host interface.
- **git stays as a toy** (borderline call). Git operations are real host capabilities that WASM can't do alone. If WASI eventually proposes `wasi:git`, we align then.
- **WASI alignment is shape-alignment, not adoption.** We design our interfaces to be close to WASI shapes so migration is easy, but we don't force-fit where Patina's needs differ.
- **The toybox is the sealed capability grant.** Mother assembles it from `child.toml`, resolves connections and credentials, and grants exactly what's declared. The manifest is the single source of truth for security review.

## Breaking Impact

This is a large breaking change. It touches every layer between WIT and child code. The risk is scope and stamina, not design uncertainty — each change has a clear before/after.

### What breaks

**WIT layer — full rewrite:**
- 22 toy `.wit` files replaced by 8-9 new interface designs (not renames — new function signatures)
- `wit/worlds/` — every composed world regenerated for collapsed toy set
- All SDK `build.rs` files — different WIT sources to bind against

**Toy host — full rewrite of dispatch:**
- `src/child/toy_host/` — every file. Domain-specific hosts (github, lake, connector, belief, etc.) deleted. New hosts gain connection-aware credential injection and collection routing.
- Mother's child loading path — must parse `[needs.connections]`, resolve connection names to toy + credential + endpoint, build the toybox with connection metadata.

**SDK — full restructure:**
- 3 tier crates (`patina-sdk-core`, `patina-sdk-data`, `patina-sdk-agent`) absorbed into `patina-sdk`. Every `use patina_sdk_core::*` and `use patina_sdk_data::*` import breaks.
- All existing toy binding modules replaced with new bindings for 8-9 toys.
- New `helpers/` modules written for domain logic that moved out of toys (github, lake, session, connector).

**Every in-tree child — must migrate:**
- `ducklake` — `lake::append_json_batch()` → `store::mutate("ducklake", ...)` or `sdk::helpers::lake::append()`
- `belief-verifier` — `belief::query()` → `store::query("beliefs", ...)`
- `spec-manager` — `session::write_artifact()` → `fs::write()` + `events::publish()` or `sdk::helpers::session`
- Same for `session-writer`, `doctor`, `lake-manager`

**Every `child.toml` — manifest schema change:**
- Old: `toys = ["lake", "connector", "checkpoint", "events"]`
- New: `toys = ["http", "store", "events", "log"]` + `[needs.connections]` section
- Old toy names become invalid

**Tests — widespread breakage:**
- `src/child/internal/tests.rs` — tests that exercise toy dispatch
- Integration tests that load children with old toy imports
- SDK unit tests

### What does NOT break

- **Core verbs** (scrape, scry, assay, oxidize, context) — no toy dependencies
- **`patina-core`, `patina-protocol`** — no toy dependencies
- **Mother's service layer** (secrets, sessions, health) — internal, not toy-mediated
- **CLI command structure** — commands don't call toys directly
- **Belief system, layer, database** — core infrastructure untouched
- **Git history, session archives** — frozen, never rewritten
- **Grammars** — pipeline grammar crates have no toy dependencies

### Mitigation

The 7-phase execution order exists to make this survivable:
- Each phase leaves the workspace green (compiling and tests passing)
- Phase 1 (WIT design) is pure design — nothing breaks until Phase 3 (host rewrite)
- Phase 5 (child migration) can proceed one child at a time
- Phase 6 (SDK consolidation) can keep tier crates as thin re-exports during transition
- Old `child.toml` toy names could be supported during a migration window via compatibility parsing in Mother's manifest loader (open question: hard cut vs migration period)

## Security Model

Capability-based security, same pattern as Cloudflare Workers:

1. **No ambient authority** — a child starts with zero capabilities
2. **Toys granted explicitly** — only what `child.toml` declares
3. **Connections are opaque handles** — child says name, Mother resolves infrastructure
4. **Credentials stay host-side** — never serialized across the WASM boundary
5. **Mother mediates every call** — can inspect, throttle, filter, revoke, audit
6. **Manifest is auditable** — read `child.toml`, you know everything the child can do
7. **Peer calls preserve boundaries** — calling another child via `peer` gives you that child's API, not its toybox

## Verification

```bash
cargo check --workspace -q
cargo test -q
# Verify exactly 8 toy WIT files:
ls wit/toys/*.wit | wc -l  # should be 8
# Verify no domain-specific types in toy interfaces:
rg "issue|pull-request|review|granted-lake|repo-binding" wit/toys/  # should be 0
# Verify children build:
for child in children/*/; do cargo check -p $(basename $child) 2>/dev/null; done
# Verify SDK is one crate:
grep 'patina-sdk-core\|patina-sdk-data\|patina-sdk-agent' Cargo.toml  # should be 0 in workspace members
```

## Build Readiness

Phase 1 (WIT design) is ready to start. Requires deep review of WASI interface shapes for http, filesystem, keyvalue, logging before committing to Patina's versions.

## Relationship to Other Specs

- **`wit-contract-single-source`** — still valid but should execute AFTER this spec. No point eliminating copies of 22 toy files if we're about to collapse to 8.
- **`greenfield-crate-extraction`** — still valid but should execute AFTER this spec. The engine crate's toy host is simpler with 8 interfaces than 22.
- Both specs should be updated to reflect the collapsed toy set once this spec completes Phase 1 (WIT design).
