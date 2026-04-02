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

Merge `PipelineEngine` (`src/child/internal/pipeline.rs`) and `KnowledgeChildEngine` (`src/child/internal/knowledge_child.rs`) into `ChildEngine`.

The merged engine:
- Links all WIT imports (WASI + Patina toys)
- Checks `[needs].toys` at load time and only activates granted toys
- Calls lifecycle exports (`tick`, `drain`, `health`) — stubs return no-ops for simple children
- AOT caching from PipelineEngine carries over

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

- **Grammar plugin recompilation** — users with installed grammars need `patina setup grammars --force` after upgrade. Add a version check in grammar discovery that detects old-world plugins and prompts.
- **Backward compat** — `kind = "knowledge-child"` and `kind = "pipeline"` must keep working in child.toml for existing children until they're recompiled. `FromStr` handles this.
- **Engine merge complexity** — `KnowledgeChildEngine` is 1109 lines, `PipelineEngine` is 276 lines. The merge needs care around linker setup (knowledge-child links ~15 interfaces, pipeline links 1). Unified engine links all interfaces but checks grants.

## Phase 3: CI Regression Guards

The audit remediation fixed several systemic issues. Without CI guards, they regrow silently. These are regression guards — we fixed the problem, but nothing prevents it from coming back.

**Hardcoded `.patina/` paths:** We migrated 5 sites to use `crate::paths` in A9. But the next person (or AI agent) writing code might just type `.join(".patina")` instead of using the paths module. Without a CI check, the duplication silently regrows.

**Blanket `#![allow(dead_code)]`:** We replaced the file-level blanket allows with per-item annotations in A22. But someone adding a new toy host function might slap `#![allow(dead_code)]` back on the file to suppress warnings. Without a guard, the blanket comes back and hides genuinely dead code again.

Both are one-line grep checks that prevent regression on work we already did.

### pva12: Update pre-push world references

`resources/git/pre-push-checks.sh:31` has `SDK_WORLDS=(knowledge-child pipeline)`. After kind unification this becomes `SDK_WORLDS=(child)`. Steps 1+2 (WIT consistency + mirror completeness) update their paths from `wit/knowledge-child/` and `wit/pipeline/` to `wit/child/`.

### pva13: Path truth regression guard

Add to `resources/scripts/check-runtime-boundaries.sh` (or a new script called from pre-push):

```bash
echo "Checking no hardcoded .patina/ paths outside paths.rs..."
violations=$(grep -r '\.join(".patina")' src/ --include='*.rs' \
    | grep -v 'src/paths.rs' \
    | grep -v '#\[cfg(test)\]' \
    | grep -v 'src/migration.rs' || true)
if [[ -n "$violations" ]]; then
    echo "$violations"
    echo "error: hardcoded .patina/ path found — use crate::paths instead"
    exit 1
fi
```

`migration.rs` is excluded because it legitimately references old paths (that's its job). Test code is excluded because temp directory setup often constructs `.patina/` inline.

### pva14: Blanket dead_code regression guard

Add to the same script:

```bash
echo "Checking no blanket #![allow(dead_code)] annotations..."
blankets=$(grep -rn '#!\[allow(dead_code)\]' src/ --include='*.rs' || true)
if [[ -n "$blankets" ]]; then
    echo "$blankets"
    echo "error: blanket #![allow(dead_code)] found — use per-item #[allow(dead_code)] instead"
    exit 1
fi
```

Per-item `#[allow(dead_code)]` on specific functions/structs is fine (some toy host functions are consumed by WASM bindgen). File-level blanket allows mask genuinely dead code.

## Not in Scope

- History rewriting
- Runtime pando execution (`patina pando run` — future spec)
- Per-child WIT world generation from manifest (compose-worlds-from-toys — future spec)
- Interface/skill system overhaul (separate spec)
