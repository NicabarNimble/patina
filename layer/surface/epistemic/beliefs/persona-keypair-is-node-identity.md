---
type: belief
id: persona-keypair-is-node-identity
persona: architect
facets: [architecture, identity, persona, crypto]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# persona-keypair-is-node-identity

The persona UID and keypair serve three roles — signing key for fact provenance, node identity for Iroh peer discovery, and UCAN issuer for capability token delegation — making persona-federation and pipe-architecture share the same identity primitive

## Statement

The persona UID and keypair serve three roles — signing key for fact provenance, node identity for Iroh peer discovery, and UCAN issuer for capability token delegation — making persona-federation and pipe-architecture share the same identity primitive

## Evidence

- [[session-20260305-224446]]: [[session-20260305-224446]] - Connected persona-federation spec (UID, registry, links) with pipe architecture (fact signing, network sync, capability tokens). One keypair, three uses: provenance, p2p identity, auth delegation (weight: 0.9)

## Supports

- [[persona-is-a-patina-instance]] — persona as sovereign instance needs a cryptographic identity, not just a string label
- [[mother-is-connection-and-continuity]] — Mother federates personas; keypair enables authenticated federation
- [[host-proxied-io-is-the-security-model]] — UCAN capability tokens derived from persona keypair scope pipe access

## Attacks

- Attacks the current model where persona is a dead string field ("architect") — replaces with cryptographic identity

## Attacked-By

- "Multiple keypairs for different concerns" — signing, network, and auth could use separate keys for isolation. Counter: one identity simplifies key management, and derivation paths can separate concerns cryptographically

## Applied-In

- [[spec-persona-federation]] — persona registry with UIDs, to gain keypair identity
- [[spec-pipe-architecture]] — fact provenance via persona signature, capability delegation via UCAN

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
