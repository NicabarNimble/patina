# Design: Scrape strategy seam exploration

## Why This Design

CV11 is a seam-definition problem, not a blind extraction task. This lane separates exploration and parity planning from runtime mutation so future extraction can be deliberate and reversible.

## Build Target

Produce an execution-ready seam contract and parity packet for non-core scrape lanes while preserving core scrape behavior.

## Execution Slices

1. Inventory current scrape lanes and boundaries with file:line evidence.
2. Define strategy contract (`ScrapeStrategy` boundary: capability inputs, output schema, error semantics, telemetry hooks).
3. Design parity harness and acceptance metrics for code/git/grammar-backed lanes.
4. Produce decision packet and follow-on spec skeleton (implement or defer).

## Rules

- No extraction writes in this spec lane.
- No CV11 status claim without runnable parity commands.
- Keep layer/beliefs scrape as core baseline.
- Any childization proposal must include rollback and compatibility bridge policy.

## Verification

- `cargo check -q`
- `cargo test -q`
- lane inventory command outputs captured in spec notes
- parity harness commands defined with expected success/failure signatures

## Build Readiness

- [ ] Inventory complete (core vs non-core lanes)
- [ ] Seam contract documented
- [ ] Parity harness plan documented
- [ ] Decision packet drafted
