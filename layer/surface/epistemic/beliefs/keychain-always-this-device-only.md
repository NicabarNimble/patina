---
type: belief
id: keychain-always-this-device-only
persona: architect
facets: [security, secrets, macos, keychain]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-18
revised: 2026-02-18
---

# keychain-always-this-device-only

Secrets stored in macOS Keychain for headless/SSH access should use kSecAttrAccessibleAlwaysThisDeviceOnly — hardware-encrypted in the Secure Enclave, accessible without user presence, device-bound so they cannot be exfiltrated via iCloud sync or backup restore

## Statement

Secrets stored in macOS Keychain for headless/SSH access should use kSecAttrAccessibleAlwaysThisDeviceOnly — hardware-encrypted in the Secure Enclave, accessible without user presence, device-bound so they cannot be exfiltrated via iCloud sync or backup restore

## Evidence

- [[session-20260218-225007]]: [[session-20260218-225007]] - SSH from Tailscale/Termius failed because WhenUnlocked policy requires an active GUI session; AlwaysThisDeviceOnly fixes it without any plaintext fallback (weight: 0.95)

## Supports

- [[transport-security-by-trust-boundary]] — device-bound hardware encryption is the trust boundary for a stationary Mac

## Attacks

- Storing `PATINA_IDENTITY` (age private key) in `~/.zshenv` to unblock SSH — this is plaintext on disk and defeats the vault's security model
- Using plaintext files with `chmod 600` for long-lived tokens — same threat model as above, just more obvious

## Attacked-By

- "Use `AfterFirstUnlock` instead — same device binding, doesn't use deprecated API" — **partially true** but fails after reboot until first login; for a headless Mac Studio this is the wrong tradeoff
- "The deprecation warning means we shouldn't use it" — **defeated**: deprecated on iOS (stolen phone threat model), correct for stationary Mac (device = auth factor); still works on macOS and present in `security-framework-sys`
- "AlwaysThisDeviceOnly breaks machine migration" — **acknowledged but acceptable**: export/import CLI (`patina secrets --export-key`) still works; the restriction is on automatic transfer (iCloud, Migration Assistant), not manual export

## Applied-In

- `src/secrets/keychain.rs` — `store_identity()` uses `SecAccessControlCreateWithFlags` with `kSecAttrAccessibleAlwaysThisDeviceOnly` directly from `security_framework_sys`, bypassing the `ProtectionMode` enum that omits it
- `src/commands/secrets.rs` — `setup_claude()` re-stores identity after token save, migrating existing WhenUnlocked items in one command

## Revision Log

- 2026-02-18: Created — metrics computed by `patina scrape`
