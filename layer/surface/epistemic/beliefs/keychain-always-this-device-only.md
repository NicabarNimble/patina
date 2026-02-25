---
type: belief
id: keychain-always-this-device-only
persona: architect
facets: [security, secrets, macos, keychain]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-18
revised: 2026-02-20
---

# keychain-always-this-device-only

Secrets stored in macOS Keychain for headless/SSH access should use kSecAttrAccessibleAlwaysThisDeviceOnly — hardware-encrypted in the Secure Enclave, accessible without user presence, device-bound so they cannot be exfiltrated via iCloud sync or backup restore

## Statement

Secrets stored in macOS Keychain for headless/SSH access should use kSecAttrAccessibleAlwaysThisDeviceOnly — hardware-encrypted in the Secure Enclave, accessible without user presence, device-bound so they cannot be exfiltrated via iCloud sync or backup restore

## Evidence

- [[session-20260218-225007]]: SSH from Tailscale/Termius failed because WhenUnlocked policy requires an active GUI session; AlwaysThisDeviceOnly fixes it without any plaintext fallback (weight: 0.95)
- [[session-20260220-120045]]: AfterFirstUnlockThisDeviceOnly broke launcher token injection over SSH — new sessions showed login prompt because keychain was inaccessible before GUI login. Switching back to AlwaysThisDeviceOnly fixed it. (weight: 0.95)
- [[spec-keychain-macos26-regression]]: Deprecation is iOS-specific (stolen phone threat model). Stationary Mac Studio is not a stolen phone. AlwaysThisDeviceOnly is correct for headless server use case. (weight: 0.9)

## Supports

- [[transport-security-by-trust-boundary]] — device-bound hardware encryption is the trust boundary for a stationary Mac

## Attacks

- Storing `PATINA_IDENTITY` (age private key) in `~/.zshenv` to unblock SSH — this is plaintext on disk and defeats the vault's security model
- Using plaintext files with `chmod 600` for long-lived tokens — same threat model as above, just more obvious

## Attacked-By

- "Use `AfterFirstUnlock` instead — same device binding, doesn't use deprecated API" — **defeated by session-20260220-120045**: tried this on Feb 19, broke SSH launcher token injection. GUI login dependency defeats the entire purpose of spec-secrets-keychain-ssh.
- "The deprecation warning means we shouldn't use it" — **defeated**: deprecated on iOS (stolen phone threat model), correct for stationary Mac (device = auth factor); still works on macOS and present in `security-framework-sys`
- "AlwaysThisDeviceOnly breaks machine migration" — **acknowledged but acceptable**: export/import CLI (`patina secrets --export-key`) still works; the restriction is on automatic transfer (iCloud, Migration Assistant), not manual export

## Applied-In

- `src/secrets/keychain.rs` — `store_identity()` uses raw `SecItemAdd` with `kSecAttrAccessibleAlwaysThisDeviceOnly` directly, bypassing the broken `SecAccessControlCreateWithFlags` API on macOS 26
- `src/commands/secrets.rs` — `setup_claude()` re-stores identity after token save, migrating existing items to current policy
- `layer/surface/build/fix/spec-secrets-keychain-ssh/SPEC.md` — original specification (Feb 18, used SecAccessControlCreateWithFlags)
- `layer/surface/build/fix/spec-keychain-macos26-regression/SPEC.md` — current specification (Feb 20, uses raw SecItemAdd)

## Revision Log

- 2026-02-18: Created — metrics computed by `patina scrape`
- 2026-02-20: Revised — updated implementation method (raw SecItemAdd), added session-20260220-120045 evidence, defeated AfterFirstUnlock attack
