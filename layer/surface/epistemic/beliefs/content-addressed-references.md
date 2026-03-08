---
type: belief
id: content-addressed-references
persona: architect
facets: [architecture, data-model, cryptography, future-proofing]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-04
revised: 2026-03-04
---

# content-addressed-references

References between events, evidence, and beliefs should be content-addressed (hashes, event IDs) rather than path-based (file paths, line numbers). Content-addressed references are stable across renames, portable across systems, and enable cryptographic verification. This keeps the ZK proof and E2EE federation paths open without requiring those systems now.

## Statement

References between events, evidence, and beliefs should be content-addressed (hashes, event IDs) rather than path-based (file paths, line numbers). Content-addressed references are stable across renames, portable across systems, and enable cryptographic verification. This keeps the ZK proof and E2EE federation paths open without requiring those systems now.

## Evidence

- [[session-20260304-120702]]: User's blockchain background (Giza, Starknet STWO, ZK circuits, Signal-like E2EE) identified that content-addressed references are prerequisite for cryptographic verification of belief grounding chains. Design for this now, build it later. (weight: 0.85)
- [[session-20260304-120702]]: ZK proof scenario: "I hold a belief grounded in 3 pieces of evidence. Here's a proof the evidence exists and supports the claim. You can't see the evidence." This requires content-addressed evidence references, not file paths. (weight: 0.8)
- [[session-20260304-120702]]: Append-only eventlog is structurally similar to a local chain. Add merkle roots = tamper evidence. Add signatures = provenance proofs. Content-addressed references are the prerequisite. (weight: 0.8)

## Supports

- [[patina-is-beliefs-plus-action]] — Verifiable beliefs are more actionable than trust-based beliefs. Content addressing enables verification.
- [[events-are-autobiography-not-telemetry]] — If events are the project's autobiography, they should be tamper-evident. Content-addressed references enable this.
- [[local-first-edge-deployable]] — Content-addressed references are portable across local and edge deployments. File paths are not.

## Attacks

<!-- None yet — this is a new direction -->

## Attacked-By

- "Premature optimization for crypto" — Valid. ZK proofs and E2EE are far-future features. Counter: content-addressed references are independently useful (stable across renames, portable) without any crypto. The crypto path is a bonus.
- "Current wikilinks work fine" — Valid. `[[session-20260304-120702]]` is a human-readable reference. Counter: wikilinks can coexist with content hashes. The wikilink is the human name, the hash is the machine reference.

## Applied-In

- Event IDs (seq numbers) in events.db are already content-independent references
- Git commit SHAs are content-addressed by construction — beliefs referencing commits already use this pattern

## Revision Log

- 2026-03-04: Created — metrics computed by `patina scrape`
