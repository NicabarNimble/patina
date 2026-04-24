# Allium Layer

This is the canonical home for Allium behavioral specs.

## Role in Patina

- Allium defines behavioral contracts (`.allium`).
- Code/tests implement and verify those contracts.
- Allium is not a runtime.

## Structure

- `layer/allium/mother/` — Mother behavioral specs and `*.plan.json` obligations.

## Minimal integration policy

- Keep artifacts lean: specs + plans.
- Run `allium check` and `allium analyse` in CI.
- Keep obligation IDs in Rust tests (resume-style).
- Use coverage docs when useful; avoid heavy artifact sprawl.
