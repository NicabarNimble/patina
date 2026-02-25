---
type: belief
id: plugins-are-three-prong-bundles
persona: architect
facets: [architecture, plugins, wasm, mcp, adapters]
entrenchment: medium
status: active
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

## Applied-In

- [[spec-workflow-rigor]] — spec system designed with 3-prong shape: 12 CLI commands, matching MCP tools, single `/spec` skill for LLM discovery. Architecture anticipates extraction to WASM plugin.
- `plugins/doctor` — already a workspace plugin crate, validates the plugin extraction pattern
- Session system — `patina session {start,update,note,end}` CLI + adapter skills (`/session-start`, `/session-end`) already follows 2 of 3 prongs. Adding MCP completes the pattern.

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
