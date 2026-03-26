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
  - id: tca0-protocol-lock
    text: "Phase 0 protocol lock is complete before Phase 1 coding: connect/http interaction model, store routing model, scope model, and WASI fitness matrix are all frozen with explicit proofs."
    checked: true
  - id: tca1-toy-count
    text: "10 toy WIT interfaces exist: 2 WASI adopted (wasi:http, wasi:filesystem), 2 WASI-aligned Patina shims (patina:log, patina:state), 1 Patina bridge (patina:connect), 5 Patina-specific (patina:store, patina:events, patina:task, patina:peer, patina:git). All old toy .wit files are deleted."
    checked: false
  - id: tca2-wasi-adopted
    text: "http and fs use actual `wasi:*` package interfaces. log and state use `patina:*` shims that track WASI shapes with documented sunset condition for migration when WASI reaches Phase 4 standardized."
    checked: false
  - id: tca3-connect-bridge
    text: "`patina:connect` exists with opaque `connection` resource type. Children resolve named connections to handles. Credentials are injected host-side when handles are used with WASI toys. Credentials never enter WASM memory."
    checked: false
  - id: tca4-domain-logic-moved
    text: "Domain logic from retired toys (github, lake, connector, belief, graph, session, etc.) is migrated to SDK helper libraries or child code. No domain-specific types in toy WIT interfaces."
    checked: false
  - id: tca5-connections-in-manifest
    text: "`child.toml` supports `[needs.connections]` for named bindings. Mother resolves connection names to connect resource + credential + config at runtime."
    checked: false
  - id: tca6-sdk-one-crate
    text: "SDK is one crate (`patina-sdk`) with feature flags per toy. Tier sub-crates (`patina-sdk-core`, `patina-sdk-data`, `patina-sdk-agent`) are retired or absorbed."
    checked: false
  - id: tca7-host-mediates-credentials
    text: "All credential injection happens via `patina:connect` resource handles on the host side of the WASM wall. No credential data appears in any toy WIT interface."
    checked: false
  - id: tca8-children-migrated
    text: "All in-tree children (ducklake, belief-verifier, spec-manager, session-writer, doctor, lake-manager) build and run using the collapsed toy set."
    checked: false
  - id: tca9-builds-pass
    text: "`cargo check --workspace`, `cargo test -q`, and all children compile and pass tests."
    checked: false
---
# refactor: Collapse toys to primitives and align with WASI/Cloudflare binding model

> Reduce 22 toys to 10 (4 WASI-aligned: 2 adopted + 2 shimmed with sunset, plus 1 Patina bridge + 5 Patina-specific). Adopt `wasi:http` and `wasi:filesystem` now; ship `patina:log` and `patina:state` as WASI-shaped shims until stable adoption. Add `patina:connect` as credential-security bridge. Move domain logic from toys to children and SDK libraries. Align with Cloudflare Workers binding model for capability grants.

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

Beyond bloat, the 22 custom toys represent a missed opportunity: WASI already defines standard interfaces for HTTP, filesystem, key-value, and logging. We were reinventing these rather than adopting them. This prevents Patina from participating in the WASI ecosystem — children can't be portable, we can't benefit from ecosystem tooling, and our Patina-specific extensions (where they're genuinely needed) are harder to distinguish from unnecessary custom work.

Cloudflare Workers validates this architecture: they run the world's largest edge compute platform with ~8 primitive binding types (KV, R2, D1, Queues, Service Bindings, Durable Objects, fetch, Secrets). There is no "GitHub binding" — you use `fetch()` with secrets. Their design model — manifest-declared bindings, capability grants, zero-access default, sealed binding set at init — is the same model Mother uses. Aligning with it keeps our toy surface honest.

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

4 of our collapsed toys map to WASI interfaces (2 adopted now: http, filesystem; 2 shimmed with sunset: keyvalue, logging). Our 5 Patina-specific toys (store, events, task, peer, git), plus the `patina:connect` bridge, fill gaps the WASI ecosystem hasn't standardized yet. If we design them cleanly — domain-agnostic, with implementation experience — they're natural candidates for WASI proposals. We don't need to plan for that; just building good interfaces makes it possible.

### 9. The toybox concept unifies everything

"Toy" expands to mean anything Mother provides to a child — capabilities, connections, resources. "Toybox" is the complete sealed grant payload. `child.toml` `[needs]` is the request. Mother is the authority that turns requests into grants. This simplifies the mental model: read `child.toml`, you know everything the child can do. Like reading `wrangler.toml`.

## Goal

Collapse 22 toys to 10 primitives by embracing WASI where WASI exists and expanding where Patina's data-mover children need capabilities the ecosystem doesn't offer:

- **Embrace WASI as-is**: Adopt `wasi:http` and `wasi:filesystem` directly. Build Patina shims for `wasi:keyvalue` and `wasi:logging` that track WASI shapes and migrate mechanically when stable.
- **Expand with purpose**: `patina:connect` (credential-safe connection bridge), `patina:store`, `patina:events`, `patina:task`, `patina:peer`, `patina:git` — honest extensions born from real use, designed clean enough to propose upstream.
- **Align with Cloudflare's design model**: Manifest-declared bindings, capability grants, zero-access default, sealed toybox at init. Keeps the toy surface honest about what's primitive.

Domain logic moves from toys to children and SDK helper libraries. The SDK simplifies from 4 crates to 1. The toybox — Mother's sealed capability grant assembled from `child.toml` — becomes the explicit architectural centerpiece: the security contract, the audit surface, the portability boundary.

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

Current in-tree child manifests (snapshot):

| Child | Current toys (`child.toml`) |
|-------|-------------------------------|
| `ducklake` | `log`, `state`, `checkpoint`, `lake`, `github`, `measure`, `task`, `peer` |
| `belief-verifier` | `log`, `state`, `checkpoint`, `events`, `belief`, `measure`, `task` |
| `session-writer` | `log`, `state`, `session`, `peer` |
| `spec-manager` | `log`, `state`, `layer-fs`, `git` |
| `doctor` | `log`, `state` |
| `lake-manager` | `log`, `state` |

```toml
[needs]
toys = ["log", "state", "checkpoint", "lake", "github", "measure", "task", "peer"]
```

No connection concept. Toy names bake in domain assumptions.

## Target State

### Toy Architecture: Embrace WASI + Patina bridge + Expand where needed

The toys split into three layers, reflecting the design principle: embrace WASI as WASI is, expand where Patina's system needs it.

**WASI-aligned toys** — adopt existing WASI interfaces. Portable across any WASI runtime.

| Toy | Package (Phase 1) | Target Package | Description |
|-----|-------------------|----------------|-------------|
| `http` | `wasi:http` | `wasi:http` | Adopt now. Standard outbound HTTP. Child constructs request, host executes. |
| `fs` | `wasi:filesystem` | `wasi:filesystem` | Adopt now. File access within granted paths. Mother scopes paths. |
| `log` | `patina:log` | `wasi:logging` | Shim with sunset. Tracks `wasi:logging` shape. Migrate when WASI Phase 4 + stable Wasmtime support. |
| `state` | `patina:state` | `wasi:keyvalue` | Shim with sunset. Tracks `wasi:keyvalue` shape. Migrate when WASI Phase 4 + stable Wasmtime support. |

**Patina bridge toy** — the layer that makes WASI toys safe for a multi-tenant child platform.

| Toy | Package | Description |
|-----|---------|-------------|
| `connect` | `patina:connect` | Named connection resolver with credential-safe request path. Returns opaque resource handle. `connect::request(...)` is the credential-aware HTTP path — Mother injects credentials host-side. Credentials never enter WASM memory. |

**Patina-specific toys** — expand where WASI doesn't cover Patina's needs. Honest extensions born from real use, designed clean enough to propose upstream.

| Toy | Package | Description |
|-----|---------|-------------|
| `store` | `patina:store` | Structured data query/mutate. Connection-handle-aware via `patina:connect`. Host routes to backend. |
| `events` | `patina:events` | Pub/sub with offset tracking and ack. |
| `task` | `patina:task` | Deferred work scheduling. |
| `peer` | `patina:peer` | Child-to-child communication via Mother. Like Cloudflare Service Bindings. |
| `git` | `patina:git` | Version control operations (real host capability). |

**Total: 4 WASI-aligned (2 adopted, 2 shimmed with sunset) + 1 bridge + 5 Patina-specific = 10 toys.**

### How `connect` bridges WASI toys to the toybox model

`wasi:http` has no concept of named connections or credential injection. The component specifies raw URLs. `patina:connect` fills that gap — it's the credential-safe path that makes WASI primitives usable in a multi-tenant child platform:

```wit
// patina:connect — named connection resolver with credential-safe request path
interface connect {
    resource connection;
    resolve: func(name: string) -> result<connection, string>;
    // Informational only; not used for auth/routing decisions.
    base-url: func(conn: borrow<connection>) -> string;
    request: func(
        conn: borrow<connection>,
        method: string,
        path: string,
        headers: list<tuple<string, string>>,
        body: option<list<u8>>,
    ) -> result<http-response, string>;
}
```

The `connection` resource is an opaque host-owned handle. The child holds a reference but cannot inspect its internals (credentials, tokens, rate limits). `connect::request(...)` is the credential-aware path: Mother resolves endpoint + policy + secret from the handle, injects credentials host-side, and executes transport through its `wasi:http` host implementation.

Child flow:
```rust
// 1. Resolve named connection → opaque handle
let github = connect::resolve("github")?;

// 2. Make credential-safe request via connect
//    Mother injects Authorization header host-side
//    Child never sees the PAT
let response = connect::request(&github, "GET", "/repos/owner/name", &[], None)?;
```

Raw `wasi:http` is available only as an explicit opt-in (`http.raw = true` in `child.toml`) and never participates in credential injection. The secure path (`connect::request`) is the default; the open path (raw `wasi:http`) is the exception.

The same pattern works for `patina:store`: the child resolves a connection ("ducklake"), gets an opaque handle, and passes it to `store::query(conn, ...)` or `store::mutate(conn, ...)`. Mother routes to the correct backend from connection metadata.

### SDK Structure (1 crate)

```toml
[dependencies]
patina-sdk = { version = "1.0", features = ["knowledge-child", "http", "connect", "store", "events", "log"] }
```

Feature flags per toy. Convenience helpers for domain logic (github, lake, session) as library modules, not toys.

### child.toml (target)

```toml
name = "ducklake"
kind = "knowledge-child"

[needs]
toys = ["http", "connect", "store", "events", "log"]

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

### Phase 0: Protocol lock + feasibility gate (no implementation)

Before any WIT/runtime/SDK code changes, freeze the protocol decisions that were raised in review.

**Decisions that must be locked:**
1. **`connect` + HTTP model**: secure path is `connect::request(...)`; raw `wasi:http` is an explicit opt-in escape hatch.
2. **`store` routing model**: `store` operations are connection-handle based and host-routed; children never infer backend type from payload shape.
3. **Scope model**: `[needs.connections]` is additive; existing `[needs.scopes]` remains and is merged into grants.
4. **WASI fitness matrix**: each target WASI interface is validated against Wasmtime/tooling reality before adoption claims.

**Exit proof:** protocol section updated and frozen in this spec + design doc, with command-backed feasibility notes for wasi:http / wasi:filesystem / wasi:keyvalue / wasi:logging status in current toolchain.

#### Phase 0 execution notes (2026-03-26)

Protocol lock decisions captured in this spec and `DESIGN.md`:
- `connect::request(...)` is the credential-aware default path.
- raw `wasi:http` is opt-in via `http.raw = true`.
- no URL-prefix credential inference for secret injection.
- store backend routing is host-owned via connection metadata.
- `[needs.connections]` and `[needs.scopes]` coexist and merge into one grant model.

Command-backed feasibility snapshot:
- `cargo check --workspace -q` ✅ pass
- `cargo tree -p patina-ai --depth 1` shows `wasmtime = 41.0.3` + `wasmtime-wasi = 41.0.3` (component model + WASIp2 baseline present)
- `cargo test -q -p patina-ai --lib` ⚠️ 1 known failing baseline test: `session::tests::test_find_project_root_not_in_project`

WASI fit matrix locked for implementation:

| Interface | Phase 0 status | Build policy |
|---|---|---|
| `wasi:http` | Fit for adoption in Phase 1 | Adopt now; connect mediates credential path |
| `wasi:filesystem` | Fit for adoption in Phase 1 | Adopt now; Mother path scope enforcement |
| `wasi:keyvalue` | Not proven stable in current runtime workflow | Start as `patina:state`; migrate on stable/runtime proof |
| `wasi:logging` | Not proven stable in current runtime workflow | Start as `patina:log`; migrate on stable/runtime proof |

### Phase 1: Design the toy WIT interfaces

Write the new `.wit` files:
- For WASI toys (`http`, `fs`): adopt the standard `wasi:*` package interfaces directly.
- For WASI-aligned shims (`log`, `state`): define `patina:*` interfaces that track `wasi:*` shapes and carry explicit sunset migration.
- For the `patina:connect` bridge: design the connection resource type and resolution function.
- For Patina-specific toys (store, events, task, peer, git): design clean, domain-agnostic interfaces.

Key design principle: WASI provides the plumbing. `patina:connect` provides the credential security. Patina-specific toys fill the gaps. Credentials never enter WASM memory.

### Phase 2: Compatibility adapters — old toys dispatch through new

Add adapter layer that maps old toy calls to new toy interfaces. Old children keep working, but under the hood they're using the new contract. Both old and new WIT exist simultaneously.

**Changes only:** runtime dispatch behavior (adapter routing).
**Does NOT change:** interface contracts (old WIT untouched), child business logic (children unchanged).

**Parity gate:** Run full test suite. Same inputs → same outputs for every existing child. Golden fixture comparison for key paths (doctor, sync, session).

### Phase 3: Connection-aware child.toml parsing

Update `child.toml` schema to support `[needs] toys = [...]` + `[needs.connections]` syntax. Existing `[needs.scopes]` semantics remain and coexist with connections. Mother's manifest parser resolves both into one grant model at load time. Old `child.toml` format still works via compat parsing.

**Changes only:** manifest schema (additive, not breaking).
**Does NOT change:** interface contracts, runtime dispatch, child business logic.

**Parity gate:** All existing `child.toml` files parse without error. New connection syntax parses correctly for test fixtures. `patina child list` shows correct toy grants.

### Phase 4: New toy host implementations (alongside old)

Implement host functions for the 10 new WIT interfaces. Connection-aware credential injection for `http` and `store`. Both old and new toy hosts exist simultaneously — old children use old hosts, new children use new hosts.

**Changes only:** runtime dispatch (adds new dispatch paths).
**Does NOT change:** interface contracts (already defined in Phase 1), child business logic, old toy host behavior.

**Parity gate:** A test child built against new WIT compiles, loads, and executes basic toy calls. Old children still work unchanged. `cargo test -q` passes.

### Phase 5: SDK helper libraries

Move domain logic from old toys into SDK helper modules (`sdk::helpers::github`, `sdk::helpers::lake`, `sdk::helpers::session`, etc.). These are library code using new toys internally.

**Changes only:** SDK library surface (additive).
**Does NOT change:** interface contracts, runtime dispatch, child business logic (children haven't migrated yet).

**Parity gate:** SDK compiles. Helper modules have unit tests proving they produce the same API calls as the old domain toys.

### Phase 6: Migrate children — one at a time

Each child migrated individually. Per-child sub-phases:
1. Update `child.toml` to new toy syntax + connections
2. Update child source to use new toy bindings + SDK helpers
3. Verify same behavior via golden fixture comparison
4. Commit as one slice per child

Migration order (risk-first after smoke test):
1. `doctor` (stub, minimal toys)
2. `ducklake` (connect + store + events + task — most complex, longest drift risk)
3. `belief-verifier` (store + events + task)
4. `session-writer` (fs + git + events)
5. `spec-manager` (fs + git)
6. `lake-manager` (minimal today: log + state)

**Changes only:** child business logic (one child per commit).
**Does NOT change:** interface contracts, runtime dispatch, other children.

**Parity gate per child:** Same input → same output. Old and new child can coexist during migration window. `cargo test -q` passes after each child.

**Zero-use gate:** After all children migrate, `rg` for retired toy imports across all `children/*/src/`. Count must be zero.

### Phase 7: Collapse SDK crates

Absorb `patina-sdk-core`, `patina-sdk-data`, `patina-sdk-agent` into `patina-sdk`. Keep tier crates temporarily as thin re-exports. Remove from workspace after zero-import proof.

**Changes only:** SDK crate structure.
**Does NOT change:** interface contracts, runtime dispatch, child business logic (all already using new SDK).

**Parity gate:** `cargo check --workspace`, all children compile. `rg "patina-sdk-core\|patina-sdk-data\|patina-sdk-agent" children/` returns zero.

### Phase 8: Remove old toys

Delete old toy WIT files. Delete old toy host implementations. Delete compatibility adapters from Phase 2. Delete old `child.toml` parsing compat. Delete `wit/worlds/` mirror copies. Update docs.

**Changes only:** cleanup (removal of dead code).
**Does NOT change:** any live behavior (everything already on new toys).

**Parity gate:** `cargo check --workspace`, `cargo test -q`. `ls wit/toys/*.wit | wc -l` = 10. `rg` for any old toy function name in `src/child/toy_host/` returns zero.

## Anti-Sprawl Rule

**No phase can change more than one of:**
- Interface contract (WIT definitions)
- Runtime dispatch behavior (toy host, adapter routing)
- Child business logic (child source code)

If two need to change, split the phase. This prevents the "just get green" cascade where a small WIT tweak ripples into host patches → SDK patches → child patches → test patches → doc patches in a single unbounded commit.

## Implementation Order

```
Phase 0 (protocol lock) ───────────────────┐
Phase 1 (WIT design) ← depends on 0        │
Phase 2 (compat adapters) ← depends on 1   │
Phase 3 (child.toml) ← depends on 0,1      │
Phase 4 (new toy host) ← depends on 0,1    │
Phase 5 (SDK helpers) ← depends on 1       │
Phase 6 (migrate children) ← depends on 2,4,5
Phase 7 (SDK collapse) ← depends on 6
Phase 8 (remove old) ← depends on 6,7
```

Phases 2-5 can overlap where dependencies allow. Phase 6 is the long middle — one child at a time. Phase 8 is the cleanup that only happens after zero-use proof.

## Phase Entry/Exit Invariants

Each phase has explicit entry conditions and exit proofs. **A phase cannot start until its listed entry condition passes.** Phases follow the dependency graph (not strictly sequential).

| Phase | Entry Condition | Exit Proof |
|-------|----------------|------------|
| 0 | Spec is active | Protocol lock complete: connect/http interaction, store routing semantics, scope coexistence, and WASI fit matrix are documented and frozen. |
| 1 | Phase 0 exit proof passes | 10 new `.wit` files exist (4 WASI-aligned: 2 `wasi:*` adopted + 2 `patina:*` WASI-shaped shims, plus 1 connect + 5 Patina-specific), compile with wit-bindgen. Old WIT untouched. |
| 2 | Phase 1 exit proof passes | Compat adapters route old toy calls through new interfaces. Full test suite passes. Golden fixture comparison shows identical output for all existing children. |
| 3 | Phases 0 and 1 exit proofs pass | New `child.toml` syntax parses. Old `child.toml` syntax still parses via compat. `patina child list` shows correct toy grants for both formats. |
| 4 | Phases 0 and 1 exit proofs pass | New toy hosts handle basic calls from a test child built against new WIT. Old toy hosts still serve old children unchanged. `cargo test -q` passes. |
| 5 | Phase 1 exit proof passes | SDK helpers compile. Unit tests prove helpers produce same API calls as old domain toys. |
| 6 | Phases 2, 4, 5 exit proofs all pass | Per-child: same input → same output via golden fixture. After all children: `rg` for retired toy imports across `children/*/src/` returns zero. |
| 7 | Phase 6 exit proof passes | `cargo check --workspace`. `rg "patina-sdk-core\|patina-sdk-data\|patina-sdk-agent" children/` returns zero. Tier crates removed from workspace members. |
| 8 | Phases 6 and 7 exit proofs pass | `ls wit/toys/*.wit \| wc -l` = 10. Zero old toy function names in `src/child/toy_host/`. `cargo check --workspace && cargo test -q`. |

## Rollback Contract

**If a parity gate fails in Phase N:**
1. Revert to the previous phase's baseline commit.
2. Do NOT carry forward partial artifacts from the failed phase.
3. Diagnose the failure before re-attempting.
4. If the failure reveals a design issue in Phase 1 (WIT contracts), the collapse map must be amended before any phase resumes.

**At any point before Phase 8:** Old children work on old toys. New children work on new toys. Both coexist. The workspace is green. Any single phase can be reverted without affecting other phases.

**Phase 8 is irreversible.** Only execute after zero-use proof passes for all old toys across all children, SDK, and host code.

## Frozen Collapse Map

This table is **immutable once Phase 2 starts.** If a mapping needs to change, amend this table in the spec first, then update the compat adapters. Do not change mappings ad-hoc during child migration.

| Old Toy | New Primitive | SDK Helper Path | Connection-Aware |
|---------|--------------|-----------------|-----------------|
| `github` (7 funcs) | `http` | `sdk::helpers::github` | Yes — connection name resolves to github.com + PAT |
| `connector` (4 funcs) | `http` | `sdk::helpers::connector` | Yes — connection name per binding |
| `ingress` (2 funcs) | `http` | `sdk::helpers::ingress` | Yes — connection name per source |
| `lake` (7 funcs) | `store` | `sdk::helpers::lake` | Yes — connection name per lake |
| `belief` (2 funcs) | `store` | child code (trivial `store::query("beliefs", ...)`) | Yes — "beliefs" collection |
| `graph` (2 funcs) | `store` | child code (trivial `store::query("graph", ...)`) | Yes — "graph" collection |
| `query` (1 func) | `store` | child code (trivial) | Yes — collection param |
| `emit` (1 func) | `events` | child code (`events::publish(...)`) | No |
| `measure` (1 func) | `events` | child code (`events::publish("measure", ...)`) | No |
| `checkpoint` (2 funcs) | `state` | child code (key-scoped `state::get/set`) | No |
| `session` (8 funcs) | `fs` + `events` | `sdk::helpers::session` | No — Mother scopes fs paths |
| `layer` (varies) | `fs` | child code | No — Mother scopes fs paths |
| `layer-fs` (6 funcs) | `fs` | renamed, same semantics | No — Mother scopes fs paths |
| `schema` (0 funcs) | deleted | types absorbed into relevant toys | — |
| `types` (0 funcs) | deleted | types absorbed into relevant toys | — |
| `git` (6 funcs) | `git` (kept) | stays — real host capability | No |

## Binding Mechanics: `patina:connect` + `wasi:http`

This is the definitive contract for how the connect bridge works with WASI toys.

**`wasi:http/outgoing-handler`** takes an `outgoing-request` resource with scheme, authority, path-with-query, headers. There is no connection concept — the component specifies raw URLs.

**`patina:connect`** defines the credential-safe path and owns connection context:

```
Child code:
  1. connect::resolve("github") → opaque connection resource + base URL string
  2. Call connect::request(conn, method, path, headers, body)
     (host composes URL from connection metadata + path)

Mother's host:
  3. Resolve conn handle to endpoint, policy, and credential binding
  4. Inject credential headers host-side (never into WASM memory)
  5. Enforce allowlists/rate limits/audit policy
  6. Execute transport via wasi:http host implementation
  7. Return response to child
```

**Security rule:** credential injection is never based on URL prefix matching. Injection only occurs when a valid `connection` handle is provided. This avoids ambiguous overlap cases and keeps mediation explicit.

`base-url` is informational (logging/debug UX) and not an authorization primitive. Mother's routing and credential checks bind to the `connection` resource handle, not caller-supplied URL strings.

**Default security policy: deny raw http unless explicitly granted.** A child that declares `toys = ["http", "connect"]` with connections can ONLY use `connect::request(...)` for declared bindings. Raw `wasi:http` to arbitrary URLs is blocked unless the child explicitly declares `http.raw = true` in its toybox request. This makes the secure path (connect) the default and the open path (raw http) the opt-in exception.

```toml
# Default: connect-mediated http only (credentials managed by Mother, URL restricted)
[needs]
toys = ["http", "connect"]
[needs.connections]
github = { toy = "http" }
# Can ONLY use the "github" connection via connect::request. Raw wasi:http blocked.

# Explicit opt-in: raw http for public endpoints (no connect, no credential injection)
[needs]
toys = ["http"]
[needs.http]
raw = true
# Can reach arbitrary URLs. No credential injection. Audit-logged.
```

```toml
# Child that uses connect (credentials managed by Mother):
[needs]
toys = ["http", "connect"]
[needs.connections]
github = { toy = "http" }

# Child that uses raw http (public APIs only, no connect):
[needs]
toys = ["http"]
# No connections section — raw wasi:http only
```

**The same pattern extends to `patina:store`**: `connect::resolve("ducklake")` returns a handle. `store::query(conn, query)` and `store::mutate(conn, action, payload)` pass that handle. Mother routes to the correct backend (lake/belief/graph/query engine) from connection metadata, not from payload guessing.

## Resolved Decisions

- **WASI adoption policy for Phase 1**: Per-interface decision matrix. `wasi:http` and `wasi:filesystem` are stable — adopt now. `wasi:logging` and `wasi:keyvalue` are proposed — start as `patina:log` and `patina:state` with a sunset condition: migrate to `wasi:*` when the WASI interface reaches Phase 4 (standardized) and Wasmtime ships stable support. SDK feature flags (`toy-log`, `toy-state`) don't change during migration.
- **Default-deny raw http**: A child with `connect` + `http` can only use declared connection handles via `connect::request(...)`. Raw `wasi:http` to arbitrary URLs requires explicit `http.raw = true` opt-in. Secure path is the default.
- **No URL-prefix credential inference**: Host credential injection requires explicit `connection` resource usage (`connect::request` or handle-bearing store calls). Request URL matching alone is never sufficient.
- **Store routing is host-owned infrastructure**: Store backend selection is resolved by connection metadata bound at grant time, not by child payload conventions.
- **Connections and scopes coexist**: `[needs.connections]` declares named external bindings; `[needs.scopes]` continues to express stream/action/path limits. Both are merged into one grant object.
- **git is the 10th toy** — `patina:git`. Git operations (tag, commit, log, diff) require host-level execution that WASM cannot do alone. Not domain logic, not an SDK helper. Kept as a standalone Patina-specific toy. This is a closed question.
- **Toy litmus test**: "Why can't the child do this itself from pure WASM compute?" If it can, it's SDK/library, not a toy.
- **Credentials never cross the WASM wall.** Children operate through `patina:connect` opaque resource handles. Mother injects credentials on the host side.
- **Connections are named bindings**, like Cloudflare Workers' `wrangler.toml` bindings. The child says "github." Mother knows what that means.
- **Domain logic belongs in children and SDK helpers, not toys.** A `github::list_issues()` helper uses the `http` toy internally — it's library code, not a host interface.
- **git stays as a toy** (borderline call). Git operations are real host capabilities that WASM can't do alone. If WASI eventually proposes `wasi:git`, we align then.
- **WASI alignment is mixed adoption + shape-alignment.** We adopt `wasi:http` and `wasi:filesystem` directly now; we shape-align `patina:log`/`patina:state` to `wasi:logging`/`wasi:keyvalue` until runtime stability is proven.
- **The toybox is the sealed capability grant.** Mother assembles it from `child.toml`, resolves connections and credentials, and grants exactly what's declared. The manifest is the single source of truth for security review.

## Breaking Impact

This is a large breaking change. It touches every layer between WIT and child code. The risk is scope and stamina, not design uncertainty — each change has a clear before/after.

### What breaks

**WIT layer — full rewrite:**
- 22 toy `.wit` files replaced by 10 new interface designs (4 WASI-aligned: 2 adopted + 2 shimmed with sunset, plus 1 connect + 5 Patina-specific — not renames, new function signatures)
- `wit/worlds/` — every composed world regenerated for collapsed toy set
- All SDK `build.rs` files — different WIT sources to bind against

**Toy host — full rewrite of dispatch:**
- `src/child/toy_host/` — every file. Domain-specific hosts (github, lake, connector, belief, etc.) deleted. New hosts gain connection-aware credential injection and collection routing.
- Mother's child loading path — must parse `[needs.connections]`, resolve connection names to toy + credential + endpoint, build the toybox with connection metadata.

**SDK — full restructure:**
- 3 tier crates (`patina-sdk-core`, `patina-sdk-data`, `patina-sdk-agent`) absorbed into `patina-sdk`. Every `use patina_sdk_core::*` and `use patina_sdk_data::*` import breaks.
- All existing toy binding modules replaced with new bindings for 10 toys.
- New `helpers/` modules written for domain logic that moved out of toys (github, lake, session, connector).

**Every in-tree child — must migrate:**
- `ducklake` — `lake::append_json_batch()` + `github::*` → `connect::resolve("github")` + `connect::request(...)` + `store::mutate(conn, ...)` (or `sdk::helpers::*` wrappers)
- `belief-verifier` — `belief::query()` → `store::query(conn, ...)` with a resolved store connection handle
- `spec-manager` — `layer-fs` + `git` stay primitive (`fs` + `git` targets); `session::*` behavior moves to SDK/child logic where needed
- Same for `session-writer`, `doctor`, `lake-manager` (using each child's real current toy set from manifest snapshot above)

**Every `child.toml` — manifest schema change:**
- Old: child-specific legacy toy sets (for example ducklake uses `checkpoint`, `lake`, `github`, `measure`, `task`, `peer`)
- New: child-specific collapsed sets + `[needs.connections]` section (for example ducklake target: `connect`, `store`, `events`, `task`, `log`, `state`)
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

The phased execution order exists to make this survivable:
- Each phase leaves the workspace green (compiling and tests passing)
- Phase 0 locks protocol decisions before coding
- Phase 1 (WIT design) is pure design — nothing breaks until Phase 4 (new toy host)
- Phase 6 (child migration) proceeds one child at a time
- Phase 7 (SDK consolidation) can keep tier crates as thin re-exports during transition
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

## Agentic Security and Telemetry

The toybox model isn't just a simplification — it's a security architecture for autonomous agents. Children are persistent data-moving agents, not stateless request handlers. The WASM sandbox + Mother mediation gives us properties most agent frameworks lack.

### Identity and Trust

- Each child has an identity: name, version, manifest hash.
- Mother can verify a child's WASM binary hasn't been tampered with — hash the component at load time, compare to a known-good manifest.
- The toybox is a signed capability grant: Mother can prove "I authorized this child to have these toys at this time."
- For third-party children: Mother could require signed manifests before loading. Untrusted children get restricted toyboxes.

### Credentials Never Cross the Wall

The WASM boundary is a real isolation barrier, not a convention. A child physically cannot:
- Access the filesystem (unless `fs` toy granted)
- Make network calls (unless `http` toy granted)
- Read environment variables or secrets
- Access other children's memory
- Call syscalls

Credentials stay on Mother's side. When a child calls `connect::request(...)` with a granted connection handle, Mother injects the `Authorization` header host-side. The child never sees the PAT, never can exfiltrate it. If the child's WASM binary is compromised, the blast radius is limited to what's in its toybox.

### Telemetry as a Property of Mediation

Because Mother mediates every toy call, telemetry is automatic:
- Every toy call is an observable event — Mother can log, meter, trace without the child's cooperation.
- Mother can inject trace IDs into every call. When child A calls `peer` to child B, Mother propagates the trace context. Children don't manage their own tracing.
- Audit is free: "child X used http to reach github.com 47 times, queried store 'beliefs' 12 times, published 3 events in this session."
- This isn't a feature to build separately — it's a consequence of the architecture. There's no way to access anything without going through Mother's toy host.

### Agent-to-Agent Trust (Peer Toy)

The `peer` toy routes through Mother. Mother can enforce:
- Which children can call which other children
- Which actions are accessible (child A can call child B's `sync` but not `admin`)
- Rate limits on peer calls
- Mutual toybox compatibility requirements
- Explicit relationship declarations in `child.toml` `[relationships]`

A child calling another child via `peer` gets that child's API, not its toybox. This is capability delegation — same as Cloudflare Service Bindings.

### OWASP Top 10 for AI Agents Alignment

The toybox model directly addresses several OWASP agent threat categories:

| OWASP Threat | How Toybox Addresses It |
|---|---|
| **Excessive agency** | Children can only do what the toybox grants. No ambient authority. |
| **Insecure output** | Mother can inspect/filter every toy response before returning to the child. |
| **Denial of service** | Mother can throttle/revoke toys mid-session. Rate limits per toy per child. |
| **Overreliance** | The belief system has evidence and confidence levels, not blind assertions. |
| **Supply chain** | Third-party children run in the same sandbox. Signed manifests before loading. Restricted toyboxes for untrusted sources. |

### What This Requires in the Toy Host

The connection-aware toy host (Phase 4) must implement:
1. **Credential injection** — resolve connection names to auth headers/tokens on every call
2. **Audit logging** — emit structured events for every toy call (child, toy, connection, timestamp, result)
3. **Rate limiting** — per-toy, per-connection, per-child
4. **Trace propagation** — inject/propagate trace context across toy and peer calls
5. **Policy enforcement** — check toybox grants before dispatching any call

These are not optional add-ons. They're the reason Mother mediates instead of passing through.

## WASI and Cloudflare Alignment Lock

### Where WASI vs Patina locks in

In the WIT `package` declaration per toy file. **`http` and `fs` are adopted directly from WASI; `log` and `state` are Patina shims aligned to WASI shapes with sunset migration.** `patina:connect` is the bridge that adds credential security on top.

| Toy | Phase 1 Package | Target Package | Layer | Status |
|-----|----------------|----------------|-------|--------|
| http | `wasi:http` | `wasi:http` | WASI adopted | Embrace as-is |
| fs | `wasi:filesystem` | `wasi:filesystem` | WASI adopted | Embrace as-is |
| log | `patina:log` | `wasi:logging` | WASI shimmed | Patina shim tracking WASI shape; migrate when stable |
| state | `patina:state` | `wasi:keyvalue` | WASI shimmed | Patina shim tracking WASI shape; migrate when stable |
| connect | `patina:connect` | `patina:connect` | Patina bridge | Credential-security layer over WASI primitives |
| store | `patina:store` | `patina:store` | Patina expansion | Expand where WASI has gaps; candidate for future proposal |
| events | `patina:events` | `patina:events` | Patina expansion | Expand where WASI has gaps; candidate for future proposal |
| task | `patina:task` | `patina:task` | Patina expansion | Expand where WASI has gaps; candidate for future proposal |
| peer | `patina:peer` | `patina:peer` | Patina expansion | No WASI equivalent |
| git | `patina:git` | `patina:git` | Patina expansion | No WASI equivalent |

SDK feature flags don't change per package — `toy-http` is `toy-http` regardless of whether the WIT says `wasi:http` or `patina:http`.

### Where Cloudflare shape locks in

1. **`child.toml` schema** — manifest-as-security-boundary. Read it, know everything the child can do. Like `wrangler.toml`.
2. **Mother's mediation contract** — credentials host-side, every call mediated, connections are opaque handles. Like the Workers runtime.
3. **Peer calls** — child-to-child without crossing capability boundaries. Like Service Bindings.

### Portability: enabled, not required

Not all children should be portable. Most children will be Patina-native — using `store`, `events`, `task`, `peer`, `git`, toys that have no Cloudflare equivalent. That's the whole point of the Patina-specific toys.

But because the primitive layer embraces WASI and the design model aligns with Cloudflare's binding approach, you *can* build a child that only uses the portable subset (`http`, `log`, `state`, `fs`) and that child *will* run as a Cloudflare Worker with an adapter layer. The architecture enables this capability without requiring it.

The `cloudflare-worker-child` spec (blocked on this spec) will prove this: one WASM component, same code, running under both Mother and Cloudflare. If it works, the toy abstraction is correct and the portable/Patina-specific boundary is drawn in the right place. If it doesn't, we learn where the abstraction leaks.

## Locked Vocabulary

Five terms. Everything else is absorbed or retired.

| Term | What it means | Maps to (Cloudflare) | Maps to (WASI) |
|------|--------------|---------------------|----------------|
| **Toy** | A primitive capability Mother grants. A door in the WASM sandbox wall. 10 total: 4 WASI-aligned (2 adopted, 2 shimmed), 1 bridge, 5 Patina-specific. | Binding | Import |
| **Toybox** | The sealed capability payload Mother assembles for a child at init. Not just a list of toy names — resolved connection handles with credentials, endpoints, scopes, rate limits, and policy attached. The toybox IS the security contract, the audit surface, and the portability boundary. | `env` object | Component's import set |
| **Kind** | Child lifecycle shape. How Mother manages you. | Worker type | World |
| **Child** | WASM worker with bounded agency. | Worker | Component |
| **Mother** | Orchestrator. Builds toyboxes, mediates calls, holds credentials. | Workers Runtime | Host |

`child.toml` = `wrangler.toml` = component manifest. The single source of truth for what a child can do.

Connection, binding, scope, world, grant, capability — all absorbed into the five terms or retired from user-facing vocabulary.

## Verification

```bash
cargo check --workspace -q
cargo test -q
# Verify exactly 10 toy WIT files:
ls wit/toys/*.wit | wc -l  # should be 10
# Verify no domain-specific types in toy interfaces:
rg "issue|pull-request|review|granted-lake|repo-binding" wit/toys/  # should be 0
# Verify children build:
for child in children/*/; do cargo check -p $(basename $child) 2>/dev/null; done
# Verify SDK is one crate:
grep 'patina-sdk-core\|patina-sdk-data\|patina-sdk-agent' Cargo.toml  # should be 0 in workspace members
```

## Build Readiness

Phase 0 is complete (protocol lock + feasibility snapshot captured). Phase 1 (WIT design) can begin.

## Relationship to Other Specs

- **`wit-contract-single-source`** — **abandoned**. Absorbed by this spec. When we write 10 new WIT files, we do them right from the start (single source, no copies). Archived at `66ab254f`.
- **`greenfield-crate-extraction`** — **blocked on this spec**. The engine crate's toy host shape depends on the collapsed toy interfaces. Blocker updated from `wit-contract-single-source` to `toy-collapse-wasi-alignment`.
- **`cloudflare-worker-child`** — **blocked on this spec**. Proves the toy collapse drew the right lines: one WASM component, same code, running under both Mother and Cloudflare Workers. The portable subset (`http`, `log`, `state`, `fs`) only exists after collapse. This is the validation that the architecture enables portable children without requiring all children to be portable.
