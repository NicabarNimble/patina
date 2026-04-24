# DESIGN — slate-pando-migration

## Intent

Implement Slate as the new full-WIT execution backend for spec workflows, while keeping `patina spec` as the stable user-facing command during migration.

This design locks **parity-first** execution:

1. match current `spec` behavior,
2. route through Slate in `observe` mode,
3. enable `execute` mode only after fixture parity.

## Architecture boundary

- **CLI (`patina spec`)**: unchanged user contract.
- **Mother**: routing authority + policy/grants.
- **Slate child**: typed execution of spec operations.
- **SDK (`sdk/patina-sdk`)**: child authoring ergonomics (toy helpers + shared types), not release policy authority.

## Proposed control modes

- `off` — current builtin flow only.
- `observe` — run Slate plan/render path and emit parity diff metadata; no side effects.
- `execute` — Slate performs side effects (PR/release actions) under toy grants.

## Child contract direction (first cut)

Add a Slate WIT contract that avoids untyped `handle(action,payload)` and exposes typed command dispatch.

Contract now includes per-command typed functions (`list-specs`, `next-specs`, `check/show/prompt/handoff/packet-spec`, `complete-spec`, `archive-spec`) while keeping `dispatch(command-json)` as transitional compatibility during migration.

## Slice progress

### Completed

1. Spec activated and indexed.
2. Backend routing seam added to spec dispatch envelopes (`off|observe|execute`).
3. Observe mode probe added (builtin result + Slate probe metadata).
4. Execute mode wired to typed Slate call path with fail-closed behavior.
   - Mother routes known `spec` commands through per-command typed operations.
   - Strict behavior: scaffold/not-implemented execute payload is treated as an error (no builtin fallback).
5. Manifest opt-in added: `.patina/manifest.toml` `[spec] mode = ...`.

### Next

1. Read-only parity in Slate child (`list/next/show/check/packet`) with fixture locks.
   - `list/next/show/check/prompt/handoff/packet` implemented via filesystem/frontmatter/design parsing in Slate child.
   - Observe-mode fixture diff harness now enforces deterministic builtin==slate-probe equality over read-only command set.
   - Execute payload contracts are being normalized to builtin response shapes to avoid child-only output drift.
2. Expand `patina:git` contract for mutate/release parity.
   - Started: added `create-tag-at`, `status-porcelain`, `add-paths`, `is-clean-tracked`,
     `commits-behind-upstream`, `is-diverged` to WIT + host bindings.
   - Extended with `remove-paths` for tracked deletion/archive workflows.
3. Execute mutate parity (`complete`/`archive`) in Slate.
   - In progress: git-backed archive path is implemented and `complete` now executes Cargo version-bump release flow for `feat`/`fix`/`refactor` and `--major`.
   - Remaining parity: tighten safeguards/output equivalence and fixture-lock behavior for all complete/archive variants.
3. Move git toy toward foldered multi-file WIT package layout (WASI-like structure) with tooling support.

## Safety

- Fail closed on missing policy/grants.
- Keep builtin spec path callable at all times until parity criteria are fully checked.
- Do not change CI topology in this slice.
