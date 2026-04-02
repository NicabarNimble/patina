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

Vocabulary overhaul: introduce "pando" for composed children, collapse "knowledge-child" and "pipeline" into just "child." One kind, one world, one engine. The toybox is the only thing that differentiates children.

## Context

Latin: pandō — "I spread." Named after the Pando aspen colony: 47,000 trees that are one organism connected by shared roots. A pando of children shares roots through Mother and appears as one capability to the user.

The six concepts:

| Concept | Word | What it is |
|---------|------|-----------|
| The protocol | **Patina** | 5 verbs, native CLI, belief core |
| The daemon | **Mother** | Authority, orchestration, state |
| A WASM worker | **Child** | One job, reusable, composable |
| A capability grant | **Toy** | WASI-first sandbox opening |
| Composed children | **Pando** | One organism, shared roots, spreads the patina |
| The workspace | **Project** | Where knowledge accumulates |

## Why One Kind

Today there are two kinds: `knowledge-child` (Mother-hosted, full toybox, lifecycle) and `pipeline` (CLI-invoked, log only, stateless). But:

- Pipeline IS a child. Saying it's a different kind implies it isn't.
- The only real difference is which toys are granted and whether lifecycle exports are active.
- A grammar parser that wanted to cache parse trees would need state — suddenly it's not a "pipeline" anymore.
- The toybox should determine behavior, not the kind label.

**Solution:** One WIT world with all exports. SDK provides default stubs for lifecycle exports. A grammar parser exports `health() -> healthy`, `tick() -> []`, `drain() -> []` as no-ops (SDK gives these for free). From Mother's perspective it's just a child with no lifecycle activity. The toybox determines what it can do.

**What was pipeline becomes:** a child with `[needs].toys = ["log"]` and default lifecycle stubs. Same behavior, simpler model.

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

**AF2 — Capability widening risk.**
Pipeline children get only host_log today. Kind collapse must not silently grant
them state, layer-fs, git, etc.
**Resolution:** The merged engine enforces `[needs].toys` from child.toml at load
time. A child declaring `toys = ["log"]` gets only log. Toy grants are checked
before linking — not after. Engine merge must preserve this check. Verification:
after merge, confirm a `toys = ["log"]` child does NOT get state/layer-fs/git.

**AF3 — kind field decision.**
**Resolution:** `kind = "child"` is required in child.toml. `"knowledge-child"`
and `"pipeline"` accepted as silent aliases mapped to `Child`. No deprecation
warning (these are internal, not user-facing). The field is not optional.

**AF4 — Pando YAML schema.**
**Resolution:** `objective_id:` key in pando YAML renamed to `pando:`.
Old `objective_id:` key rejected with message: "Renamed: use `pando:` instead
of `objective_id:`". Locked in pva1 commit.

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

## Not in Scope

- History rewriting (sessions, archived specs, git tags stay as-is)
- Runtime pando execution model (future spec — `patina pando run`, pando YAML format, Mother orchestration)
- Compose-worlds-from-toys (future spec — per-child WIT generation from manifest)
