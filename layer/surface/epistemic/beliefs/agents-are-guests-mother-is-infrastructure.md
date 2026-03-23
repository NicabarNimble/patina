---
type: belief
id: agents-are-guests-mother-is-infrastructure
persona: architect
facets: [architecture, agents, mother, children, toys, identity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-21
revised: 2026-03-21
---

# agents-are-guests-mother-is-infrastructure

Patina is infrastructure for agents, not an agent itself. Mother is the daemon that manages children and toys. Agents (Claude Code, OpenCode, pi, Gemini CLI) are guests that connect to Mother directly — no adapters, no protocol bridges. Children are composable workers with bounded agency and toys. The belief system is the core value that persists across agents, interfaces, and sessions.

## Statement

Patina is infrastructure for agents, not an agent itself. Mother is the daemon that manages children and toys. Agents (Claude Code, OpenCode, pi, Gemini CLI) are guests that connect to Mother directly — no adapters, no protocol bridges. Children are composable workers with bounded agency and toys. The belief system is the core value that persists across agents, interfaces, and sessions.

## Evidence

- [[session-20260320-212325-011658000]]: Deep session dive traced the convergence — interfaces and children follow the same pattern (bounded capabilities, Mother manages lifecycle). MCP adapters are bridges that became permanent. Pi-mono reference repo proves agents can connect without MCP. (weight: 0.95)

## Supports

- [[mother-is-the-daemon]] — Mother as always-running daemon is the infrastructure foundation
- [[children-have-agency-toys-are-capabilities]] — children and toys are the composable units Mother manages
- [[patina-is-knowledge-layer]] — belief system as the core value that persists
- [[durability-lives-outside-interface-process]] — interfaces are guests, durability lives in Mother's children
- [[initialize-is-capability-grant]] — Mother grants capabilities at connection time, not via protocol negotiation
- [[bridges-become-permanent]] — MCP adapter is the bridge this belief retires

## Attacks

- [[mcp-is-discovery-cli-is-execution]] — the MCP-vs-CLI distinction dissolves; both are guest agents connecting to Mother
- [[mcp-is-shim-cli-is-product]] — reframes: neither CLI nor MCP is "the product"; Mother and the belief system are the product

## Attacked-By

- External agents (Claude Code, OpenCode) have their own runtime constraints — they can't speak WIT natively, so some connection protocol is still needed between external processes and Mother
- MCP is a de facto standard for LLM tool discovery — abandoning it may reduce compatibility with future LLM tooling

## Applied-In

- [[spec-composable-toy-sdk]] — per-child WIT worlds, toys as composable components, github as toy not child
- [[spec-interface-session-model]] — session child spawned for any agent that connects, interface-agnostic

## Revision Log

- 2026-03-21: Created — metrics computed by `patina scrape`
