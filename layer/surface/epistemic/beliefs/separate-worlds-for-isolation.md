---
type: belief
id: separate-worlds-for-isolation
persona: architect
facets: [wasm, security, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# separate-worlds-for-isolation

Each plugin type gets its own WIT world with only the imports it needs - oracle plugins should not see HTTP imports

## Statement

Each plugin type gets its own WIT world with only the imports it needs - oracle plugins should not see HTTP imports

## Evidence

- [[session-20260205-115835]]: [[session-20260205-115835]] - Diverging from Zed's single-world approach for stricter capability isolation (weight: 0.9)

## Supports

- [[dependable-rust]] — Minimal interfaces reduce attack surface
- [[two-layer-capability-grants]] — World-level + grant-level = defense in depth

## Attacks

- Zed's single-world approach — Trade-off: we choose security over flexibility

## Attacked-By

- "Single world is simpler to maintain" — Valid concern, accepted trade-off
- "Plugins may need cross-capability features" — Addressed by creating new plugin types when needed

## Applied-In

- [[wit-interfaces]] — Separate worlds: oracle-plugin, embedding-plugin, forge-plugin, etc.
- [[patina-platform]] — Core vs plugin split with explicit capability boundaries

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
