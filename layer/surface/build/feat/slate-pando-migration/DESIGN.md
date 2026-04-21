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

Initial contract can be a typed envelope with explicit routing mode and structured response, then tighten into per-command typed functions once parity fixtures are stable.

## Slice 1 (start now)

1. Activate spec and index state.
2. Freeze parity fixtures for existing `patina spec` command outputs (JSON-first).
3. Add Slate backend routing seam in spec dispatch payload (non-breaking, ignored when unused).
4. Add observe-mode plumbing hooks (no side effects yet).

## Safety

- Fail closed on missing policy/grants.
- Keep builtin spec path callable at all times until parity criteria are fully checked.
- Do not change CI topology in this slice.
