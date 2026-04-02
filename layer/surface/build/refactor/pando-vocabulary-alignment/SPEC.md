---
type: refactor
id: pando-vocabulary-alignment
status: draft
created: 2026-04-01
sessions:
  origin: 20260331-224232-852361000
references:
  - layer/core/patina-identity.md
  - layer/core/unix-philosophy.md
  - layer/surface/build/feat/child-construction-canon/SPEC.md
beliefs:
  - "[[pando-is-composed-children]]"
  - "[[five-boundaries-no-overlap]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[wasi-is-foundation-not-option]]"
  - "[[children-are-wasm]]"
  - "[[world-boundary-is-type-safety]]"
  - "[[core-primitives-are-not-children]]"
related:
  - layer/surface/build/feat/child-construction-canon/SPEC.md
  - AGENTS.md
  - sdk/patina-sdk/
  - src/child/internal/mod.rs
  - src/child/internal/knowledge_child.rs
  - src/child/engine.rs
  - wit/knowledge-child/
  - wit/pipeline/
  - children/
exit_criteria:

# Phase 1: Vocabulary (docs only, no code)

  - id: pva1-canon-updated
    text: "child-construction-canon SPEC.md uses 'pando' instead of 'objective recipe' for composed child groups. Recipe YAML section renamed. ccc7 gate text updated."
    checked: false
  - id: pva2-agents-updated
    text: "AGENTS.md vocabulary section uses the six concepts: Patina, Mother, child, toy, pando, project."
    checked: false
  - id: pva3-beliefs-connected
    text: "[[pando-is-composed-children]] belief linked from [[child-construction-canon]] and [[five-boundaries-no-overlap]]."
    checked: false
  - id: pva4-sdk-docs
    text: "sdk/patina-sdk/README.md uses pando vocabulary for composition examples and drops knowledge-child/pipeline distinction in user-facing docs."
    checked: false

# Phase 2: Kind unification (code change)

  - id: pva5-one-wit-world
    text: "wit/pipeline/pipeline.wit merged into wit/knowledge-child/ (renamed to wit/child/). One WIT world with all exports. SDK provides default stub implementations for lifecycle exports (health, tick, drain, on-load, on-unload)."
    checked: false
  - id: pva6-one-engine
    text: "PipelineEngine and KnowledgeChildEngine merged into single ChildEngine. One engine loads all children regardless of which exports they actively use."
    checked: false
  - id: pva7-kind-collapsed
    text: "ChildKind enum has one variant (Child). child.toml requires `kind = \"child\"`. Old values \"knowledge-child\" and \"pipeline\" accepted as silent aliases. Retired kind error messages updated."
    checked: false
  - id: pva8-grammar-plugins-recompiled
    text: "9 grammar plugins recompiled against unified world. `patina setup grammars` installs updated binaries."
    checked: false
  - id: pva9-sdk-unified
    text: "SDK `knowledge-child` and `pipeline` features merged into single `child` feature. patina-sdk Cargo.toml and src/lib.rs updated."
    checked: false
  - id: pva10-children-updated
    text: "All 13 children in children/ updated: child.toml uses `kind = \"child\"`, Cargo.toml targets unified WIT world."
    checked: false
  - id: pva11-template-updated
    text: "children/template/ and `patina child init` scaffold use `kind = \"child\"` and unified world."
    checked: false

# Phase 3: CI regression guards

  - id: pva12-ci-world-refs
    text: "pre-push-checks.sh SDK_WORLDS updated from (knowledge-child pipeline) to (child). WIT consistency and mirror checks use wit/child/ paths."
    checked: false
  - id: pva13-ci-path-guard
    text: "CI guard for hardcoded .patina/ paths exists in resources/scripts/check-runtime-boundaries.sh and passes."
    checked: true
  - id: pva14-ci-blanket-dead-code-guard
    text: "CI guard for blanket #![allow(dead_code)] exists in resources/scripts/check-runtime-boundaries.sh and passes."
    checked: true

# Proof

  - id: pva15-no-history-rewrite
    text: "No session artifacts, archived specs, or git tags are modified."
    checked: false
  - id: pva16-compile-proof
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass after all changes."
    checked: false
  - id: pva17-child-run-proof
    text: "`patina child run doctor health` works. `patina scrape code` uses grammar plugins successfully. Both paths use the unified engine."
    checked: false
  - id: pva18-guard-scripts-pass
    text: "All pre-push guard scripts pass including the new path and dead_code guards."
    checked: false
---

# refactor: Pando Vocabulary Alignment

Two changes: give a name to composed children (pando), and collapse the
artificial knowledge-child/pipeline split into one kind (child).

## Pando — a word for a group of children

Latin: pandō — "I spread." Named after the Pando aspen colony: 47,000 trees
that are one organism connected by shared roots.

Sometimes you compose several children to do one thing. A pando is that group.
"The scrape pando" instead of "the group of six children that together do
folder-text-to-parquet."

Each child in a pando has bounded agency (`[[children-have-agency-toys-are-capabilities]]`)
— it makes decisions within the sandbox Mother grants. The pando's behavior
emerges from composition, not from any single child's intelligence. Mother
orchestrates the pando; children do the work; toys are the capability surface
each child gets.

## Why One Kind

Today there are two kinds of child: `knowledge-child` and `pipeline`. We split
them early because grammar parsers felt different from knowledge children. They
aren't — or at least, we haven't proven they are. The split was premature.

**Don't invent categories before the seam is proven.** If a real seam emerges
from building more children, we split then — with evidence.

### What the beliefs tell us

**`[[children-are-wasm]]`** — children are WASM runtime units. Not some
children. All children. Native capabilities belong to Mother. The pipeline/
knowledge-child split created two WASM engine paths for what is one runtime
model. One kind of child, one WASM runtime.

**`[[children-have-agency-toys-are-capabilities]]`** — children have bounded
agency within Mother's sandbox. Toys are capability surfaces granted at init
time via `[needs].toys` in child.toml. The manifest determines what a child
can do — not a kind label. A grammar parser with `toys = ["log"]` has bounded
agency over logging. A knowledge child with `toys = ["log", "state", "layer-fs"]`
has broader agency. Same runtime, different grants.

**`[[world-boundary-is-type-safety]]`** — the WIT world boundary is where type
safety lives. Capability isolation determines what a child can see. Having two
worlds (knowledge-child and pipeline) with different `handle()` signatures
created two type boundaries for what should be one. One world means one type
contract. String dispatch within that world is intentional low coupling — the
world boundary provides isolation, not the payload types.

**`[[core-primitives-are-not-children]]`** — Patina's knowledge primitives
(scrape, scry, assay, belief) are core Mother capabilities. Children are
pluggable strategy providers that feed INTO core. Grammar parsers are scrape
strategy children — they feed into the core scrape primitive. They're not a
different species. They're children doing one job.

### What this means concretely

- The toybox determines behavior, not the kind label.
- A grammar parser that wants to cache parse trees would need state — under
  the old model it suddenly isn't a "pipeline" anymore. Under one kind, it
  just adds `state` to its toys.
- `GrantedCapabilities` is resolved from `[needs].toys` at init time and
  checked at call-time. This is the security boundary, not the kind enum.

**Solution:** One WIT world with all exports. SDK provides default stubs for
lifecycle exports. A grammar parser exports `health() -> healthy`,
`tick() -> []`, `drain() -> []` as no-ops (SDK gives these for free). From
Mother's perspective it's just a child with no lifecycle activity. The toybox
determines what it can do.

**What was pipeline becomes:** a child with `[needs].toys = ["log"]` and
default lifecycle stubs. Same behavior, simpler model. More children will
come. If a real seam appears, we split with evidence then.

## Phases

**Phase 1 (docs only):** Update vocabulary in specs, AGENTS.md, SDK docs, beliefs. No code changes. No compile risk.

**Phase 2 (code change):** Merge WIT worlds, merge engines, collapse ChildKind, recompile grammar plugins, update SDK features, update all children and templates.

## Audit Findings (resolved)

Audit review surfaced 10 risks. Each is resolved here or in DESIGN.md.

**AF1 — handle() signature mismatch (CRITICAL).**
pipeline exports `handle(request: string)`, knowledge-child exports
`handle(action: string, payload: string)`. This is the real migration risk.
**Resolution:** Unified world uses the knowledge-child signature (action + payload).
Pipeline's single `request` maps to `action = "handle", payload = request`.
Grammar plugins must be recompiled against the new signature — the SDK provides
a compatibility shim in the default stubs so existing plugin code compiles with
a one-line change to the export. Details in DESIGN.md pva5.
**Non-SDK children:** Any child built with raw `wit-bindgen` (not patina-sdk)
must update their `handle` export to match the unified signature manually.
This spec assumes all current children use the SDK. If a non-SDK child exists,
it must be identified and migrated explicitly during pva10.

**AF2 — Capability widening risk.**
Pipeline children get only host_log today. Kind collapse must not silently grant
them state, layer-fs, git, etc.
**Resolution:** The merged engine links all WIT interfaces at linker build time
(Wasmtime requires this), but gates capability at the call boundary — host
functions for non-granted toys return an error or no-op when invoked. A child
declaring `toys = ["log"]` has all imports wired but only log actually functions.
This is how KnowledgeChildEngine already works. Engine merge must preserve this
runtime gate pattern, not weaken it.
**Verification (required, not optional):** Add a test that loads a child with
`toys = ["log"]` and asserts that calling a state/layer-fs/git toy function
returns an error. This must be in `cargo test`, not "inspect logs."

**AF3 — kind field decision.**
**Resolution:** `kind = "child"` is required in child.toml. `"knowledge-child"`
and `"pipeline"` accepted as silent aliases mapped to `Child`. No deprecation
warning (these are internal, not user-facing). The field is not optional.

**AF4 — Pando YAML schema.**
**Resolution:** `objective_id:` key in pando YAML renamed to `pando:` in docs
(Phase 1, pva1). Parser enforcement — old `objective_id:` key rejected with
message "Renamed: use `pando:` instead of `objective_id:`" — happens in Phase 2
when code changes are in scope. Phase 1 is docs-only; the parser doesn't exist
yet so rejection behavior is a Phase 2 deliverable, not a Phase 1 claim.

**AF5 — Grammar plugin path/layout.**
`~/.patina/pipeline/` contains 9 grammar plugins. Path stays after kind collapse
(it's a storage location, not a kind label). Grammar plugins are recompiled
against the unified world but installed to the same path. No path rename.

**AF6 — SDK macro bridge.**
`register_pipeline_child!` macro and `PipelineChild` trait exist in
`sdk/patina-sdk/src/pipeline.rs` (lines 134-226), exported via
`sdk/patina-sdk/src/lib.rs:55` behind the `pipeline` feature flag. Grammar
plugins use these.
**Resolution:** When SDK features merge (pva9), `register_pipeline_child!` and
`PipelineChild` become deprecated aliases. The macro re-exports to the unified
registration. The trait gets a blanket impl that adapts `handle(request)` to
`handle(action, payload)` (ignoring action, passing payload as request). These
shims stay for one release, then are removed. `pipeline.rs` file is kept but
marked `#[deprecated]` with migration guidance pointing to the unified `Child`
trait.

**AF7 — Phase 1 docs vs Phase 2 code gap.**
**Resolution:** Phase 1 doc updates include a compatibility note:
"Note: code still uses knowledge-child/pipeline until Phase 2 of
pando-vocabulary-alignment." Note removed in Phase 2 pva7 commit.

**AF8 — CI guard duplication.**
pva13 (hardcoded .patina/ paths) and pva14 (blanket dead_code) guards
ALREADY EXIST in `resources/scripts/check-runtime-boundaries.sh` (lines 92-151).
**Resolution:** pva13 and pva14 exit criteria are already met. Check them off.
Do not duplicate. pva12 (SDK_WORLDS update) is the only Phase 3 code change.

**AF9 — Test acceptance too light.**
**Resolution:** Expanded verification after Phase 2 (added to pva16/pva17):
- Old kind = "knowledge-child" child.toml still loads
- Old kind = "pipeline" child.toml still loads
- `patina child run doctor health` works
- `patina scrape code` works (grammar plugins via unified engine)
- A child with `toys = ["log"]` does NOT get state/layer-fs/git access
- `patina child init` produces valid child.toml with `kind = "child"`

**AF10 — Terminology lock drift.**
**Resolution:** Phase 1 AGENTS.md update references target state with "will
become wit/child/ in Phase 2" note. Phase 2 pva7 commit removes the note
and updates paths to final state.

## Policies (locked)

**Alias lifetime:** `kind = "knowledge-child"` and `kind = "pipeline"` aliases
in FromStr, and `pipeline`/`knowledge-child` feature aliases in SDK Cargo.toml,
are removed in the NEXT minor release after this spec completes. If this spec
ships as v0.46.0, aliases are removed in v0.47.0. Enforcement: the removal is
a one-line change tracked as a follow-up commit after next minor bump.

**WIT package migration:** The unified world uses package `patina:child@0.1.0`,
replacing `patina:knowledge-child@0.1.0` and `patina:pipeline@0.1.0`. Old
compiled WASM artifacts (targeting old package names) will fail to instantiate
with a linker error. There is no runtime compatibility for old artifacts — they
must be recompiled. Grammar plugins are recompiled in pva8. Third-party children
(if any) must recompile against the new world.

**User-facing terminology:** `~/.patina/pipeline/` path stays as a storage
location. The word "pipeline" in path names is not deprecated — it describes
what grammar plugins do (parse pipelines). The deprecation applies to
`kind = "pipeline"` in child.toml and `pipeline` as a world/feature name, not
to the storage directory or the concept of processing pipelines.

**Required exports for non-SDK components:** The unified world requires all
exports (init, name, handle, health, tick, drain, on-load, on-unload). SDK
children get default stubs for free. Non-SDK children (raw wit-bindgen) must
implement all exports. The WIT file is the contract — if it exports it, you
must provide it.

## Not in Scope

- History rewriting (sessions, archived specs, git tags stay as-is)
- Runtime pando execution model (future spec — `patina pando run`, pando YAML format, Mother orchestration)
- Compose-worlds-from-toys (future spec — per-child WIT generation from manifest)
