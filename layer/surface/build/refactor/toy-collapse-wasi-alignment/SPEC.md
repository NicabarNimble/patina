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
