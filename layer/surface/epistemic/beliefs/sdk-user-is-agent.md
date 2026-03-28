---
type: belief
id: sdk-user-is-agent
persona: architect
facets: [sdk, developer-experience, agents, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-26
revised: 2026-03-26
---

# sdk-user-is-agent

The SDK's primary user is an LLM agent, not a human developer. The SDK surface must be legible to any competent agent — clear traits, one right way per pattern, types as contracts, code as documentation. A user says what they need; their agent reads the SDK and generates a working child.

## Statement

The SDK's primary user is an LLM agent, not a human developer. The SDK surface must be legible to any competent agent — clear traits, one right way per pattern, types as contracts, code as documentation. A user says what they need; their agent reads the SDK and generates a working child.

## Evidence

- [[session-20260326-165149-931909000]]: Observed that in practice, children are already built by LLM agents (Claude, OpenCode) reading SDK source in sessions. No human hand-writes connectors in 2026. The agent reads the trait, generates the implementation, tests it. The SDK's design should optimize for this workflow. (weight: 0.95)
- [[session-20260326-165149-931909000]]: Patina's competitive position is "knowledge substrate any agent can extend" — not locked to Anthropic, OpenAI, or any vendor. Any agent that can read a trait and generate Rust can build a Patina child. The protocol is the constant; agents come and go. (weight: 0.90)

## Supports

- [[signal-over-noise]] — an agent-legible SDK is maximum signal: clear types, no ambiguity, no multiple paths to the same outcome
- [[patina-is-knowledge-layer]] — Patina is substrate, not tool. Agents are the tools that use it. SDK enables agents to extend it.

## Attacks

<!-- none yet -->

## Attacked-By

- "What about human developers who don't use agents?" — They still benefit from a clean, unambiguous SDK. Agent-legibility is a superset of human-legibility. One right way, clear types, minimal boilerplate works for everyone.
- "Agent capabilities change fast — optimizing for current agents is premature." — The optimization is for legibility, not for specific agent features. Clear code, clear contracts, minimal concepts. This is durable regardless of agent evolution.

## Applied-In

- Design direction for SDK child factory: kind-specific templates, trait-per-kind, Ctx-based toy access, macro-generated boilerplate — all optimized for an agent generating a working child from the SDK source alone.

## Revision Log

- 2026-03-26: Created in [[session-20260326-165149-931909000]] — emerged from greenfield child architecture discussion when asking "who is the SDK user?"
