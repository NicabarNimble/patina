---
type: refactor
id: child-rename
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-135104-312151000
blocked_by:
  - ducklake-retirement
beliefs:
  - "[[children-are-wasm]]"
  - "[[children-have-agency-toys-are-capabilities]]"
related:
  - src/child/internal/mod.rs
  - src/child/internal/knowledge_child.rs
  - src/child/engine.rs
  - src/mother/mod.rs
  - mother/src/runtime.rs
  - sdk/patina-sdk/
  - wit/knowledge-child/
  - children/
exit_criteria:

  - id: cr1-wit-world-renamed
    text: "wit/knowledge-child/ renamed to wit/child/. WIT package becomes patina:child@0.1.0, world becomes child. SDK embedded WIT copy updated to match."
    checked: false

  - id: cr2-engine-renamed
    text: "KnowledgeChildEngine renamed to ChildEngine throughout src/. src/child/internal/knowledge_child.rs renamed to child.rs. Bindgen path and world strings updated."
    checked: false

  - id: cr3-trait-renamed
    text: "KnowledgeChild trait (defined in mother/src/runtime.rs and re-exported via src/mother/mod.rs) renamed to Child. All implementors and call sites updated (broker, checkin, daemon, runtime)."
    checked: false

  - id: cr4-kind-collapsed
    text: "ChildKind::KnowledgeChild variant renamed to Child. FromStr keeps silent aliases: \"knowledge-child\" → Child, \"pipeline\" stays Pipeline. Error messages for retired kinds updated."
    checked: false

  - id: cr5-sdk-renamed
    text: "sdk/patina-sdk/src/knowledge_child.rs renamed to child.rs. KnowledgeChild trait → Child, KnowledgeChildPlugin → ChildPlugin. SDK Cargo.toml adds child feature; knowledge-child becomes alias for child."
    checked: false

  - id: cr6-children-updated
    text: "All child manifests under children/* updated: kind=\"child\" (14 files including template). All child Cargo.toml files under children/* updated to feature=\"child\" and world target path=\"../../wit/child\" with world=\"child\" (14 files including template). The 7 broken wit/worlds references are fixed as part of this."
    checked: false

  - id: cr7-templates-updated
    text: "resources/templates/child/knowledge-child/ renamed to resources/templates/child/child/. Template files updated. patina child init scaffold uses child world."
    checked: false

  - id: cr8-ci-updated
    text: "resources/git/pre-push-checks.sh SDK_WORLDS updated from (knowledge-child pipeline) to (child pipeline)."
    checked: false

  - id: cr9-compile-proof
    text: "cargo check --workspace -q passes. cargo test -q --lib passes."
    checked: false
---

# refactor: Child Rename

Rename `knowledge-child` → `child` everywhere. The `knowledge-child` name was
a premature distinction. All children are knowledge children. The name adds no
information.

Pipeline and grammar plugins are untouched by this spec. That consolidation is
`engine-consolidate`, which is blocked by this spec.

## What changes

**WIT world** (`cr1`): `wit/knowledge-child/` → `wit/child/`. Package
`patina:knowledge-child@0.1.0` → `patina:child@0.1.0`. World `knowledge-child`
→ `child`. SDK has an embedded copy of the WIT — both updated together.

**Engine** (`cr2`): `KnowledgeChildEngine` → `ChildEngine`. File
`src/child/internal/knowledge_child.rs` → `child.rs`. Two string literals
in the bindgen macro change. No logic changes.

**Trait** (`cr3`): `KnowledgeChild` trait in `mother/src/runtime.rs`
(re-exported via `src/mother/mod.rs`) → `Child`. Used in broker, checkin,
daemon, runtime — all call sites renamed. The trait is the Mother-side
interface to a loaded child WASM instance.

**ChildKind enum** (`cr4`): `KnowledgeChild` variant → `Child`. `FromStr`
keeps `"knowledge-child"` as a silent alias so existing child.toml files
still load. Silent alias removed in the next minor release.

**SDK** (`cr5`): `sdk/patina-sdk/src/knowledge_child.rs` → `child.rs`.
`KnowledgeChild` trait and `KnowledgeChildPlugin` renamed. Cargo feature
`child` added; `knowledge-child = ["child"]` alias kept temporarily.

**Children** (`cr6`): All `children/*/child.toml` files (14 including
`children/template`) use `kind = "child"`. All `children/*/Cargo.toml` files
(14 including template) are updated to the `child` SDK feature and
`path = "../../wit/child"`, `world = "child"`. The 7 children pointing at the
nonexistent `wit/worlds` path are fixed here — they all become `wit/child`.

**Templates + CI** (`cr7`, `cr8`): Scaffold template dir renamed. CI world
reference updated.

## What does not change

- `PipelineEngine` — untouched
- `wit/pipeline/` — untouched
- Grammar plugin discovery and `~/.patina/pipeline/` path — untouched
- Engine logic — zero changes, only string literals and names
- Runtime behavior — none intended (rename-only); manifest parsing aliases and
  retired-kind error text are updated as part of `cr4`

## Risks

- `KnowledgeChild` trait rename crosses 5+ files — mechanical but ripply.
  Read all call sites before editing.
- WIT package rename changes the generated Rust module path from
  `patina::knowledge_child::*` to `patina::child::*`. References in
  `sdk/patina-sdk/src/child.rs` (post-rename) must be updated to match.
- The 7 `wit/worlds` children have never been compiled against a real WIT
  world — verify they actually compile after this fix.
