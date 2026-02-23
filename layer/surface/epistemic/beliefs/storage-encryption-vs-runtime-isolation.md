---
type: belief
id: storage-encryption-vs-runtime-isolation
persona: architect
facets: [security, architecture, secrets]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-22
revised: 2026-02-22
---

# storage-encryption-vs-runtime-isolation

At-rest encryption and runtime isolation are fundamentally different threat models — encrypted storage solves the first, process boundaries solve the second.

## Statement

At-rest encryption and runtime isolation are fundamentally different threat models — encrypted storage solves the first, process boundaries solve the second.

## Evidence

- [[session-20260222-165738]]: Discovered contradiction: patina encrypts secrets at rest (vault.age, identity.enc) but `run_with_secrets` injects ALL secrets as plaintext env vars into LLM process. LLM runs `env` and sees everything. (weight: 0.95)
- IronClaw (`nearai/ironclaw`) WASM boundary protects tool-level secrets via host HTTP injection, but LLM's own API key still delivered as env var to container. Their proxy admits HTTPS CONNECT can't inject credentials (src/sandbox/proxy/http.rs:246-248). (weight: 0.9)
- Cloudflare Workers: `env.MY_SECRET` returns plaintext to consuming worker. Isolation is between workers, not from the worker itself. (weight: 0.8)
- [[spec-secrets-dual-storage]]: Spec addresses at-rest threat model (encrypted file, machine-binding) but runtime injection was out of scope. (weight: 0.7)

## Supports

- [[transport-security-by-trust-boundary]]
- [[llm-threat-model-unique]]

## Attacks

<!-- This belief challenges the implicit assumption that encrypting secrets "protects from LLM" — encryption protects at rest, but the LLM runs at runtime where secrets must be decrypted for use -->

## Attacked-By

- "A local auth proxy with MITM CA could hide credentials from HTTPS consumers" — technically possible but breaks TLS guarantees, invasive

## Applied-In

- `src/secrets/encrypted_file.rs`: At-rest protection (ChaCha20-Poly1305, machine-bound)
- `src/secrets/mod.rs::run_with_secrets()`: Runtime injection gap (all secrets as env vars)
- `wit/deps/patina-host/host.wit:102`: "credential injection" at WASM boundary — runtime isolation for plugins (designed but not wired to vault)

## Revision Log

- 2026-02-22: Created — metrics computed by `patina scrape`
