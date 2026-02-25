---
type: belief
id: raw-keychain-over-access-control
persona: architect
facets: [macos, secrets, keychain, security]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-20
revised: 2026-02-20
---

# raw-keychain-over-access-control

Use kSecAttrAccessible directly via SecItemAdd for Keychain storage, never SecAccessControlCreateWithFlags — the AccessControl API fails with -34018 on macOS 26 for ad-hoc signed binaries. Combine with kSecAttrAccessibleAlwaysThisDeviceOnly for headless/SSH access (the raw API works with any policy, choose the right one for your threat model).

## Statement

Use kSecAttrAccessible directly via SecItemAdd for Keychain storage, never SecAccessControlCreateWithFlags — the AccessControl API fails with -34018 on macOS 26 for ad-hoc signed binaries. Combine with kSecAttrAccessibleAlwaysThisDeviceOnly for headless/SSH access (the raw API works with any policy, choose the right one for your threat model).

## Evidence

- [[session-20260219-083531]]: SecAccessControlCreateWithFlags returns -34018 on macOS 26 (Tahoe) for all policies when binary is ad-hoc/linker-signed. kSecAttrAccessible set directly via SecItemAdd works for all policies. Confirmed with Swift test harness. (weight: 0.95)
- [[session-20260218-225007]]: Previous fix used SecAccessControlCreateWithFlags with AlwaysThisDeviceOnly — worked on older macOS but broke on Tahoe upgrade. (weight: 0.8)
- [[session-20260220-120045]]: Initial raw SecItemAdd implementation (commit 1cca67ed) incorrectly used AfterFirstUnlockThisDeviceOnly instead of AlwaysThisDeviceOnly, breaking SSH access. Fixed by combining raw API (correct) with AlwaysThisDeviceOnly policy (also correct). (weight: 0.9)

## Supports

- [[compiler-enforced-safety]]: Raw API is simpler and more explicit than wrapper abstractions that hide failure modes
- [[keychain-always-this-device-only]]: Raw SecItemAdd works with AlwaysThisDeviceOnly policy, enabling headless/SSH access on macOS 26
- [[transport-security-by-trust-boundary]]: Device-bound hardware encryption (Secure Enclave) is the trust boundary

## Attacks

None — this is the correct approach for macOS 26 with ad-hoc signing.

## Attacked-By

- Code signing would fix -34018: If patina were properly code-signed (Apple Developer ID), SecAccessControlCreateWithFlags would likely work. Counter: ad-hoc signing is correct for `cargo install` distribution model. (status: defeated)
- Upstream crate fix: If `security-framework` added `set_accessible()` to PasswordOptions, we wouldn't need raw API. Counter: we can't control upstream timeline, and raw SecItemAdd is simpler anyway. (status: defeated)
- "AfterFirstUnlockThisDeviceOnly avoids deprecation warnings": True but wrong tradeoff — breaks SSH access after reboot, defeats the purpose of spec-secrets-keychain-ssh. Deprecation is iOS-only. (status: defeated)

## Applied-In

- `src/secrets/keychain.rs:store_identity()` — raw SecItemAdd with kSecAttrAccessible + kSecAttrAccessibleAlwaysThisDeviceOnly
- `layer/surface/build/fix/spec-keychain-macos26-regression/SPEC.md` — full specification of the macOS 26 fix

## Revision Log

- 2026-02-20: Created — initial version incorrectly specified AfterFirstUnlockThisDeviceOnly
- 2026-02-20: Revised — corrected to AlwaysThisDeviceOnly, added session-20260220-120045 evidence
