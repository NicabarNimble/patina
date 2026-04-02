# Design: Pando Vocabulary Alignment

## Principle Alignment

- [[pando-is-composed-children]] — the naming this spec implements
- [[five-boundaries-no-overlap]] — pando fills the composition gap; kind unification cleans child+toy boundary
- [[wasi-is-foundation-not-option]] — one world means all children build on the same WASI foundation
- [[children-have-agency-toys-are-capabilities]] — the toybox determines behavior, not the kind label

## Phase 1: Vocabulary (docs only)

### pva1: child-construction-canon

Changes to `layer/surface/build/feat/child-construction-canon/SPEC.md`:
- "## Objective Recipe Format" → "## Pando Format"
- "Every objective recipe defines:" → "Every pando defines:"
- `objective_id:` YAML key → `pando:`
- ccc7 gate: "using the recipe format" → "using the pando format"
- "composition" subsection in pando YAML stays (describes the wiring within a pando)

### pva2: AGENTS.md

Add vocabulary table to project-specific notes. Remove stale SDK tier and WIT directory references (overlaps with car-cleanup-non-a but vocab is the driver here).

### pva3: Beliefs

- Add `[[pando-is-composed-children]]` to child-construction-canon `beliefs:` list
- Verify [[five-boundaries-no-overlap]] references pando (already done this session)

### pva4: SDK docs

- sdk/patina-sdk/README.md: composition examples use "pando" terminology
- Drop "knowledge-child" vs "pipeline" distinction in user-facing docs
- One SDK, one child kind, toys determine behavior

## Phase 2: Kind Unification (code)

### pva5: One WIT World

Merge `wit/pipeline/pipeline.wit` into `wit/knowledge-child/knowledge-child.wit`. Rename directory to `wit/child/`.

**CRITICAL — handle() signature mismatch (AF1):**
- pipeline: `export handle: func(request: string) -> result<string, string>;`
- knowledge-child: `export handle: func(action: string, payload: string) -> result<string, string>;`

The unified world uses the knowledge-child signature (action + payload). This is
the richer contract and all knowledge children already implement it.

For existing grammar plugins (compiled against pipeline world), recompilation is
required (pva8). The SDK provides a compatibility adapter: when a former-pipeline
child only cares about the request body, it implements
`handle(action: &str, payload: &str)` and ignores the action parameter. The
migration is a one-line signature change per plugin.

The unified world keeps all current knowledge-child exports. SDK provides default stubs:

```rust
// In patina-sdk, provided automatically unless the child overrides:
fn health() -> ChildHealth { ChildHealth { status: Healthy, reason: None } }
fn tick() -> Vec<TaskIntent> { vec![] }
fn drain(_limit: u32) -> Result<Vec<PendingEvent>, String> { Ok(vec![]) }
fn on_load() -> Result<(), String> { Ok(()) }
fn on_unload() {}
```

A child that only implements `init`, `name`, `handle` gets the stubs for free. Same as today's pipeline behavior, but compiled against the unified world.

### pva6: One Engine

Merge `PipelineEngine` (`src/child/internal/pipeline.rs`, 276 lines) and
`KnowledgeChildEngine` (`src/child/internal/knowledge_child.rs`, 1109 lines)
into `ChildEngine`.

**Read both files fully before merging.**

The merged engine:
- Links all WIT imports (WASI + Patina toys)
- Checks `[needs].toys` at load time and only activates granted toys
- Calls lifecycle exports (`tick`, `drain`, `health`) — stubs return no-ops for simple children
- AOT caching from PipelineEngine carries over

**CRITICAL — Capability enforcement (AF2):**
Today PipelineEngine links only WASI + host_log. KnowledgeChildEngine links ~15
interfaces but checks grants. The merged engine must NOT widen access for
former-pipeline children. After merge, verify: a child with `toys = ["log"]`
gets ONLY log. The grant check in KnowledgeChildEngine is the pattern to follow —
link all interfaces, but gate activation on `[needs].toys` from child.toml.

### pva7: ChildKind Collapsed

```rust
// Before:
pub enum ChildKind {
    KnowledgeChild,
    Pipeline,
}

// After:
// No enum needed. All children are children.
// If we keep the field for forward compat:
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

# After:
kind = "child"
```

`FromStr` keeps retired kind error messages:
- `"knowledge-child"` → accepted, maps to `Child` (backward compat)
- `"pipeline"` → accepted, maps to `Child` (backward compat)
- `"command"` / `"task"` → error with migration message (already exists)

### pva8: Grammar Plugin Recompilation

9 grammar plugins in `~/.patina/pipeline/` need recompilation against unified world. `patina setup grammars` installs updated binaries. The plugins themselves barely change — they just compile against a world with more exports (all stubbed by SDK).

### pva9: SDK Unification

```toml
# Before (patina-sdk Cargo.toml):
[features]
knowledge-child = []
pipeline = []

# After:
[features]
child = []
```

`src/lib.rs` compile-error guard simplified — no more mutual exclusion check. One world, one feature.

### pva10-11: Children and Template

All 13 children + template: `kind = "knowledge-child"` → `kind = "child"` in child.toml. Cargo.toml metadata target path: `wit/knowledge-child/` → `wit/child/`.

## Risks

- **handle() signature mismatch (AF1)** — the real migration risk. Pipeline uses
  `handle(request)`, knowledge-child uses `handle(action, payload)`. Unified world
  uses the richer signature. Grammar plugins must be recompiled. SDK adapter makes
  migration a one-line change per plugin.
- **Capability widening (AF2)** — pipeline children get only host_log today. Merged
  engine must enforce `[needs].toys` to prevent silent capability escalation. Verify
  after merge that `toys = ["log"]` children don't get state/layer-fs/git.
- **Grammar plugin recompilation** — users with installed grammars need `patina setup
  grammars --force` after upgrade. Add a version check in grammar discovery that
  detects old-world plugins and prompts. Plugins live in `~/.patina/pipeline/` — this
  path stays (AF5), only the WASM world changes.
- **Backward compat** — `kind = "knowledge-child"` and `kind = "pipeline"` accepted
  silently as aliases in `FromStr`, mapped to `Child` (AF3). No warning, no error.
- **Engine merge complexity** — KnowledgeChildEngine is 1109 lines, PipelineEngine is
  276 lines. Read both fully before merging. The merge needs care around linker setup
  (knowledge-child links ~15 interfaces, pipeline links 1). Unified engine links all
  interfaces but checks grants.
- **Phase 1/2 doc-code gap (AF7)** — Phase 1 docs describe pando/unified model before
  Phase 2 code exists. Compatibility notes in AGENTS.md bridge the gap, removed in Phase 2.

## Expanded Verification (AF9)

After Phase 2 complete, in addition to `cargo check` and `cargo test --lib`:

```bash
# Backward compat: old kind values still load
# (verify FromStr accepts "knowledge-child" and "pipeline")

# Runtime proof
patina child run doctor health          # unified engine runs doctor
patina scrape code                      # grammar plugins via unified engine

# Capability enforcement
# Verify a toys = ["log"] child does NOT get state/layer-fs/git access
# (inspect engine linking logs with PATINA_LOG=1, or unit test)

# Template proof
# patina child init produces valid child.toml with kind = "child"
```

## Phase 3: CI Regression Guards

### pva12: Update pre-push world references

`resources/git/pre-push-checks.sh:31` has `SDK_WORLDS=(knowledge-child pipeline)`. After kind unification this becomes `SDK_WORLDS=(child)`. Steps 1+2 (WIT consistency + mirror completeness) update their paths from `wit/knowledge-child/` and `wit/pipeline/` to `wit/child/`.

### pva13 + pva14: Already done (AF8)

Both guards already exist in `resources/scripts/check-runtime-boundaries.sh`
(lines 92-151), added during the code audit remediation. Verified:
- Hardcoded `.patina/` path guard: lines 92-141
- Blanket `#![allow(dead_code)]` guard: lines 142-155

Exit criteria pva13 and pva14 are checked off. No work needed.

## Not in Scope

- History rewriting
- Runtime pando execution (`patina pando run` — future spec)
- Per-child WIT world generation from manifest (compose-worlds-from-toys — future spec)
- Interface/skill system overhaul (separate spec)
