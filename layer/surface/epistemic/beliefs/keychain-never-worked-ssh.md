---
type: belief
id: keychain-never-worked-ssh
persona: architect
facets: [macos, security, ssh, keychain]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-22
revised: 2026-02-22
---

# keychain-never-worked-ssh

macOS Keychain never worked over SSH - Security framework blocks with OSStatus -25308 regardless of API approach or accessibility attribute

## Statement

macOS Keychain never worked over SSH - Security framework blocks with OSStatus -25308 regardless of API approach or accessibility attribute

## Evidence

- [[session-20260222-054702]]: [[session-20260222-054702]] - Empirical testing of 3 approaches: get_generic_password (FAILED -25308), raw SecItemCopyMatching (FAILED -25308), fresh AlwaysThisDeviceOnly item (FAILED -25308) (weight: 0.95)

## Supports

- [[spec-secrets-dual-storage]]: Motivated the dual-storage architecture (Keychain for console, encrypted file for SSH)
- [[llm-threat-model-unique]]: Understanding this limitation led to designing encrypted file storage for SSH contexts

## Attacks

- [[keychain-always-this-device-only]]: Refutes the claim that AlwaysThisDeviceOnly enables SSH access
- [[raw-keychain-over-access-control]]: Refutes the claim that raw SecItemCopyMatching bypasses SSH restrictions
- [[spec-secrets-keychain-ssh]]: Proves this spec never actually worked (marked complete erroneously)
- [[spec-keychain-macos26-regression]]: Proves the "regression" was actually discovering it never worked

## Attacked-By

<!-- No known attacks - empirically validated -->

## Applied-In

- `spec-secrets-dual-storage`: Designed encrypted file fallback for SSH contexts based on this constraint
- `src/secrets/storage.rs`: (Future) Will detect SSH_CONNECTION and use encrypted file instead of Keychain
- Test suite: `test-ssh-localhost.sh`, `test-keychain-access.sh` validate this behavior

## Revision Log

- 2026-02-22: Created — metrics computed by `patina scrape`
