---
type: feat
id: e2ee-multimother-chat
status: draft
created: 2026-03-27
parent: child-construction-canon
sessions:
  origin: 20260327-104954-066673000
blocked_by:
  - multiproject-belief-share
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[observation-at-the-boundary]]"
  - "[[wasi-is-foundation-not-option]]"
related:
  - sdk/patina-sdk/
  - children/
  - wit/knowledge-child/
  - layer/surface/build/feat/child-construction-canon/
exit_criteria:
  - id: emc1-children-reused
    text: "Blocks reused from prior MVPs without modification: event-router, encryption-envelope, record-writer, schema-enforcer."
    checked: false
  - id: emc2-reuse-failures-documented
    text: "Any child that required modification has the failure documented and the child adjusted."
    checked: false
  - id: emc3-p2p-children-built
    text: "2 new P2P children built: message-relay, notification-emitter."
    checked: false
  - id: emc4-multi-mother-trust
    text: "Trust model for cross-Mother authority — how peers authenticate and what grants they honor."
    checked: false
  - id: emc5-e2ee-working
    text: "End-to-end encrypted messages delivered between two Mother instances."
    checked: false
  - id: emc6-registry-complete
    text: "Full registry of 11+ reusable children proven across pipeline, federation, and P2P domains."
    checked: false
  - id: emc7-recipe-validated
    text: "Objective recipe filled in with concrete values including trust_model and encryption_requirements."
    checked: false
---
# feat: e2ee-multimother-chat

## Problem

The children registry must hold across all three domains. This is the final proof — P2P communication between Mother instances, the hardest domain for authority boundaries and observation. It reuses children from both prior MVPs and builds the last 2 new children.

## Goal

End-to-end encrypted messaging across multiple Mother instances connected via iroh. In doing so, prove the full registry works cross-domain and build the P2P children.

The critical test: **can children from MVP 1 and MVP 2 compose into a P2P system without modification?** This is the hardest reuse challenge — the children were built for pipelines and federation, now they must work for real-time communication.

## Non-Goals

- Building a production chat application.
- Group chat or multi-party key exchange in this phase.
- Designing iroh transport internals (infrastructure concern).

## Blocks Reused from Prior MVPs

| Child | Built in | Used for | Expected modification |
|---|---|---|---|
| `event-router` | MVP 2 | Route messages between local children and relay | None |
| `encryption-envelope` | MVP 2 | E2EE message encryption/decryption | None |
| `record-writer` | MVP 1 | Persist chat history to parquet | None |
| `schema-enforcer` | MVP 1 | Validate message schema | None |

## New Children Built

### 10. `message-relay`

**Capability:** Relay messages between peers/Mothers via network transport.

**Toys:** `patina:events-stream`, `wasi:messaging/producer`, `wasi:http`, `patina:connect` (for peer binding), `wasi:logging`

**How it works:** Subscribes to outgoing message events. Resolves peer binding via `patina:connect`. Sends encrypted message payloads to remote Mother via `wasi:http` (or iroh transport when available). Receives incoming messages from remote peers and publishes to local event streams.

**Reuse:** any cross-Mother communication, distributed game state, federated notifications.

### 11. `notification-emitter`

**Capability:** Send alerts/notifications based on event patterns.

**Toys:** `patina:events-stream`, `wasi:http`, `patina:connect`, `wasi:logging`

**How it works:** Subscribes to configurable event patterns. When pattern matches, sends notification via configured channel (HTTP webhook, push notification). Manifest configures: event patterns, notification targets, rate limiting.

**Reuse:** monitoring alerts, game events, audit notifications, deployment status.

## Composition

```
[local Mother]
user input → event-router
    → [message.outgoing] →
encryption-envelope (encrypt)
    → [message.encrypted] →
message-relay (send to remote peer)
    → ... network (iroh/HTTP) ... →

[remote Mother]
message-relay (receive from peer)
    → [message.received] →
encryption-envelope (decrypt)
    → [message.decrypted] →
event-router (route to display + archive)
    → [message.display] → (UI)
    → [message.archive] →
schema-enforcer → record-writer → lakehouse-catalog
    (chat history persisted as parquet)
```

## Unknowns (resolved during build)

| Unknown | When we'll hit it | What breaks if wrong |
|---|---|---|
| Children from MVPs 1+2 compose into a P2P system | First attempt to wire event-router + encryption-envelope with message-relay | Reuse thesis at its hardest — pipeline/federation children in a real-time context |
| Multi-Mother authority model is viable | Building message-relay and defining peer trust | May need new hard rules or Mother-to-Mother protocol that doesn't exist yet |
| iroh integration works from WASM children | Building message-relay | Falls back to HTTP relay through known servers instead of P2P |
| Observation works across Mother boundaries | Testing cross-Mother message flow | Each Mother observes her own children, but inter-Mother link may be a blind spot |

## Open Design Questions

These will be resolved during build, not before:

- How does Mother authority (hard rule 1) work across peer boundaries? Each Mother is sovereign — what's the trust handshake?
- iroh vs HTTP for transport? Resolved by what works when we build message-relay.
- Cross-Mother observation? Resolved when we hit it.
- `patina:connect` binding for peer auth vs iroh's own auth? Resolved during build.

## Acceptance Gates

- 4+ children reused from prior MVPs without modification. *(registry validation)*
- E2EE message delivered between two Mother instances. *(integration test)*
- Chat history persisted to parquet via reused record-writer + lakehouse-catalog. *(composition test)*
- Mother-tier metrics work for all children including relay. *(observation test)*

## Verification

```bash
patina spec check e2ee-multimother-chat --json
cargo check --workspace -q
cargo test -q --workspace
```

## Build Readiness

Blocked by `multiproject-belief-share`. Federation children (event-router, encryption-envelope) must exist before this MVP can prove P2P reuse.
