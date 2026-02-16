---
type: belief
id: root-communicates-identity
persona: architect
facets: [architecture, workspace, identity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-15
revised: 2026-02-15
---

# root-communicates-identity

The project root directory should communicate what Patina IS (protocol engine, knowledge product, interface contract) — not list every grammar plugin and deprecated API crate.

## Statement

The project root directory should communicate what Patina IS (protocol engine, knowledge product, interface contract) — not list every grammar plugin and deprecated API crate.

## Evidence

- [[session-20260215-204444]]: Exploration found 26 root dirs, 18 of which are plugin/crate sprawl that buries src/, layer/, wit/ (weight: 0.9)
- [[session-20260215-204444]]: [[patina-identity]] says "The binary is the pipeline. The layer is the product. The protocol is the contract." — root should reflect this trinity, not 9 grammar plugins (weight: 0.8)
- [[session-20260215-204444]]: [[unix-philosophy]] says "Each component has a single, clear responsibility" — root layout IS the first interface new contributors see (weight: 0.7)
- [[session-20260215-204444]]: [[dependable-rust]] says "Keep your public interface small and stable" — the root IS the public interface of the repo (weight: 0.7)
- [[session-20260215-204444]]: Grammar crates are completely standalone (own Cargo.lock, wasm32 target, crates.io deps) — they CAN live in a subdirectory without breaking builds (weight: 0.8)

## Supports

- [[patina-identity]] — root should communicate protocol identity
- [[dependable-rust]] — small stable interface principle applies to repo layout
- [[unix-philosophy]] — clarity and single responsibility at directory level

## Attacks

- Flat structure makes every crate equally discoverable from root (counter: discoverability ≠ clarity — 18 peer dirs makes NOTHING discoverable)

## Attacked-By

- git history disruption: moving dirs may complicate blame (mitigation: `git mv` preserves history)
- patina-sdk is published to crates.io: relative paths in downstream crates (finding: grammar crates use crates.io version, not path deps — no impact)

## Applied-In

- Proposed spec: `workspace-cleanup` — phased consolidation of root sprawl

## Revision Log

- 2026-02-15: Created — metrics computed by `patina scrape`
