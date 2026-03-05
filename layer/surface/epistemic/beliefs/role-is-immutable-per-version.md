---
type: belief
id: role-is-immutable-per-version
persona: architect
facets: [plugins, architecture, invariants]
entrenchment: high
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# role-is-immutable-per-version

A plugin's role must not change at runtime — it is parsed from TOML at load time with no mutation API, making role an immutable association on the plugin entity, not mutable state.

## Statement

A plugin's role must not change at runtime — it is parsed from TOML at load time with no mutation API, making role an immutable association on the plugin entity, not mutable state.

## Evidence

- [[session-20260305-132827]]: Helland advisory review: role as immutable metadata is architecturally sound per entity/association model. Current design enforces correctly — parsed from plugin.toml, stored on PluginManifest, no setter. Preserve this invariant as system evolves. (weight: 0.9)

## Supports

- [[wit-is-contract-wasm-is-one-runtime]] — role is part of the contract, not runtime state

## Attacks

## Attacked-By

- Future hot-reload scenarios might want role changes without restart. But role changes are version changes — deploy a new version, don't mutate the running one.

## Applied-In

- `src/plugin/internal/mod.rs` — `PluginManifest.role` is `Option<PluginRole>`, parsed in `from_path()`, no setter method. `PluginRole` has no `&mut self` methods.

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
