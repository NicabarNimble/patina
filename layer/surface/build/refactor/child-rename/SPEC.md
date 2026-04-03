---
type: refactor
id: child-rename
status: complete
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
  - wit/child/
  - children/
exit_criteria:

  - id: cr1-wit-world-renamed
    text: "wit/knowledge-child/ renamed to wit/child/. WIT package becomes patina:child@0.1.0, world becomes child. SDK embedded WIT copy renamed from sdk/patina-sdk/wit/knowledge-child/ to sdk/patina-sdk/wit/child/ and updated to match. Drift check: diff -r wit/child/ sdk/patina-sdk/wit/child/ shows no differences."
    checked: true

  - id: cr2-engine-renamed
    text: "KnowledgeChildEngine renamed to ChildEngine throughout src/. src/child/internal/knowledge_child.rs renamed to child.rs. Bindgen path and world strings updated. No logic changes."
    checked: true

  - id: cr3-trait-renamed
    text: "KnowledgeChild trait (defined in mother/src/runtime.rs, re-exported through src/child/runtime.rs into src/mother/mod.rs:46) renamed to Child. All implementors and call sites updated (broker, checkin, daemon, runtime). Deprecated alias added at src/mother/mod.rs: `#[deprecated(since = \"0.46.0\", note = \"use Child\")] pub use crate::child::runtime::Child as KnowledgeChild;`"
    checked: true

  - id: cr4-kind-collapsed
    text: "ChildKind::KnowledgeChild variant renamed to Child. FromStr: \"child\" → Child (canonical), \"knowledge-child\" → Child (alias, emits tracing::warn! once per process via OnceLock so migration signal is visible without log spam). \"pipeline\" stays Pipeline. Alias and warn removed in v0.47.0."
    checked: true

  - id: cr5-sdk-renamed
    text: "sdk/patina-sdk/src/knowledge_child.rs renamed to child.rs. KnowledgeChild trait → Child, KnowledgeChildPlugin → ChildPlugin. Re-export aliases added: `pub use child::Child as KnowledgeChild` and `pub use child::ChildPlugin as KnowledgeChildPlugin` marked #[deprecated]. SDK Cargo.toml: child feature added; knowledge-child = [\"child\"] alias added. Generated module path changes from patina::knowledge_child::* to patina::child::* — all internal references in child.rs updated."
    checked: true

  - id: cr6-children-updated
    text: "All 13 child.toml files under children/ (12 named children + template) updated: kind=\"child\". All 13 Cargo.toml files updated: features=[\"child\",...], path=\"../../wit/child\", world=\"child\". The 6 files pointing at nonexistent wit/worlds are fixed here."
    checked: true

  - id: cr7-templates-updated
    text: "resources/templates/child/knowledge-child/ renamed to resources/templates/child/child/. Template files updated. src/child/scaffold.rs updated. patina child init scaffold produces kind=\"child\" and wit/child target."
    checked: true

  - id: cr8-user-facing-drift
    text: "src/main.rs help text updated (\"knowledge-child, pipeline\" → \"child, pipeline\"). AGENTS.md vocabulary section updated. sdk/patina-sdk/README.md updated. All user-visible strings referencing knowledge-child replaced."
    checked: true

  - id: cr9-ci-updated
    text: "resources/git/pre-push-checks.sh SDK_WORLDS updated from (knowledge-child pipeline) to (child pipeline)."
    checked: true

  - id: cr10-compile-proof
    text: "cargo check --workspace -q passes. cargo test -q --lib passes. cargo test -q --tests passes (catches integration/scaffold breakage). WIT drift check passes: diff -r wit/child/ sdk/patina-sdk/wit/child/ is empty."
    checked: true

  - id: cr11-compat-proof
    text: "A dedicated fixture in src/child/internal/tests.rs uses features=[\"knowledge-child\"] and references KnowledgeChild/KnowledgeChildPlugin via the old import path. cargo test -q --lib passes with deprecation warnings and zero errors. Fixture is self-contained and order-independent from cr6."
    checked: true
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
→ `child`. SDK embedded copy renamed from `sdk/patina-sdk/wit/knowledge-child/`
to `sdk/patina-sdk/wit/child/` and updated to match. Verified with
`diff -r wit/child/ sdk/patina-sdk/wit/child/`.

**Engine** (`cr2`): `KnowledgeChildEngine` → `ChildEngine`. File
`src/child/internal/knowledge_child.rs` → `child.rs`. Two string literals
in the bindgen macro change. No logic changes.

**Trait** (`cr3`): `KnowledgeChild` trait in `mother/src/runtime.rs`
(re-exported through `src/child/runtime.rs` → `src/mother/mod.rs:46`) →
`Child`. Deprecated alias at `src/mother/mod.rs`:
`#[deprecated(since = "0.46.0", note = "use Child")] pub use crate::child::runtime::Child as KnowledgeChild;`
so SDK consumers get a compiler warning, not a hard break. All first-party
call sites updated.

**ChildKind enum** (`cr4`): `KnowledgeChild` variant → `Child`. `FromStr`
keeps `"knowledge-child"` as an alias but emits `tracing::warn!` at parse
time so migration progress is observable in logs. Silent aliases hide
drift — the warn makes it visible. Alias and warn removed in v0.47.0.

**SDK** (`cr5`): `sdk/patina-sdk/src/knowledge_child.rs` → `child.rs`.
`KnowledgeChild` trait and `KnowledgeChildPlugin` renamed to `Child` and
`ChildPlugin`. Deprecated type aliases re-exported for source compatibility.
Cargo feature `child` added; `knowledge-child = ["child"]` alias kept until
v0.47.0. Generated module path changes from `patina::knowledge_child::*` to
`patina::child::*` — all references inside the SDK updated.

**Children** (`cr6`): All 13 `child.toml` files use `kind = "child"`. All
13 `Cargo.toml` files updated to `child` SDK feature and `wit/child` target.
The 6 pointing at nonexistent `wit/worlds` are fixed here.

**Templates + scaffold** (`cr7`): Scaffold template dir renamed. `patina
child init` produces `kind = "child"` and `wit/child` target.

**User-facing drift** (`cr8`): `src/main.rs` help text, `AGENTS.md`
vocabulary, SDK README updated. These are the spots agents and users read —
leaving them stale defeats the purpose of the rename.

**CI** (`cr9`): pre-push-checks.sh SDK_WORLDS updated.

## Alias retirement: v0.47.0

Current version: v0.45.8. This spec ships as part of v0.46.x. Aliases
removed in v0.47.0:
- `FromStr` alias `"knowledge-child"` → `Child` (and the `OnceLock` warn)
- SDK Cargo feature alias `knowledge-child = ["child"]`
- Deprecated re-exports `KnowledgeChild`, `KnowledgeChildPlugin`

**Enforcement gate for v0.47.0:** Before the alias removal commit, run:
```
rg "knowledge-child|knowledge_child|KnowledgeChild" \
  src/ sdk/ children/ resources/ \
  --glob '!*.lock' --glob '!target/**' \
  -l | { grep -v "MIGRATION-SHIM" || true; }
```
Must produce no output. Every allowed shim is marked `// MIGRATION-SHIM: remove in v0.47.0`.
Any unmarked hit is a blocker.

v0.47.0 breaking change noted in CHANGELOG at that time.

## What does not change

- `PipelineEngine` — untouched
- `wit/pipeline/` — untouched
- Grammar plugin discovery and `~/.patina/pipeline/` path — untouched
- Engine logic — zero changes, only names and strings
- Behavior — none intended

## Risks

- `KnowledgeChild` trait rename crosses 5+ files. Read all call sites before
  editing. Deprecated re-export at re-export site bridges external consumers.
- WIT package rename changes generated Rust module path from
  `patina::knowledge_child::*` to `patina::child::*`. All references inside
  `sdk/patina-sdk/src/child.rs` must be updated to match.
- The 6 `wit/worlds` children have never compiled against real WIT. Verify
  they actually compile after the path fix.
- WIT dual-copy drift: root `wit/child/` and SDK embedded copy must stay
  identical. Enforced by diff check in cr1 and cr10.
