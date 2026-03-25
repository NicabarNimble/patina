---
type: refactor
id: plugin-vocabulary-retirement
status: draft
created: 2026-03-25
related:
  - src/main.rs
  - src/paths.rs
  - src/child/scaffold.rs
  - resources/templates/plugin/
  - resources/grammar-defaults.toml
  - Cargo.toml
  - README.md
  - OPENCODE.md
exit_criteria:
  - id: inventory-buckets-accepted
    text: Plugin vocabulary inventory is grouped into logical buckets and approved with explicit keep/rename/archive policy per bucket
    checked: false
  - id: compatibility-policy-locked
    text: Compatibility surfaces that may retain plugin wording are explicitly documented with rationale and sunset trigger
    checked: false
  - id: active-surface-vocabulary-clean
    text: Active runtime and primary docs use child-first vocabulary except explicitly approved compatibility surfaces
    checked: false
  - id: anti-regression-guard
    text: A deterministic check exists to prevent accidental reintroduction of plugin-era wording in active surfaces
    checked: false
---
# refactor: retire plugin vocabulary from active Patina surfaces

> Remove plugin-era language from active runtime/docs while preserving intentional compatibility and historical evidence.

## Problem

The architecture is child-first, but plugin-era wording still appears across runtime command surfaces, path contracts, scripts, docs, and historical artifacts. This creates identity drift and makes it unclear which references are intentional compatibility versus stale vocabulary.

## Goal

Define and execute a controlled vocabulary retirement plan: keep required compatibility points explicit, migrate active surfaces to child wording, and preserve historical records without rewriting history.

## Non-Goals

- Do not rewrite archived session artifacts.
- Do not rewrite historical spec/audit records that are evidence of past states.
- Do not perform a risky big-bang folder move without explicit approval.

## Inventory Buckets (grouped)

Counts from current audit:

- `src/`: 297 matches
- `resources/`: 27 matches
- top-level active docs: 6 matches
- `layer/surface/`: 625 matches
- `layer/sessions/`: 1085 matches

### Bucket A - Runtime compatibility command/API surfaces

Representative files:

- `src/main.rs` (Plugin command surface and user messaging)
- `src/paths.rs` (`plugins_dir`, `plugin-config`, legacy `paths::plugin` alias)
- `src/child/scaffold.rs` (`patina plugin init` text and template paths)

### Bucket B - Grammar pipeline naming

Representative files:

- `src/commands/setup/grammars.rs`
- `src/commands/scrape/code/extract.rs`
- `src/child/internal/pipeline.rs`
- `src/commands/bench/grammar.rs`
- `resources/grammar-defaults.toml`
- `resources/scripts/grammar-compare.sh`

### Bucket C - Filesystem and workspace topology naming

Representative files:

- `Cargo.toml` (workspace members under `plugins/*`)
- `resources/git/pre-push-checks.sh`
- `resources/scripts/check-crate-names.sh`
- `resources/scripts/check-single-sdk-surface.sh`

### Bucket D - Scaffolding and template paths

Representative paths:

- `resources/templates/plugin/*`
- references to those templates in `src/child/scaffold.rs`

### Bucket E - Active user-facing docs and guidance

Representative files:

- `README.md`
- `OPENCODE.md`
- `sdk/patina-sdk/README.md`
- `children/README.md`
- `plugins/README.md`

### Bucket F - Living architecture/spec docs (`layer/surface`)

Includes active beliefs/specs/design docs where wording may be normative today. Must be triaged into:

- still-canonical doctrine text to rename
- historical rationale that should stay unchanged with framing note

### Bucket G - Session archives (`layer/sessions`)

Historical evidence and prompts. Default policy is preserve-as-is.

### Bucket H - External runtime artifact names (`~/.patina/*`)

Legacy on-disk names referenced by docs/session handoffs (for example older `plugin.*` artifacts). Requires explicit migration policy to avoid user breakage.

## Decision Matrix (requires user direction)

For each bucket: choose `rename now`, `compat keep`, `archive freeze`, or `defer`.

## Session Direction (2026-03-25)

- Bucket A hard-cut was prototyped, then rolled back intentionally to protect concurrent work; no accidental drift from that rollback should be reintroduced.
- Bucket A is still approved in principle, but execution remains deferred until an explicit migration window is chosen.
- Bucket G (`layer/sessions`) is locked as historical record: never rewrite history.
- Bucket F (`layer/surface`) should be treated as mixed: update only living/canonical doctrine, preserve historical analysis as historical.
- Bucket D (templates/scaffolding) is likely tied to ongoing interface redesign; deprecation/migration to child naming is preferred over keeping plugin paths.
- Buckets B/C/E remain pending explicit execution decisions after deeper review.

## Execution Boundary

Do not start broad implementation from this spec until bucket policy is locked. This is an editorial/compatibility migration, not a pure architectural deletion slice.

Required policy lock before execution:

- Bucket B: choose full rename vs hybrid grammar-lane strategy.
- Bucket C: choose path-only wording cleanup vs physical topology move.
- Bucket E: choose strict active-doc cleanup level.
- Bucket F: choose exact scope of "living doctrine" edits vs historical freeze boundaries.

Recommended next operator behavior:

- Treat this as bucketed RFC execution (A/B/C/E/F), not line-by-line grep churn.
- Land one bucket per commit slice with explicit rollback notes.
- Keep G frozen and add forward-looking clarification notes instead of rewriting history.

## Verification

- `cargo check --workspace`
- `cargo test -q`
- targeted command checks (`patina --help`, `patina plugin --help` or replacement)
- deterministic vocabulary guard script for active surfaces

## Build Readiness

Not execution-ready yet. Ready only after bucket-level policy decisions are explicitly approved and sequenced into low-risk slices.
