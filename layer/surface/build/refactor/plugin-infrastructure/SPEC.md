---
type: refactor
id: plugin-infrastructure
status: draft
created: 2026-03-04
sessions:
  origin: 20260304-120702
beliefs:
- wit-is-contract-wasm-is-one-runtime
- patina-is-knowledge-protocol
- reads-via-host-writes-via-intents
exit_criteria:
- id: children-complete
  text: All child specs (host-emit-wit, plugin-roles) are complete
  checked: false
---
# refactor: Plugin Infrastructure — Host Emit and Roles

> Foundation spec. Plugins can emit facts, declare roles, and use an
> updated SDK. Everything else depends on this.

## Context

This is a **container spec** — it tracks two child specs that together
build the plugin infrastructure needed for core extraction and Mother
maturation.

**Architecture context:**
- [[session-20260303-190855]] — forge audit revealed plugin system gaps
- [[session-20260304-120702]] — refined into 3-stream spec structure
- [[wit-is-contract-wasm-is-one-runtime]] — WIT defines contracts, WASM
  is one runtime. The plugin system should not couple interface to execution.
- [[patina-is-knowledge-protocol]] — protocol verbs are the core. Everything
  else extends via plugins.
- [[reads-via-host-writes-via-intents]] — plugins read via host calls,
  write via intents. host_emit is the missing write path for facts.

**Current plugin system (read these for code context):**
- `src/plugin/internal/mod.rs` — PluginManifest, PluginProvides, capability gating
- `src/plugin/internal/host_support.rs` — host function implementations
- `src/plugin/internal/mother_child.rs` — WASM runtime, WasmChild adapter
- `wit/deps/patina-host/host.wit` — host interfaces (NO emit interface)
- `wit/mother-child/mother-child.wit` — mother-child world definition

**Key gaps identified:**
- No `host_emit` — plugins can read (scry/assay/context) but cannot write
  facts to the eventlog
- No role metadata — manifests declare capabilities but not purpose
  (connector vs grammar vs extension vs app)
- Schema interface defined in host.wit but NOT implemented in any world
- SDK does not reflect these new capabilities

## Children

| Spec | What it delivers | Build order |
|------|-----------------|-------------|
| [[host-emit-wit]] | Plugins can write facts to eventlog | First |
| [[plugin-roles]] | Role metadata in manifests, role-based dispatch | Second (or parallel) |

## Implementation Prerequisites

Resolve before or during implementation of child specs:

- **Schema-install mechanism.** When a connector plugin ships a schema,
  how does it get installed? Does `patina plugin install` copy to
  `.patina/schemas/`? Or does the host resolve schemas from plugin
  directories? Blocks [[spec-forge-plugin-extraction]] EC3
  (schema-ships-with-plugin). See forge-plugin-extraction SPEC.md
  Open Questions.

- **Projection table ownership.** After extraction, who creates
  materialized views (e.g., `forge_issues`, `forge_prs`) from emitted
  events? Options: (a) scrape creates them when it sees matching events,
  (b) schema.toml declares projections and scrape materializes them
  generically. Option (b) is more extensible. See forge-plugin-extraction
  SPEC.md Open Questions.

## Exit Criteria

This spec is complete when both children are complete.
