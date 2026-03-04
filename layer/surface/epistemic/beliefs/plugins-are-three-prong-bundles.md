---
type: belief
id: plugins-are-three-prong-bundles
persona: architect
facets: [architecture, plugins, wasm, mcp, adapters]
entrenchment: medium
status: defeated
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# plugins-are-three-prong-bundles

A plugin is not just code — it is a CLI + MCP + Skill bundle that brings a complete capability to Patina, making capabilities installable, replaceable, and self-describing

## Statement

A plugin is not just code — it is a CLI + MCP + Skill bundle that brings a complete capability to Patina, making capabilities installable, replaceable, and self-describing

## Evidence

- [[session-20260223-120524]]: [[session-20260223-120524]] - Spec-workflow-rigor analysis revealed that spec commands need all three layers: CLI for deterministic execution, MCP for programmatic LLM access, adapter skills for teaching LLMs when and how to act. The pattern generalizes to session, release, doctor, beliefs — all fit the same 3-prong shape. Core Patina is the knowledge pipeline + plugin host; everything else is plugins bringing their own bundle. (weight: 0.9)

## Supports

- [[unix-philosophy]] — one plugin, one capability, done well. Plugins decompose Patina into focused tools.
- [[specs-orthogonal-to-sessions]] — plugins enforce the separation: spec plugin owns work lifecycle, session plugin owns time lifecycle
- [[mutation-completes-query]] — the 3-prong bundle ensures every query (MCP) has a corresponding mutation (CLI) and the LLM knows when to use it (skill)

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- WASM sandbox limitations — plugins can't call git/filesystem directly, must go through host functions. Host function surface must be rich enough for complex plugins like spec.
- [[wit-is-contract-wasm-is-one-runtime]] — The WIT-contract-with-roles model has 4 plugin types (connector, grammar, extension, app). Grammars are pure compute (no CLI/MCP/skill). Connectors are I/O (no skill). Apps are their own deployments. Most plugins never interact with an LLM.
- [[mcp-is-shim-cli-is-product]] — MCP is Patina infrastructure that wraps CLI, not something plugin authors bundle. The "MCP prong" is not a plugin concern.

## Defeated

- **Date**: 2026-03-04
- **Reason**: Assumed all plugins are LLM-facing bundles that need CLI + MCP + Skill delivery. The domain-agnostic plugin architecture (wit-is-contract-wasm-is-one-runtime) has roles where most plugins (grammars, connectors, apps) never interact with an LLM. MCP is infrastructure, not a plugin shape. The CLI/MCP/Skill delivery surface is Patina's concern, not the plugin's.

## Applied-In

- [[spec-workflow-rigor]] — spec system designed with 3-prong shape: 12 CLI commands, matching MCP tools, single `/spec` skill for LLM discovery. Architecture anticipates extraction to WASM plugin.
- `plugins/doctor` — already a workspace plugin crate, validates the plugin extraction pattern
- Session system — `patina session {start,update,note,end}` CLI + adapter skills (`/session-start`, `/session-end`) already follows 2 of 3 prongs. Adding MCP completes the pattern.

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
- 2026-03-04: Defeated — most plugin roles are not LLM-facing. MCP/Skill delivery is infrastructure, not plugin shape.
