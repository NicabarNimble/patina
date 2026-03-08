---
type: belief
id: plugin-is-agent-plus-skill
persona: architect
facets: [plugins, skills, architecture, llm]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-13
---

# plugin-is-agent-plus-skill

A Patina plugin is a bundle of agent (WASM) + skill (prompt template) + manifest (capabilities) — the agent is the sensor/executor, the skill is the LLM playbook, and the manifest declares trust boundaries — enabling Obsidian-level accessibility with WASM-level safety in an LLM-authored plugin ecosystem.

## Statement

A Patina plugin is a bundle of agent (WASM) + skill (prompt template) + manifest (capabilities) — the agent is the sensor/executor, the skill is the LLM playbook, and the manifest declares trust boundaries — enabling Obsidian-level accessibility with WASM-level safety in an LLM-authored plugin ecosystem.

## Evidence

- [[session-20260213-055346]]: Comparative analysis of Zed (WASM safe, no LLM story), Obsidian (low barrier, no safety), and Patina design space — identified that LLMs eliminate the Rust+WASM barrier, shifting design from human-writability to LLM-generability + user-installability + sandbox safety (weight: 0.8)
- [[session-20260213-055346]]: 10-scenario walkthrough stress-tested world/zone alignment — calling convention is the real distinction between worlds. Four execution contracts emerged: pipeline (host-invoked pure compute), command (user-invoked intelligence), task (user-invoked action with toys), mother-child (daemon continuous action). Each justified by scenarios that break in the others. (weight: 0.9)

## Supports

- [[separate-worlds-for-isolation]] — Worlds define what the agent half of a bundle can do
- [[two-layer-capability-grants]] — Manifest gates what gets approved at install time
- [[patina-is-knowledge-layer]] — Plugins extend the knowledge system, skills teach the LLM how to use it
- [[skills-for-structured-output]] — Skills already proven as LLM-facing API pattern
- [[mother-is-the-daemon]] — Mother children are the agent half; daemon provides the heartbeat lifecycle

## Attacks

- "Plugins should just be code, keep LLM concerns separate" — Defeated: the LLM is already the primary consumer of Patina's knowledge. Ignoring it in plugin design creates the Zed gap (safe but no LLM story).

## Attacked-By

- "Skills are adapter-specific (Claude vs Gemini), plugins are universal" — Valid tension. Bundle may need adapter-agnostic skill definitions or per-adapter skill variants within a single bundle.
- "Three-part bundles are over-engineered for simple plugins" — Valid for pipeline plugins (swap an embedding model) that need no LLM interaction. Not all bundles require all three parts — agent-only and skill-only bundles are valid subsets.
- [[wit-is-contract-wasm-is-one-runtime]] — The WIT-contract-with-roles model is the universal plugin architecture. Connectors (I/O), grammars (pure compute), and apps have no LLM interaction and no "skill" component. The agent+skill bundle is one pattern within the broader system, not the universal plugin model.

## Scope

This belief applies to **command and task plugins** that interact with LLMs — where the agent half executes logic and the skill half teaches the LLM how to use it. It does NOT apply to connectors (pure I/O, no LLM), grammars (pure compute, no LLM), or apps (their own deployments). The universal plugin model is WIT contracts with roles ([[wit-is-contract-wasm-is-one-runtime]]); this belief describes one role's internal structure.

## Applied-In

- [[plugin-ecosystem]] — Design spec for four-world model, three-zone taxonomy, and bundle concept

## Revision Log

- 2026-02-13: Created — metrics computed by `patina scrape`
- 2026-02-13: Revised — 10-scenario walkthrough validated 4-world model (pipeline, command, task, mother-child). Added evidence for calling-convention distinction. Pipeline defined as pure compute (no host imports beyond log) — side effects pushed into host.
- 2026-03-04: Scoped — applies to command/task plugins only, not universal plugin model. WIT-contract-with-roles is the broader architecture.
