---
type: belief
id: world-boundary-is-type-safety
persona: architect
facets: [architecture, plugin-system, wasm, wit]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-12
revised: 2026-02-12
---

# world-boundary-is-type-safety

In a separate-worlds-per-plugin-type architecture, the WIT world boundary is where type safety lives — capability isolation determines what a plugin can see, and string dispatch within a world is intentional low coupling.

## Statement

In a separate-worlds-per-plugin-type architecture, the WIT world boundary is where type safety lives — capability isolation determines what a plugin can see, and string dispatch within a world is intentional low coupling.

## Evidence

- [[session-20260212-093831]]: [[session-20260212-093831]] - Patina uses separate worlds (mother-child, command, oracle, scraper, grammar) each with different imports; Zed uses single world with typed dispatch; Patina's handle(string, string) is deliberate — world boundary provides isolation, not payload types; per [[coupling-is-complexity]] typed payloads couple WIT definition to child implementation (weight: 0.9)

## Supports

- [[separate-worlds-for-isolation]] — each plugin type gets its own WIT world with only the imports it needs
- [[coupling-is-complexity]] — string dispatch avoids coupling WIT definitions to child implementations
- [[two-layer-capability-grants]] — world boundary enforces first layer (what's importable), manifest enforces second

## Attacks

- Per-child typed WIT variants — would require per-child worlds (one world per child instance), adding Linker complexity for marginal type safety gain within the boundary

## Attacked-By

- "JSON-RPC over WASM" criticism — we have wasmtime's type system but use string/string at the interface. Counter: the type safety lives at the world boundary (capability isolation), not the payload boundary.

## Applied-In

- `wit/mother-child.wit` — `handle(action: string, payload: string)` is the canonical example: models and repos children share one world, negotiate payloads by JSON convention
- [[plugin-system]] spec — 6 worlds planned (mother-child, command, oracle, scraper, forge-reader, grammar), each with different imports, all using string dispatch within
- Contrasted with Zed's `world extension` — single world, all capabilities, typed dispatch. Different trade-off: flexibility vs isolation

## Revision Log

- 2026-02-12: Created — metrics computed by `patina scrape`
