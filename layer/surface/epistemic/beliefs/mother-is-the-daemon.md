---
type: belief
id: mother-is-the-daemon
persona: architect
facets: [architecture, naming, daemon]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-04
revised: 2026-02-04
---

# mother-is-the-daemon

Mother is the always-running daemon — serve, graph, cache, and cross-project routing are all facets of mother, not separate concepts. Unify under 'patina mother' with subcommands (start, stop, status, graph) following the Docker/Ollama daemon model.

## Statement

Mother is the always-running daemon — serve, graph, cache, and cross-project routing are all facets of mother, not separate concepts. Unify under 'patina mother' with subcommands (start, stop, status, graph) following the Docker/Ollama daemon model.

## Evidence

- [[session-20260204-193822]]: [[20260204-193822]] - Discussion comparing Docker/Ollama daemon patterns to current patina serve/mother split. 'serve' help text already says 'Start the Mother daemon' — the naming confusion proves they're the same thing. (weight: 0.9)

## Supports

- [[patina-is-knowledge-layer]] — mother as daemon is the runtime manifestation of the knowledge layer
- [[transport-security-by-trust-boundary]] — UDS-first daemon model aligns with trust boundary design

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Complexity: one binary doing daemon + CLI + MCP is a lot of modes. Separate binaries (patina, patina-motherd) would be clearer process boundaries.

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-02-04: Created — metrics computed by `patina scrape`
