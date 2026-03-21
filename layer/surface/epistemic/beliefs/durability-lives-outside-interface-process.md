---
type: belief
id: durability-lives-outside-interface-process
persona: architect
facets: [sessions, architecture, mother-child-toy]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-20
revised: 2026-03-20
---

# durability-lives-outside-interface-process

Session artifact durability must live outside the interface process because the interface can die at any time — a WASM child spawned at Mother handshake owns artifact persistence and crash recovery

## Statement

Session artifact durability must live outside the interface process because the interface can die at any time — a WASM child spawned at Mother handshake owns artifact persistence and crash recovery

## Evidence

- [[session-20260318-221008-061837000]] - proposed ChildType::Interface for durable artifact holding after repeated tmux lane deaths; [[session-20260320-075256-088035000]] - formalized as WASM child interface helper in spec-interface-session-model (weight: 0.9)

## Supports

- [[universal-artifact-interface-specific-enrichment]] — the child is the mechanism that enables interface-specific enrichment of the universal artifact
- [[tmux-lane-defines-active-session]] — lane death is exactly the failure mode this belief addresses

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Complexity cost: adding a WASM child per interface session is significant infrastructure. Could a simpler periodic-write model achieve 80% of the durability?

## Applied-In

- [[spec-interface-session-model]] — Thread 4: WASM child interface helper design

## Revision Log

- 2026-03-20: Created — metrics computed by `patina scrape`
