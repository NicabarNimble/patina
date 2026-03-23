# Design: Close remaining code-to-vision seams

## Why This Design

This lane isolates unresolved architecture seams from the completed migration slices,
so closure work can be intentional, testable, and reversible.

## Build Target

Close or explicitly lock CV1, CV2, and CV11 with parity-backed decisions.

## Execution Slices

1. CV1 seam ownership audit (Mother runtime centralization).
2. CV2 CLI boundary audit (runtime code vs transport/client seams).
3. CV11 scrape seam contract + parity gate definition.
4. Closure decision pass: implement, or mark permanent seam with proof.

## Rules

- No status flips without command/file evidence.
- Any ownership move must have rollback instructions.
- If a seam remains permanent, rationale and boundary contract must be explicit.

## Verification

- `cargo check -q`
- `cargo test -q`
- Mother status/routing checks for affected surfaces
- scrape parity proof command(s) with recorded key lines

## Build Readiness

- [ ] CV1 evidence complete
- [ ] CV2 evidence complete
- [ ] CV11 evidence complete
- [ ] truth map updated in both this spec and parent spec references
