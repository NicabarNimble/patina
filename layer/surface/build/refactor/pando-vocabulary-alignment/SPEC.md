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
    text: "ChildKind enum has one variant. child.toml uses `kind = \"child\"` (or kind field removed entirely). Retired kind error messages updated."
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
    text: "CI guard added: `grep -r '.join(\".patina\")' src/ --include='*.rs' | grep -v 'src/paths.rs' | grep -v '#[cfg(test)]'` must return empty. Prevents hardcoded .patina/ paths regrowing outside paths.rs (regression guard for A9)."
    checked: false
  - id: pva14-ci-blanket-dead-code-guard
    text: "CI guard added: `grep -r '#![allow(dead_code)]' src/ --include='*.rs'` must return empty. Prevents blanket dead_code allows reappearing on files (regression guard for A22)."
    checked: false

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

## Not in Scope

- History rewriting (sessions, archived specs, git tags stay as-is)
- Runtime pando execution model (future spec — `patina pando run`, pando YAML format, Mother orchestration)
- Compose-worlds-from-toys (future spec — per-child WIT generation from manifest)
