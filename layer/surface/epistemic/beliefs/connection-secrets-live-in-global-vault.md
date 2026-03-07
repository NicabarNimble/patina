---
type: belief
id: connection-secrets-live-in-global-vault
persona: architect
facets: [security, architecture, secrets]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-07
revised: 2026-03-07
---

# connection-secrets-live-in-global-vault

Credentials referenced by ~/.patina/connections/ configs must be stored in the global vault (patina secrets add --global), not the project vault — the broker calls get_global_secret() which only reads the global vault.

## Statement

Credentials referenced by ~/.patina/connections/ configs must be stored in the global vault (patina secrets add --global), not the project vault — the broker calls get_global_secret() which only reads the global vault.

## Evidence

- [[session-20260307-165002]]: [[session-20260307-165002]] - Test setup for broker EC verification failed silently when credential was added to project vault. get_global_secret() returned None. Fixed by using patina secrets add --global. Connection configs are global (~/.patina/connections/), so their credentials must also be global. (weight: 0.9)

## Supports

- [[host-proxied-io-is-the-security-model]] — Mother decrypts credentials from vault, children never access vault directly

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/broker/mod.rs:run_source()` — calls `secrets::get_global_secret()` for connection credentials
- `src/broker/connection.rs` — `credential` field in `[connection]` section maps to a global vault key
- `~/.patina/connections/*.toml` — global config, so secrets must match scope

## Revision Log

- 2026-03-07: Created — metrics computed by `patina scrape`
