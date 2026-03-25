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
  - id: pvr1-policy-lock-committed
    text: Bucket policy table (A-H) is locked in this spec, including explicit no-physical-move scope for Bucket C and historical freeze for Bucket G
    checked: false
  - id: pvr2-reaudit-recorded
    text: INVENTORY.md is refreshed with current counts and representative files (no references to removed paths)
    checked: false
  - id: pvr3-gates-have-proofs
    text: PVR gate plan (G0-G7) is present with per-gate entry/exit proofs and deterministic verification commands
    checked: false
  - id: pvr4-guard-defined
    text: Anti-regression guard scope and command are specified for active surfaces, explicitly excluding historical/session archives
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

Counts from current audit (2026-03-25 refresh, post PVR-G3):

- `src/`: 249 matches
- `resources/`: 33 matches
- top-level active docs: 5 matches
- `layer/surface/`: 658 matches
- `layer/sessions/`: 1109 matches

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

Scope lock:

- This bucket is topology/reference normalization only (script/doc/path wording).
- Do **not** physically move or consolidate `plugins/` into `children/` in this spec.
- Any physical `plugins/` -> `children/` consolidation requires a separate spec.

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

## Locked Bucket Policies (PVR-G0)

These policies are locked before execution. If any policy changes, amend this spec first.

| Bucket | Policy | Rationale |
|--------|--------|-----------|
| A - Runtime compatibility command/API surfaces | rename now | Active user-facing CLI/path vocabulary should be child-first; keep compatibility aliases during migration window. |
| B - Grammar pipeline naming | compat keep | In pipeline context, "plugin" is currently a domain term for host-invoked WASM grammar processors; keep and document explicitly. |
| C - Filesystem/workspace topology naming | rename now (scope-limited) | Normalize script/doc references and path wording only; no physical directory move in this spec. |
| D - Scaffolding and template paths | rename now | Move scaffolding/template vocabulary to child-first names. Do not delete legacy paths until callsites are migrated and proven unused. |
| E - Active user-facing docs and guidance | rename now | Primary docs should present canonical child-first vocabulary with explicit compatibility notes where needed. |
| F - Living architecture/spec docs (`layer/surface`) | selective | Update only living/canonical doctrine text; preserve historical analysis and completed records as history. |
| G - Session archives (`layer/sessions`) | archive freeze | Historical evidence; never rewrite. |
| H - External runtime artifact names (`~/.patina/*`) | compat alias | Keep existing paths working via compatibility alias while documenting canonical child-first targets and sunset trigger. |

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

Policy lock is now established in this spec under "Locked Bucket Policies (PVR-G0)".

Recommended next operator behavior:

- Treat this as bucketed RFC execution (A/B/C/E/F), not line-by-line grep churn.
- Land one bucket per commit slice with explicit rollback notes.
- Keep G frozen and add forward-looking clarification notes instead of rewriting history.

## Verification

## Gate Plan (PVR-G0 through PVR-G7)

### PVR-G0: Lock policy and gate contract in spec

- Entry: spec in draft with bucket inventory present.
- Work: lock bucket policy table (A-H), add gate sequence, add proof commands.
- Exit proofs:
  - `patina spec check plugin-vocabulary-retirement --json` shows criteria present and unchecked (execution not yet started).

### PVR-G1: Re-audit inventory post-greenfield cleanup

- Entry: PVR-G0 complete.
- Work: refresh `INVENTORY.md` counts and representative files; remove references to deleted paths.
- Exit proofs:
  - `rg -n "\bplugin(s)?\b|plugin\.toml|plugins/|Plugin[A-Z]" src | wc -l` matches inventory `src` count.
  - `rg -n "\bplugin(s)?\b|plugin\.toml|plugins/|Plugin[A-Z]" resources | wc -l` matches inventory `resources` count.

### PVR-G2: Bucket D templates/scaffolding rename

- Entry: PVR-G1 complete.
- Work: migrate scaffold/template naming to child-first terminology; migrate `resources/templates/plugin/mother-child` to child-first location/name if still active; delete legacy template paths only after callsites are migrated.
- Exit proofs:
  - `rg -n "resources/templates/plugin" src/child/scaffold.rs` returns zero.
  - `cargo build -q` succeeds.

### PVR-G3: Bucket A CLI/path compatibility surface

- Entry: PVR-G2 complete.
- Work: child-first command/help text and path wording; maintain explicit compatibility alias behavior where needed.
- Exit proofs:
  - `patina --help` shows child-first canonical wording for this surface.
  - `cargo test -q -- src/child` succeeds.

### PVR-G4: Bucket C topology/reference normalization

- Entry: PVR-G3 complete.
- Work: update script/doc/path references from plugin-era wording where in-scope; no physical `plugins/` -> `children/` move.
- Exit proofs:
  - `rg -n "plugins/models|plugins/repos" resources scripts Cargo.toml README.md` returns zero in active surfaces.
  - `cargo check --workspace` succeeds.

### PVR-G5: Bucket E active docs cleanup

- Entry: PVR-G4 complete.
- Work: update active user-facing docs to child-first wording; keep explicit compatibility notes where required.
- Exit proofs:
  - `rg -n "\bplugin(s)?\b" README.md OPENCODE.md children/README.md plugins/README.md` only returns approved compatibility notes.

### PVR-G6: Bucket F selective doctrine updates

- Entry: PVR-G5 complete.
- Work: update only living/canonical doctrine/spec text under `layer/surface`; preserve completed/historical analysis and all session archives.
- Exit proofs:
  - `rg -n "\bplugin(s)?\b" layer/surface/build/refactor/plugin-vocabulary-retirement` reflects policy-locked language and historical carve-outs.

### PVR-G7: Anti-regression guard + final verification

- Entry: PVR-G6 complete.
- Work: add deterministic guard command/script for active surfaces, excluding frozen history.
- Exit proofs:
  - `bash resources/scripts/check-plugin-vocab-guard.sh` passes on clean tree.
  - `cargo check --workspace && cargo test -q` succeeds.

## Final Verification Commands

- `cargo check --workspace`
- `cargo test -q`
- targeted command checks (`patina --help`, canonical child command help + compatibility alias behavior)
- `bash resources/scripts/check-plugin-vocab-guard.sh`

## Build Readiness

Not execution-ready yet. Ready only after bucket-level policy decisions are explicitly approved and sequenced into low-risk slices.
