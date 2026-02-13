---
type: belief
id: patina-is-knowledge-protocol
persona: architect
facets: [architecture, identity, plugins, protocol]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-13
---

# patina-is-knowledge-protocol

Patina is a knowledge protocol, not a monolith — like git is to version control, Patina is to development knowledge. The core (capture, index, search, believe, evolve) should work standalone and LLM-agnostic. Everything else — connectors, grammars, exporters, monitors — belongs in the plugin ecosystem. The extraction roadmap is not binary optimization; it is protocol distillation.

## Statement

Patina is a knowledge protocol, not a monolith — like git is to version control, Patina is to development knowledge. The core (capture, index, search, believe, evolve) should work standalone and LLM-agnostic. Everything else — connectors, grammars, exporters, monitors — belongs in the plugin ecosystem. The extraction roadmap is not binary optimization; it is protocol distillation.

## Evidence

- [[session-20260213-055346]]: [[session-20260213-055346]] - Session culmination: after Zed/Obsidian comparative analysis, 10-scenario walkthrough, and 4-world model design, the protocol framing emerged as the unifying principle — Patina should be trimmable to its knowledge core while the plugin system extends it in ways we cannot predict yet (weight: 0.9)

## Supports

- [[patina-is-knowledge-layer]] — Protocol framing sharpens the knowledge-layer identity: the layer is the protocol core, everything else is extension
- [[plugin-is-agent-plus-skill]] — The bundle model is the extension mechanism for the protocol
- [[separate-worlds-for-isolation]] — Worlds are the protocol's extension surface, each with a different calling convention
- [[compiler-enforced-safety]] — Protocol boundary enforced by WASM sandbox + WIT types, not trust

## Attacks

- "Monoliths ship faster" — Defeated: Patina already shipped as monolith (v0.17.0). Protocol distillation is the next phase, not the starting point. Ship monolith, distill protocol.

## Attacked-By

- "Protocol implies specification and stability guarantees" — Valid. If Patina is a protocol, WIT interfaces become versioned contracts. Breaking changes have ecosystem consequences. We accept this cost — it forces good design.
- "Git works without plugins; Patina may need plugins to be useful" — Valid tension. The core must be complete enough to stand alone (scrape, scry, assay, context, beliefs). Plugins extend, they don't complete.

## Applied-In

- [[plugin-ecosystem]] — Design spec built on protocol-first thinking: 4 worlds as extension surface, 3 zones as user taxonomy
- [[plugin-command-extractions]] — Extracting commands to plugins IS protocol distillation
- [[plugin-oracle-scraper]] — Making oracle/scraper extensible IS opening the protocol
- [[plugin-grammars]] — Grammars as pipeline plugins IS decoupling the protocol from specific languages

## Revision Log

- 2026-02-13: Created — metrics computed by `patina scrape`
