# Design: Greenfield crate extraction

## Why This Design

The `src/` monolith violates `[[four-roles-no-overlap]]` at the crate boundary level. Core, engine, store, and connect are conceptually separate roles but compile as one unit. Extracting them makes the dependency direction explicit and compiler-enforced, improves build parallelism, and makes each role independently testable.

The builder agent's key insight: "this is a migration architecture spec, not a move files spec." The hard part is defining trait boundaries at seams, not `git mv`.

## Build Target

Four phases over multiple sessions. Each phase leaves the workspace green. Total new crates: 3 (engine, store, connect).

## Resolved Decisions

- Move one vertical feature path at a time, not one module at a time. E.g., "make the doctor path work through engine" before "move all of toy_host."
- Create thin adapter crates first with re-exports, then migrate internals. This avoids big-bang breakage.
- Session lifecycle splits: lifecycle management → Mother service. Artifact persistence → store. Map this seam during Phase 2.
- Store may split later (db vs search). Start as one crate, split if boundary is obvious.

## Phase 1: Engine Extraction

### Pre-work: Import Map

Before moving code, map what `src/child/internal/` imports from `src/`:

```
src/child/internal/mod.rs        → imports from: ???
src/child/internal/command.rs    → imports from: ???
src/child/internal/task.rs       → imports from: ???
src/child/internal/pipeline.rs   → imports from: ???
src/child/internal/knowledge_child.rs → imports from: ???
src/child/internal/host_support.rs → imports from: ???
src/child/toy_host/*.rs          → imports from: ???
```

Each import falls into one of:
- **Already in core/protocol** → direct dep, clean
- **Pure types/validation** → move to core first, then dep
- **IO operation** → define trait in core, impl in adapter

### Commits (Phase 1)

1. `refactor(core): move engine-needed types into patina-core` — Move types that engine needs but that currently live in `src/`. This grows core before engine exists.

2. `feat(engine): scaffold patina-engine crate with re-exports` — Create `crates/patina-engine/` with `Cargo.toml`. Initially re-exports from `src/child/` so the binary still works. Tests verify the re-export path.

3. `refactor(engine): migrate toy_host into engine crate` — Move `src/child/toy_host/` → `crates/patina-engine/src/toy_host/`. Define IO traits where toy_host needs layer/event access.

4. `refactor(engine): migrate child runtime into engine crate` — Move `src/child/internal/` → `crates/patina-engine/src/runtime/`. Wire trait implementations from CLI side.

5. `test(engine): verify engine is independently testable` — Write a test that loads a WASM child, grants toys, runs it — using only engine + core + protocol deps.

### Key Files (Phase 1)

- `crates/patina-engine/Cargo.toml` — new
- `crates/patina-engine/src/lib.rs` — new
- `src/child/` — becomes thin adapter after migration
- `crates/patina-core/src/` — grows with engine-needed types

## Phase 2: Store Extraction

### Commits (Phase 2)

1. `feat(store): scaffold patina-store crate` — Create `crates/patina-store/` with re-exports.
2. `refactor(store): migrate db module` — Move `src/db/` → `crates/patina-store/src/db/`.
3. `refactor(store): migrate layer IO` — Move `src/layer/` → `crates/patina-store/src/layer/`.
4. `refactor(store): migrate embeddings` — Move `src/embeddings/` → `crates/patina-store/src/embeddings/`. Evaluate if this should be separate crate.

### Key Files (Phase 2)

- `crates/patina-store/Cargo.toml` — new
- `src/db/`, `src/layer/`, `src/embeddings/` — become adapters or deleted

## Phase 3: Connect Extraction

### Commits (Phase 3)

1. `feat(connect): scaffold patina-connect crate` — Create `crates/patina-connect/`.
2. `refactor(connect): migrate provider adapters` — Move `src/connect/` → `crates/patina-connect/src/`.
3. `refactor(core): define LLM trait boundary in core` — Provider-specific types stay in connect. Core defines traits (`trait LlmProvider`). CLI and Mother depend on trait, not implementation.

## Phase 4: CLI Thinning (Emergent)

No explicit commits planned — this is what's left in `src/` after Phases 1-3. Verify that `src/` only contains:
- `main.rs` — binary entrypoint
- `commands/` — subcommand dispatch
- Adapter wiring (connecting crates together)

## Dependency Guards

After all phases, verify with `cargo tree`:
```bash
# engine should only depend on core + protocol (plus external deps)
cargo tree -p patina-engine --depth 1 | grep 'patina-'
# Expected: patina-core, patina-protocol

# store should only depend on core
cargo tree -p patina-store --depth 1 | grep 'patina-'
# Expected: patina-core

# connect should only depend on core + protocol
cargo tree -p patina-connect --depth 1 | grep 'patina-'
# Expected: patina-core, patina-protocol
```

## Verification Plan

After each phase:
```bash
cargo check --workspace -q
cargo test --workspace -q
cargo run -q -- doctor --json
cargo run -q -- child list
```

## Build Readiness

Blocked on `wit-contract-single-source`. Phase 1 additionally requires the import map (pre-work above) to be filled in by reading `src/child/internal/` imports.

## Open Questions

- Should `src/session/` go to store or stay as a Mother service concern? Needs seam analysis.
- Should `src/git/`, `src/project/`, `src/workspace/` go to core or store? They mix types (core) with IO (store).
- Should engine depend on store (for toy implementations that need DB access)? If yes, that adds a store → engine path that complicates the graph. Alternative: engine defines trait, CLI/Mother provides impl.
- How to handle `src/mother/` (client-side code in the binary) vs `mother/` (daemon crate)? May need a `patina-mother-client` or just keep it in CLI.
