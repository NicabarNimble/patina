---
type: belief
id: role-belongs-on-granted-capabilities
persona: architect
facets: [plugins, architecture, performance]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# role-belongs-on-granted-capabilities

PluginRole should be cached on GrantedCapabilities for O(1) runtime access when Mother routes by role — add when continuous-operation or mother-maturation needs it.

## Statement

PluginRole should be cached on GrantedCapabilities for O(1) runtime access when Mother routes by role — add when continuous-operation or mother-maturation needs it.

## Evidence

- [[session-20260305-132827]]: Gjengset advisory review: role lives only on PluginManifest, not GrantedCapabilities. When Mother dispatches by role (spec-continuous-operation), re-parsing manifests is wrong. Cache like host_emit is cached. (weight: 0.8)

## Supports

- [[gjengset-lens-type-integrity]] — cache at load time, not runtime

## Attacks

## Attacked-By

- [[enum-earns-keep-when-dispatched]] — if the enum isn't matched in production, caching it is premature too

## Applied-In

- `src/plugin/internal/mod.rs` — `GrantedCapabilities` already caches `host_emit: bool` and `schema_facts` at load time. `role` would follow the same pattern when needed.

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
