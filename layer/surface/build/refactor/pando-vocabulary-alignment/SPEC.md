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
    text: "AGENTS.md vocabulary section reflects current concepts: Patina, Mother, child, toy, pando, project. Includes Phase 2 compatibility note per AF7."
    checked: false
  - id: pva3-beliefs-connected
    text: "[[pando-is-composed-children]] belief linked from [[child-construction-canon]] and [[five-boundaries-no-overlap]]."
    checked: false
  - id: pva4-sdk-docs
    text: "sdk/patina-sdk/README.md uses pando vocabulary for composition examples and drops knowledge-child/pipeline distinction in user-facing docs."
    checked: false

# Phase 2: SDK-based kind collapse and world unification (code change)

  - id: pva5-one-wit-world
    text: "wit/pipeline/ merged into wit/knowledge-child/ (renamed to wit/child/). One WIT world. SDK provides default stub implementations for lifecycle exports."
    checked: false
  - id: pva6-one-engine
    text: "PipelineEngine and KnowledgeChildEngine merged into single ChildEngine."
    checked: false
  - id: pva7-kind-collapsed
    text: "ChildKind enum has one variant (Child). child.toml requires `kind = \"child\"`. Old values \"knowledge-child\" and \"pipeline\" accepted as silent aliases."
    checked: false
  - id: pva8-grammar-plugins-recompiled
    text: "9 grammar plugins recompiled against unified world via SDK. `patina setup grammars` installs updated binaries."
    checked: false
  - id: pva9-sdk-unified
    text: "SDK `knowledge-child` and `pipeline` features merged into single `child` feature. PipelineChild trait and register_pipeline_child! macro deprecated as temporary shims."
    checked: false
  - id: pva10-children-updated
    text: "All 14 child.toml files (13 children + template) updated: `kind = \"child\"`, Cargo.toml targets unified WIT world. Legacy children inventoried."
    checked: false
  - id: pva11-template-updated
    text: "children/template/ and `patina child init` scaffold use `kind = \"child\"` and unified world. No legacy world option exposed."
    checked: false
  - id: pva-cap-test
    text: "Cargo test verifies a child with toys = [\"log\"] cannot access state/layer-fs/git host functions."
    checked: false
  - id: pva-legacy-inventory
    text: "Legacy children inventoried. Migration follow-ups created for each. No new legacy children may be introduced."
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
    text: "All pre-push guard scripts pass."
    checked: false
---

# refactor: Pando Vocabulary Alignment

Two changes: give a name to composed children (pando), and collapse the
artificial knowledge-child/pipeline split into one kind and one world (child).

## Pando — a word for a group of children

Latin: pandō — "I spread." Named after the Pando aspen colony: 47,000 trees
that are one organism connected by shared roots.

Sometimes you compose several children to do one thing. A pando is that group.
"The scrape pando" instead of "the group of six children that together do
folder-text-to-parquet." DuckLake is a pando — multiple children, each with
a role, composed for one purpose.

Each child in a pando has bounded agency (`[[children-have-agency-toys-are-capabilities]]`)
— it makes decisions within the sandbox Mother grants. The pando's behavior
emerges from composition, not from any single child's intelligence. Mother
orchestrates; children do the work; toys are the capability surface each
child gets.

## Why One Kind and One World

Today there are two kinds (`knowledge-child`, `pipeline`) and two WIT worlds.
We split them early because grammar parsers felt different from knowledge
children. They aren't — or at least, we haven't proven they are. The split
was premature. Both the kind enum and the two worlds were the same false seam.

**Don't invent categories before the seam is proven.** If a real seam emerges
from building more children, we split then — with evidence.

### What the beliefs tell us

**`[[children-are-wasm]]`** — children are WASM runtime units. Not some
children. All children. Native capabilities belong to Mother. The two-engine
split (PipelineEngine, KnowledgeChildEngine) was two runtime paths for one
model.

**`[[children-have-agency-toys-are-capabilities]]`** — children have bounded
agency within Mother's sandbox. Toys are capability surfaces granted at init
time via `[needs].toys` in child.toml. A grammar parser with `toys = ["log"]`
has bounded agency over logging. A knowledge child with
`toys = ["log", "state", "layer-fs"]` has broader agency. Same runtime,
different grants. The manifest determines behavior — not a kind label.

**`[[world-boundary-is-type-safety]]`** — WIT world boundaries provide
compile-time isolation. This principle is sound. But pipeline and
knowledge-child were never a real seam — they were 1:1 with the false kind
split. Merging them into `wit/child/` corrects a misapplication. Future worlds
may emerge when real seams appear. The number of worlds grows from evidence.

**`[[core-primitives-are-not-children]]`** — Patina's knowledge primitives
(scrape, scry, assay, belief) are core Mother capabilities. Children are
pluggable strategy providers that feed INTO core. Grammar parsers are scrape
strategy children. They're not a different species. They're children doing
one job.

### What this means concretely

- The toybox determines behavior, not the kind label.
- A grammar parser that wants to cache parse trees would need state — under
  the old model it suddenly isn't a "pipeline" anymore. Under one kind, it
  just adds `state` to its toys.
- `GrantedCapabilities` is resolved from `[needs].toys` at init time and
  checked at the call boundary. This is the security boundary, not the kind
  enum and not the world split.

**Solution:** One WIT world (`wit/child/`) with all exports. SDK provides
default stubs for lifecycle exports. A grammar parser exports
`health() -> healthy`, `tick() -> []`, `drain() -> []` as no-ops (SDK gives
these for free). From Mother's perspective it's just a child with no lifecycle
activity. The toybox determines what it can do.

**What was pipeline becomes:** a child with `[needs].toys = ["log"]` and
default lifecycle stubs. Same behavior, simpler model. More children will
come. If a real seam appears, we split with evidence then.

## Hard Rules

**All children must use the SDK.** The SDK is the only supported way to build
children. Legacy children (built with raw wit-bindgen or hand-rolled bindings)
exist only as temporary compatibility artifacts from before SDK maturity.
They are slated for migration to the SDK, not long-term support.

**No new legacy children may be introduced.** `patina child init` produces
SDK children. The template uses the SDK. Documentation describes the SDK path
only.

**Legacy children must be inventoried.** During pva10, identify any children
not using patina-sdk. Create explicit migration follow-ups for each. The
shims in pva9 (`PipelineChild` trait, `register_pipeline_child!` macro) are
temporary triage — they exist to keep legacy grammar plugins compiling during
this transition, not as a permanent compatibility layer.

## Phases

**Phase 1 (docs only):** Vocabulary truth alignment. Update specs, AGENTS.md,
SDK docs, beliefs. No code changes. No compile risk.

**Phase 2 (code change):** SDK-based kind collapse and world unification.
Merge WIT worlds into `wit/child/`, merge engines, collapse ChildKind,
recompile grammar plugins via SDK, update SDK features, update all children
and templates. Inventory legacy children.

**Phase 3 (CI):** Update pre-push world references.

## Audit Findings (resolved)

Audit review surfaced 10 risks. Each resolved here or in DESIGN.md.

**AF1 — handle() signature mismatch (CRITICAL).**
pipeline exports `handle(request: string)`, knowledge-child exports
`handle(action: string, payload: string)`.
**Resolution:** Unified world uses the knowledge-child signature (action +
payload). Grammar plugins must be recompiled via SDK. SDK provides a temporary
shim: `PipelineChild` trait adapts `handle(request)` to `handle(action, payload)`.
**Legacy children:** Any child not using the SDK must be inventoried in pva10
and migrated. The shim is temporary triage, not permanent support.

**AF2 — Capability widening risk.**
Pipeline children get only host_log today. Kind collapse must not silently
grant them state, layer-fs, git, etc.
**Resolution:** The merged engine links all WIT interfaces at linker build
time (Wasmtime requires this), but gates capability at the call boundary —
host functions for non-granted toys return an error when invoked. A child
declaring `toys = ["log"]` has all imports wired but only log functions.
This is how KnowledgeChildEngine already works.
**Required test:** `cargo test` must verify a child with `toys = ["log"]`
cannot access state/layer-fs/git host functions. Not optional.

**AF3 — kind field decision.**
`kind = "child"` is required in child.toml. `"knowledge-child"` and
`"pipeline"` accepted as silent aliases mapped to `Child`. Not optional.

**AF4 — Pando YAML schema.**
`objective_id:` key renamed to `pando:` in docs (Phase 1, pva1). Parser
enforcement is out of scope for this spec — no pando YAML parser exists yet.
The rename is vocabulary only. Parser behavior will be defined when the
runtime pando execution spec is written.

**AF5 — Grammar plugin path/layout.**
`~/.patina/pipeline/` path stays. It's a storage location, not a kind label.
Grammar plugins are recompiled against the unified world but installed to
the same path.

**AF6 — SDK macro bridge.**
`register_pipeline_child!` macro and `PipelineChild` trait exist in
`sdk/patina-sdk/src/pipeline.rs`. When SDK features merge (pva9), these
become deprecated temporary shims. `PipelineChild` adapts `handle(request)`
to `handle(action, payload)`. Shims removed in the next minor release after
this spec ships. They are migration triage, not permanent compatibility.

**AF7 — Phase 1 docs vs Phase 2 code gap.**
Phase 1 doc updates include a compatibility note: "Note: code still uses
knowledge-child/pipeline until Phase 2." Note removed in Phase 2 pva7 commit.

**AF8 — CI guard duplication.**
pva13 and pva14 guards already exist in
`resources/scripts/check-runtime-boundaries.sh`. Already checked off.

**AF9 — Test acceptance.**
Phase 1: truth-alignment checks only (docs match intended vocabulary).
Phase 2 verification:
- Old `kind = "knowledge-child"` child.toml still loads (alias)
- Old `kind = "pipeline"` child.toml still loads (alias)
- `patina child run doctor health` works
- `patina scrape code` works (grammar plugins via unified engine)
- Capability gate: `cargo test` proves `toys = ["log"]` child blocked from state/layer-fs/git
- `patina child init` produces valid SDK child with `kind = "child"`
- Legacy children inventoried with migration follow-ups

**AF10 — Terminology lock drift.**
Phase 1 AGENTS.md references target state with "will become wit/child/ in
Phase 2" note. Phase 2 pva7 commit removes the note.

## Policies (locked)

**Alias lifetime:** `kind = "knowledge-child"` and `kind = "pipeline"` aliases
in FromStr, and `pipeline`/`knowledge-child` feature aliases in SDK Cargo.toml,
are removed in the next minor release after this spec ships. If this spec
ships as v0.46.0, aliases are removed in v0.47.0.

**WIT package migration:** Unified world uses package `patina:child@0.1.0`,
replacing `patina:knowledge-child@0.1.0` and `patina:pipeline@0.1.0`. Old
compiled WASM artifacts will fail to instantiate with a linker error. No
runtime compatibility — they must be recompiled.

**User-facing terminology:** `~/.patina/pipeline/` path stays as a storage
location. The word "pipeline" in path names is not deprecated. The deprecation
applies to `kind = "pipeline"` in child.toml and `pipeline` as a world/feature
name.

**SDK-only children:** All children must use the SDK. The unified world
requires all exports (init, name, handle, health, tick, drain, on-load,
on-unload). SDK children get default stubs for free. Legacy children are
temporary — they must migrate to the SDK. No new legacy children.

**World boundaries going forward:** This spec merges two worlds because the
pipeline/knowledge-child split was a false seam. The principle that world
boundaries provide compile-time isolation (`[[world-boundary-is-type-safety]]`)
remains sound. New worlds may emerge when real seams appear — the number of
worlds is not fixed, it grows from evidence.

## In Scope

- Vocabulary truth alignment (Phase 1)
- SDK-based kind/world/engine unification (Phase 2)
- Legacy child inventory and migration follow-up creation (Phase 2)
- CI world reference updates (Phase 3)

## Not in Scope

- History rewriting (sessions, archived specs, git tags stay as-is)
- Runtime pando execution model (future spec)
- Per-child WIT world generation from manifest (future spec)
- Pando YAML parser enforcement (future spec — parser doesn't exist yet)
- Broad legacy/third-party compatibility guarantees
