---
type: belief
id: raw-keychain-over-access-control
persona: architect
facets: [macos, secrets, keychain, security]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-20
revised: 2026-02-20
---

# raw-keychain-over-access-control

Use kSecAttrAccessible directly via SecItemAdd for Keychain storage, never SecAccessControlCreateWithFlags — the AccessControl API fails with -34018 on macOS 26 for ad-hoc signed binaries and is overengineered for non-biometric use cases.

## Statement

Use kSecAttrAccessible directly via SecItemAdd for Keychain storage, never SecAccessControlCreateWithFlags — the AccessControl API fails with -34018 on macOS 26 for ad-hoc signed binaries and is overengineered for non-biometric use cases.

## Evidence

- [[session-20260219-083531]]: SecAccessControlCreateWithFlags returns -34018 on macOS 26 (Tahoe) for all policies when binary is ad-hoc/linker-signed. kSecAttrAccessible set directly via SecItemAdd works for all policies. Confirmed with Swift test harness. (weight: 0.95)
- [[session-20260218-225007]]: Previous fix used SecAccessControlCreateWithFlags with AlwaysThisDeviceOnly — worked on older macOS but broke on Tahoe upgrade. (weight: 0.8)

## Supports

- [[compiler-enforced-safety]]: Raw API is simpler and more explicit than wrapper abstractions that hide failure modes
- [[transport-security-by-trust-boundary]]: Keychain policy choice (AfterFirstUnlockThisDeviceOnly) aligns with trust boundary design — device is the auth factor

## Attacks

- [[keychain-always-this-device-only]]: Supersedes this belief — AlwaysThisDeviceOnly policy replaced by AfterFirstUnlockThisDeviceOnly, and SecAccessControlCreateWithFlags replaced by raw SecItemAdd

## Attacked-By

- Code signing would fix -34018: If patina were properly code-signed (Apple Developer ID), SecAccessControlCreateWithFlags would likely work. Counter: ad-hoc signing is correct for `cargo install` distribution model. (status: defeated)
- Upstream crate fix: If `security-framework` added `set_accessible()` to PasswordOptions, we wouldn't need raw API. Counter: we can't control upstream timeline, and raw SecItemAdd is simpler anyway. (status: defeated)

## Applied-In

- `src/secrets/keychain.rs:store_identity()` — raw SecItemAdd with kSecAttrAccessible + AfterFirstUnlockThisDeviceOnly

## Revision Log

- 2026-02-20: Created — metrics computed by `patina scrape`
