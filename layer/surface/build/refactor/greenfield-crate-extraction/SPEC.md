---
type: refactor
id: greenfield-crate-extraction
status: draft
created: 2026-03-26
blocked_by:
  - wit-contract-single-source
sessions:
  origin: 20260325-150227-161735000
beliefs:
  - "[[root-communicates-identity]]"
  - "[[four-roles-no-overlap]]"
  - "[[core-verbs-standalone-mother-additive]]"
  - "[[children-have-agency-toys-are-capabilities]]"
related:
  - src/
  - src/child/
  - src/child/internal/
  - src/child/toy_host/
  - src/db/
  - src/embeddings/
  - src/connect/
  - src/commands/
  - crates/patina-core/
  - crates/patina-protocol/
  - mother/
exit_criteria:
  - id: gce1-engine-crate
    text: "WASM child runtime (`src/child/internal/`, `src/child/toy_host/`) extracted into a standalone `crates/patina-engine/` crate. Engine is testable without CLI or Mother."
    checked: false
  - id: gce2-store-crate
    text: "Persistence layer (`src/db/`, `src/layer/`, `src/embeddings/`) extracted into `crates/patina-store/` (or split further). Store is usable by both CLI and Mother without circular deps."
    checked: false
  - id: gce3-connect-crate
    text: "LLM provider adapters (`src/connect/`, `src/models/`) extracted into `crates/patina-connect/`. Provider SDK types do not leak into core or CLI surfaces."
    checked: false
  - id: gce4-cli-thin
    text: "`src/` (the root crate) is a thin CLI adapter: arg parsing, subcommand dispatch, output formatting. Business logic lives in engine/store/connect/core/protocol crates."
    checked: false
  - id: gce5-dep-direction
    text: "Dependency graph enforced: cli → {engine, mother, store, connect, core, protocol}. mother → {engine, store, core, protocol}. engine → {core, protocol}. store → {core}. connect → {core, protocol}. core → nothing. protocol → nothing. No cycles."
    checked: false
  - id: gce6-parity
    text: "All existing tests pass. `cargo test --workspace -q` succeeds. CLI behavior is identical pre/post extraction."
    checked: false
---
# refactor: Greenfield crate extraction

> Extract the src/ monolith into focused crates with enforced dependency direction. Make each architectural role independently testable and buildable.

## Problem

`src/` is a single crate containing the CLI binary, WASM child runtime, database layer, embedding engine, LLM connectors, session manager, git integration, secret store, retrieval system, and a Mother client. Everything depends on everything. Changing one module recompiles the entire binary. The engine can't be tested without the CLI. Mother can't be tested without the engine. The dependency direction between architectural roles is implicit and unenforced.

`patina-core` and `patina-protocol` exist but are mostly empty — the domain logic they should hold is still trapped in `src/`.

## Goal

Each architectural role lives in its own crate with explicit, enforced dependencies. The compiler prevents circular deps. Each crate is independently testable. Build times improve through parallelism and incremental compilation.

## Non-Goals

- Do NOT extract children to separate repos (that's a later effort).
- Do NOT change the SDK or WIT interfaces (handled by `wit-contract-single-source`).
- Do NOT add new features — this is a pure structural refactor.
- Do NOT rewrite algorithms or business logic — move, don't modify.
- Do NOT change CLI behavior or user-facing output.

## Current State

```
src/                          # One crate: patina-ai (binary + library)
├── commands/                 # 22 CLI subcommand modules
├── child/internal/           # WASM runtime (wasmtime, component loading)
├── child/toy_host/           # Toy grant dispatch (log, state, github, etc.)
├── connect/                  # LLM provider adapters
├── db/                       # SQLite persistence (events.db, patina.db)
├── embeddings/               # Vector store + ONNX inference
├── layer/                    # Knowledge layer read/write
├── session/                  # Session lifecycle
├── beliefs/                  # Belief system
├── mother/                   # Mother client (broker, doctor_runtime)
├── git/                      # Git integration
├── project/                  # Project detection, config
├── secrets/                  # Secret management
├── retrieval/                # Search/query layer
├── scanner/                  # Code scanning
├── workspace/                # Workspace detection
├── models/                   # Model management
└── ...

crates/
├── patina-core/              # Nearly empty (doctor.rs, lake.rs)
└── patina-protocol/          # Wire types (BuiltinChild*, typed requests)

mother/                       # Daemon crate (services, broker, HTTP API)
```

## Target State

```
cli/                          # Thin binary: args, dispatch, output
  or src/                     # (same crate name, just thinner)

crates/
├── patina-core/              # Domain invariants, types, validation (zero IO deps)
├── patina-protocol/          # Wire types, serialization
├── patina-engine/            # WASM runtime + toy host dispatch
├── patina-store/             # DB + layer + embeddings persistence
└── patina-connect/           # LLM provider adapters

mother/                       # Daemon: services, broker, HTTP API
```

## Target Dependency Graph

```
cli → engine, mother, store, connect, core, protocol
mother → engine, store, core, protocol
engine → core, protocol
store → core
connect → core, protocol
core → (nothing)
protocol → (nothing)
```

No cycles. The compiler enforces this — Cargo won't build a workspace with circular deps.

## Solution

### Phase 1: Engine Extraction (highest value)

Extract `src/child/internal/` and `src/child/toy_host/` into `crates/patina-engine/`.

This is the biggest win: the WASM runtime becomes independently testable. Load a child, grant toys, run it — no CLI, no daemon.

**Seam analysis needed:** What does `child/internal/` currently import from `src/`? Each import is a dependency that must be satisfied via `core`, `protocol`, or a new trait boundary.

**Approach:**
1. Create thin `crates/patina-engine/` crate with re-exports
2. Move one module at a time, starting with `toy_host/` (fewer deps)
3. Define trait boundaries where engine needs IO (e.g., layer reads, event writes)
4. `src/child/` becomes a thin adapter that wires engine traits to concrete implementations

### Phase 2: Store Extraction

Extract `src/db/`, `src/layer/`, `src/embeddings/` into `crates/patina-store/`.

**Caution (from builder agent):** embeddings + DB + layer IO may be too broad for one crate. May need `patina-store-db` vs `patina-store-search` split. Start as one, split if the boundary is obvious.

**Session lifecycle caution:** Session management may split between Mother (lifecycle) and store (artifact persistence). Don't move session blindly — map the seam first.

### Phase 3: Connect Extraction

Extract `src/connect/` and `src/models/` into `crates/patina-connect/`.

**Rule:** Provider SDK types (e.g., `reqwest::Response`, OpenAI types) must not leak into core or CLI surfaces. Define trait boundaries in core, implement in connect.

### Phase 4: CLI Thinning

What remains in `src/` after extraction is the CLI adapter: arg parsing, subcommand dispatch, output formatting, and wiring the extracted crates together.

**This phase is emergent** — it's what's left after Phases 1-3, not a separate move operation.

## Implementation Order

Phases are sequential. Each phase must leave the workspace green.

Each phase follows the same pattern (per builder agent advice):
1. Create thin crate with re-exports/adapters
2. Move one vertical feature path at a time (e.g., doctor path, then scrape path)
3. Add compile-time dependency guards
4. Only after parity, delete old `src/` paths

## Resolved Decisions

- `patina-core` and `patina-protocol` already exist. Engine/store/connect are new crates alongside them in `crates/`.
- The root crate (`patina-ai`) stays as the CLI binary. It just gets thinner.
- Mother daemon stays in `mother/`. It gains deps on engine and store.
- Children and SDK are untouched — they depend on SDK, not on internal crates.
- Anti-cycle enforcement is free: Cargo rejects circular workspace deps at compile time.

## Verification

After each phase:
```bash
cargo check --workspace -q
cargo test -q
cargo run -q -- --help       # CLI still works
cargo run -q -- doctor --json # Feature path still works
```

After all phases:
```bash
# Verify dependency direction:
cargo tree -p patina-engine | grep -v patina-core | grep -v patina-protocol | grep patina-  # should be empty
cargo tree -p patina-store | grep -v patina-core | grep patina-  # should be empty
cargo tree -p patina-connect | grep -v patina-core | grep -v patina-protocol | grep patina-  # should be empty
```

## Build Readiness

Blocked on `wit-contract-single-source`. Engine extraction touches `src/child/` which is a WIT consumer — WIT source of truth must be clean before moving WIT-dependent code between crates.

Phase 1 requires seam analysis of `src/child/internal/` imports before execution. This is design work, not investigation — the spec is ready, the design doc needs the import map.
