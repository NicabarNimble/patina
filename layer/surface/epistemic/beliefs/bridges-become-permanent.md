---
type: belief
id: bridges-become-permanent
persona: architect
facets: [architecture, process, debt]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-04
revised: 2026-02-04
---

# bridges-become-permanent

Temporary bridges become permanent infrastructure unless a retirement spec with an expiry date is created at the same time as the bridge.

## Statement

Temporary bridges become permanent infrastructure unless a retirement spec with an expiry date is created at the same time as the bridge.

## Evidence

- [[session-20260204-110139]]: [[analysis-three-servers.md]] - `--hybrid` flag added Dec 16 2025 as temporary bridge, never retired, became permanent bifurcation for 2 months (weight: 0.95)
- [[session-20251216-130440]]: "CLI vs MCP gap was intentional" — the bridge decision point. No retirement date set. (weight: 0.9)
- [[cli-queryengine-default/SPEC.md]]: Fix required 3 lines of code. The debt was decision debt, not technical complexity. (weight: 0.8)
- Decision debt compounds faster than technical debt — each new feature (persona, beliefs, modes, graph routing) had to be implemented across all diverged paths independently (weight: 0.85)

## Supports

- [[temporal-layering-divergence]] — bridges are how temporal layering creates divergence
- [[mcp-is-shim-cli-is-product]] — the bridge delayed the decision about which path was primary

## Attacks

<!-- None yet -->

## Attacked-By

- Pragmatic conservatism: "Don't break what works while proving the new thing" — valid instinct, but must come with an expiry date

## Applied-In

- `--hybrid` flag on CLI scry (Dec 16, 2025 → Feb 4, 2026) — retired by [[cli-queryengine-default/SPEC.md]]
- Serve daemon `if hybrid` branch in `handle_scry()` — still present, scheduled for D0 cleanup

## Revision Log

- 2026-02-04: Created — metrics computed by `patina scrape`
