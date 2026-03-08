---
type: belief
id: code-is-not-core
persona: architect
facets: [architecture, protocol, plugins, domain-agnostic]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-04
revised: 2026-03-04
---

# code-is-not-core

Code analysis (tree-sitter parsing, language grammars, AST traversal, call graphs) is a plugin capability, not Patina core. Not every Patina project has code. A law firm, a CRM, a game AI system — none of these need Rust syntax knowledge. Code grammars are plugins that a development-focused project installs. Patina core has no domain-specific code — no Rust knowledge, no GitHub API knowledge, no email parsing.

## Statement

Code analysis (tree-sitter parsing, language grammars, AST traversal, call graphs) is a plugin capability, not Patina core. Not every Patina project has code. A law firm, a CRM, a game AI system — none of these need Rust syntax knowledge. Code grammars are plugins that a development-focused project installs. Patina core has no domain-specific code — no Rust knowledge, no GitHub API knowledge, no email parsing.

## Evidence

- [[session-20260303-190855]]: Walked through 9 project scenarios (law firm, medical research, architecture firm, investment fund, journalism, manufacturing QA, education, real estate). Code parsing only relevant to scenario 1 (software dev). Text is universal (9/9), PDFs (8/9), APIs (8/9). Code is niche. (weight: 0.9)
- [[session-20260303-190855]]: User: "scrape code is NOT core — it's a capability added when a project needs code analysis. Like adding a grammar plugin." (weight: 0.85)

## Supports

- [[patina-is-domain-agnostic-knowledge-system]] — Domain-agnostic means no domain code in core. Code analysis is the software development domain.
- [[scrape-is-local-capture]] — Scrape captures from git (text, markdown, beliefs). Code parsing is one way to index that captured data, provided by a grammar plugin.
- [[patina-is-knowledge-protocol]] — Protocol verbs don't mention code. capture/index/search/believe/evolve work on any domain.

## Attacks

- "Patina IS a development tool" — Scoped: Patina started as a dev tool and its current user base is developers. But the identity has evolved. Development is one application of the belief+action pattern, not the identity.

## Attacked-By

- "Removing code analysis from core breaks existing users" — Valid transition concern. Counter: code grammars already exist as pipeline plugins. The extraction path is proven. Users install the code grammar plugin and nothing changes for them.
- "Code analysis IS core for dogfooding" — Valid. Patina-the-project needs code analysis. Counter: Patina-the-project installs code grammar plugins like any other project would. Eating your own cooking means using your plugin system.

## Applied-In

- Pipeline plugins already exist for Rust, Python, etc. — the extraction pattern is partially proven
- grammar-forge already processes forge staging files as a pipeline plugin

## Revision Log

- 2026-03-04: Created — metrics computed by `patina scrape`
