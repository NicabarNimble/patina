# Design: Pando Vocabulary Alignment

## Principle Alignment

- [[pando-is-composed-children]] — pando is a word for a group of children
- [[five-boundaries-no-overlap]] — five boundaries; pando is composition within child+toy
- [[children-are-wasm]] — all children are WASM; no species split
- [[children-have-agency-toys-are-capabilities]] — toybox determines behavior, not kind
- [[world-boundary-is-type-safety]] — world boundaries provide compile-time isolation;
  pipeline/knowledge-child was a false seam. The principle is sound; the application
  was wrong. Future worlds emerge from evidence.
- [[wasi-is-foundation-not-option]] — one world built on WASI foundation
- [[core-primitives-are-not-children]] — grammar parsers are scrape strategy children

## Phase 1: Vocabulary (docs only)

Phase 1 is truth alignment. Docs reflect intended vocabulary. No code changes.

### pva1: child-construction-canon

Changes to `layer/surface/build/feat/child-construction-canon/SPEC.md`:
- "## Objective Recipe Format" → "## Pando Format"
- "Every objective recipe defines:" → "Every pando defines:"
- `objective_id:` YAML key → `pando:`
- ccc7 gate: "using the recipe format" → "using the pando format"
- "composition" subsection stays (describes wiring within a pando)

No parser enforcement — pando YAML parser doesn't exist yet. Parser behavior
is a future spec (runtime pando execution).

### pva2: AGENTS.md

Add vocabulary to project-specific notes: Patina, Mother, child, toy, pando,
project. Remove stale SDK tier and WIT directory references.

Add compatibility note: "Note: code still uses knowledge-child/pipeline until
Phase 2 of pando-vocabulary-alignment. wit/ directories will become wit/child/."
Removed in Phase 2 pva7 commit.

### pva3: Beliefs

- Add `[[pando-is-composed-children]]` to child-construction-canon `beliefs:` list
- Verify `[[five-boundaries-no-overlap]]` references pando

### pva4: SDK docs

- sdk/patina-sdk/README.md: composition examples use "pando" terminology
- Drop "knowledge-child" vs "pipeline" distinction in user-facing docs
- One SDK, one child kind, toys determine behavior

**Phase 1 verification:** docs match intended vocabulary. No compile checks.

## Phase 2: SDK-Based Kind Collapse and World Unification (code)

All children use the SDK. Legacy children (not using SDK) are temporary
compatibility artifacts, inventoried and slated for migration.

### pva5: One WIT World

Merge `wit/pipeline/pipeline.wit` into `wit/knowledge-child/knowledge-child.wit`.
Rename directory to `wit/child/`. Package becomes `patina:child@0.1.0`.

**handle() signature (AF1):**
- pipeline: `export handle: func(request: string) -> result<string, string>;`
- knowledge-child: `export handle: func(action: string, payload: string) -> result<string, string>;`

Unified world uses the knowledge-child signature (action + payload). This is
the richer contract. All SDK knowledge children already implement it.

Grammar plugins require recompilation (pva8). SDK provides a temporary shim:
`PipelineChild` trait adapts `handle(request)` to `handle(action, payload)` by
passing payload as request. This shim is migration triage — it keeps legacy
grammar plugins compiling during transition, not permanent support.

The unified world keeps all current knowledge-child exports. SDK provides
default stubs:

```rust
// In patina-sdk, provided automatically unless the child overrides:
fn health() -> ChildHealth { ChildHealth { status: Healthy, reason: None } }
fn tick() -> Vec<TaskIntent> { vec![] }
fn drain(_limit: u32) -> Result<Vec<PendingEvent>, String> { Ok(vec![]) }
fn on_load() -> Result<(), String> { Ok(()) }
fn on_unload() {}
```

A child that only implements `init`, `name`, `handle` gets stubs for free.
Same as today's pipeline behavior, compiled against the unified world.

**World boundaries going forward:** This merge removes a false seam, not the
principle. If a real seam appears from building more children, a new world
earns its existence then.

### pva6: One Engine

Merge `PipelineEngine` (`src/child/internal/pipeline.rs`, 276 lines) and
`KnowledgeChildEngine` (`src/child/internal/knowledge_child.rs`, 1109 lines)
into `ChildEngine`.

**Read both files fully before merging.**

The merged engine:
- Links all WIT imports (WASI + Patina toys)
- Gates capability at the call boundary via `GrantedCapabilities` from
  `[needs].toys` in child.toml
- Calls lifecycle exports (`tick`, `drain`, `health`) — stubs return no-ops
  for simple children
- AOT caching from PipelineEngine carries over

**Capability enforcement (AF2):**
Today PipelineEngine links only WASI + host_log. KnowledgeChildEngine links
~15 interfaces but gates non-granted toys at the call boundary (host functions
return error for non-granted toys). The merged engine links ALL interfaces
(Wasmtime requires imports to be satisfied at link time) but gates access at
the call boundary — the same pattern KnowledgeChildEngine already uses.

**Required test (pva-cap-test):** Add a test that loads a child with
`toys = ["log"]` and asserts that invoking a state/layer-fs/git host function
returns an error. This must be in `cargo test`.

### pva7: ChildKind Collapsed

```rust
// Before:
pub enum ChildKind {
    KnowledgeChild,
    Pipeline,
}

// After:
pub enum ChildKind {
    Child,
}
```

`child.toml` changes:
```toml
# Before:
kind = "knowledge-child"
# or
kind = "pipeline"

# After (required):
kind = "child"
```

`kind` field is required, not optional. `FromStr` behavior:
- `"child"` → `Child` (canonical)
- `"knowledge-child"` → `Child` (silent alias, removed next minor)
- `"pipeline"` → `Child` (silent alias, removed next minor)
- `"command"` / `"task"` → error with migration message (already exists)

Remove Phase 1 compatibility note from AGENTS.md. Update wit/ path references
to final state.

### pva8: Grammar Plugin Recompilation

9 grammar plugins in `~/.patina/pipeline/` need recompilation against unified
world via SDK. `patina setup grammars` installs updated binaries.

Plugins change: `handle(request)` signature becomes `handle(action, payload)`.
SDK `PipelineChild` shim handles this temporarily. Grammar plugins should
migrate fully to the unified `Child` trait by next minor release.

Path `~/.patina/pipeline/` stays — it's a storage location, not a kind label.

### pva9: SDK Unification

```toml
# Before (patina-sdk Cargo.toml):
[features]
knowledge-child = []
pipeline = []

# After:
[features]
child = []
pipeline = ["child"]           # temporary alias, removed next minor
knowledge-child = ["child"]    # temporary alias, removed next minor
```

`src/lib.rs` compile-error guard simplified — no more mutual exclusion check.
One world, one feature.

**SDK macro/trait bridge (AF6) — temporary triage:**
`sdk/patina-sdk/src/pipeline.rs` exports `PipelineChild` trait (line 136) and
`register_pipeline_child!` macro (line 219). Legacy grammar plugins use these.

Temporary migration plan:
- Keep `pipeline.rs` file, mark module `#[deprecated(note = "Use Child trait — legacy, removed next minor")]`
- `PipelineChild` trait adapts `handle(request)` to `handle(action, payload)`
- `register_pipeline_child!` macro re-exports to unified child registration
- **Removed in next minor release.** These are not permanent compatibility.

### pva10: Children Updated + Legacy Inventory

All 14 child.toml files (13 children + template):
- `kind = "knowledge-child"` → `kind = "child"`
- Cargo.toml `metadata.component.target.path`: `wit/knowledge-child/` → `wit/child/`

**Legacy inventory (pva-legacy-inventory):** Identify any children not using
patina-sdk. For each legacy child found:
1. Document which child and what bindings it uses
2. Create an explicit migration follow-up (issue or spec)
3. Mark the shim dependency so it's tracked for removal

**Hard rule: no new legacy children.** `patina child init` produces SDK
children only. The template uses the SDK. No legacy world option exposed.

### pva11: Template Updated

`children/template/` and `patina child init` scaffold:
- child.toml: `kind = "child"`
- Cargo.toml: target path `wit/child/`, depends on patina-sdk with `child` feature
- CLI help: `--world` options updated (no legacy options)

## Risks

- **handle() signature (AF1)** — real migration risk. SDK shim handles it
  temporarily for legacy grammar plugins. Legacy children inventoried.
- **Capability widening (AF2)** — merged engine must gate at call boundary.
  Required cargo test validates this.
- **Grammar recompilation** — users need `patina setup grammars --force` after
  upgrade. Add version check in grammar discovery to detect old-world plugins.
- **Backward compat** — silent aliases in FromStr and SDK features. Temporary.
  Removed next minor.
- **Engine merge complexity** — 1109 + 276 lines. Read both fully. Linker
  setup is the tricky part (knowledge-child links ~15 interfaces, pipeline
  links 1).
- **Doc-code gap (AF7)** — Phase 1 compatibility note bridges it.

## Verification

**Phase 1:** Truth-alignment checks. Docs match intended vocabulary. No
compile checks needed.

**Phase 2:**

```bash
# Compile proof
cargo check --workspace -q
cargo test -q --lib

# Backward compat (aliases)
# Verify FromStr accepts "knowledge-child" and "pipeline" silently

# Runtime proof
patina child run doctor health          # unified engine
patina scrape code                      # grammar plugins

# Capability enforcement (required cargo test)
cargo test -- capability_gate

# Template proof
# patina child init produces SDK child with kind = "child" and wit/child/ target

# Legacy inventory
# List any children not using patina-sdk
# Migration follow-ups created for each
```

## Phase 3: CI Regression Guards

### pva12: Update pre-push world references

`resources/git/pre-push-checks.sh:31`: `SDK_WORLDS=(knowledge-child pipeline)`
→ `SDK_WORLDS=(child)`. WIT consistency and mirror checks use `wit/child/` paths.

### pva13 + pva14: Already done (AF8)

Both guards already exist in `resources/scripts/check-runtime-boundaries.sh`
(lines 92-151). Exit criteria checked off.

## Not in Scope

- History rewriting
- Runtime pando execution (`patina pando run` — future spec)
- Per-child WIT world generation from manifest (future spec)
- Pando YAML parser enforcement (future spec — parser doesn't exist)
- Broad legacy/third-party compatibility guarantees
