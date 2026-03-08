---
type: belief
id: role-validation-should-have-strict-mode
persona: architect
facets: [plugins, validation, developer-experience]
entrenchment: low
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# role-validation-should-have-strict-mode

Role-world validation currently warns but doesn't block — doctor should surface these warnings, and a --strict mode should be able to promote them to errors for production deployments.

## Statement

Role-world validation currently warns but doesn't block — doctor should surface these warnings, and a --strict mode should be able to promote them to errors for production deployments.

## Evidence

- [[session-20260305-132827]]: Kelley advisory review: warning-not-blocking on invalid role-world combos accumulates tech debt as warnings nobody reads. Current warn-not-block is correct (grammars might need HTTP), but long-term need a path to strictness via doctor checks or --strict flag. (weight: 0.7)

## Supports

- [[role-is-immutable-per-version]] — stricter validation reinforces immutability guarantees

## Attacks

## Attacked-By

- Current DESIGN.md explicitly chose warn-not-block. Strict mode would be opt-in, not default.

## Applied-In

- `src/plugin/internal/mother_child.rs` — `check_capabilities()` uses `eprintln!` for role-world mismatch. Future: doctor could surface via `patina doctor` checks.

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
