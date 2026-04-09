---
type: belief
id: voice-keypair-is-node-identity
persona: architect
facets: [architecture, identity, persona, crypto]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-04-09
---

# voice-keypair-is-node-identity

The voice keypair is voice identity — signing beliefs, issuing UCAN capability tokens, and proving authorship. It is NOT node/machine identity. Mother (the machine node) has its own identity for P2P peer discovery. Two levels: Mother-keypair identifies the machine, voice-keypair identifies the knowledge context. A voice keypair can exist on multiple Mothers (same voice, multiple machines). A Mother hosts multiple voice keypairs (multiple contexts, one machine).

## Statement

The voice keypair is voice identity — signing beliefs, issuing UCAN capability tokens, and proving authorship. It is NOT node/machine identity. Mother (the machine node) has its own identity for P2P peer discovery. Two levels: Mother-keypair identifies the machine, voice-keypair identifies the knowledge context. A voice keypair can exist on multiple Mothers (same voice, multiple machines). A Mother hosts multiple voice keypairs (multiple contexts, one machine).

## Evidence

- [[session-20260305-224446]]: [[session-20260305-224446]] - Connected persona-federation spec (UID, registry, links) with pipe architecture (fact signing, network sync, capability tokens). One keypair, three uses: provenance, p2p identity, auth delegation (weight: 0.9)

## Supports

- [[persona-is-a-patina-instance]] — scoped predecessor model that introduced identity separation now captured as voice namespace
- [[mother-is-connection-and-continuity]] — Mother federates voices; keypair enables authenticated federation
- [[five-boundaries-no-overlap]] — voice identity and Mother node identity remain separate role boundaries.

## Attacks

- Attacks the current model where voice is a dead string field ("architect") — replaces with cryptographic identity

## Attacked-By

- "Multiple keypairs for different concerns" — signing, network, and auth could use separate keys for isolation. Counter: one identity simplifies key management, and derivation paths can separate concerns cryptographically

## Applied-In

- [[spec-persona-federation]] — identity registry with UIDs, to gain keypair identity
- [[spec-pipe-architecture]] — fact provenance via voice signature, capability delegation via UCAN

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
- 2026-03-21: Revised — separated persona identity from node identity. Persona keypair signs beliefs and issues UCANs. Mother keypair identifies the machine for P2P. Two levels, not one conflated key. Prompted by [[session-20260320-212325-011658000]] discussion of Mother = machine node, persona = crypto namespace.
- 2026-04-09: Revised — Era 3 identity terminology corrected from persona to voice. Belief id and narrative updated to match voice namespace vocabulary while preserving the two-level identity model.
