---
type: belief
id: scrape-is-local-capture
persona: architect
facets: [architecture, scrape, protocol, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-04
revised: 2026-03-04
---

# scrape-is-local-capture

Scrape reads what's inside the project (git). External data comes through connectors independently, not through scrape dispatch. Both write to the eventlog, both feed the belief network, but they are separate concerns. Scrape code (tree-sitter, language grammars) is a plugin capability, not protocol core — not every Patina project has code.

## Statement

Scrape reads what's inside the project (git). External data comes through connectors independently, not through scrape dispatch. Both write to the eventlog, both feed the belief network, but they are separate concerns. Scrape code (tree-sitter, language grammars) is a plugin capability, not protocol core — not every Patina project has code.

## Evidence

- [[session-20260303-190855]]: User: scrape becomes local capture from git ONLY. External data comes through plugins independently. Scrape code is NOT core. Arrived at through 9 project scenarios — a law firm has no code, a CRM has no git commits to parse. (weight: 0.9)
- [[session-20260304-120702]]: Decomposed current scrape by protocol verb: capture from git (read what changed), index from captured data (parse into facts), capture from external (connectors, NOT scrape). Scrape = capture + index for local sources. (weight: 0.85)

## Supports

- [[patina-is-domain-agnostic-knowledge-system]] — If scrape doesn't assume code, Patina doesn't assume development. Domain-agnostic identity requires scrape to be domain-agnostic.
- [[patina-is-knowledge-protocol]] — Protocol verbs map to commands. Scrape = capture + index. External capture is a different command/plugin, not scrape with extra dispatch.
- [[patina-is-beliefs-plus-action]] — Both scrape and connectors feed the same belief network. They're different sources of evidence, same destination.

## Attacks

- "source-kind dispatch in scrape" — Defeated: the forge-plugin-extraction spec had EC3 (scrape dispatches to plugins by source kind). This belief says that's wrong. Scrape is local, connectors are external. Different commands, different lifecycles.

## Attacked-By

- "Scrape is the user's mental model for 'get data into Patina'" — Valid. Users run `patina scrape` and expect everything to update. Counter: `patina scrape` can trigger connectors as a convenience, but the architecture treats them as separate operations. Like `git pull` is fetch + merge.
- "Current scrape does forge AND code AND layer — splitting breaks workflows" — Valid transition concern. Counter: the split is architectural, the CLI can still offer `patina scrape --all` as sugar.

## Applied-In

- Current scrape mod.rs: already has separate dispatch paths for git, code, layer, forge, beliefs. The separation exists in code, just not in architecture.

## Revision Log

- 2026-03-04: Created — metrics computed by `patina scrape`
