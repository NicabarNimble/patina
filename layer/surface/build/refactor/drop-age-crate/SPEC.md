---
type: refactor
id: drop-age-crate
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
parent: mother-vault-authority
blocked_by:
  - vault-mother-consolidation
exit_criteria:
  - age crate removed from Cargo.toml and mother/Cargo.toml
  - cargo tree shows zero age/fluent/i18n/scrypt/salsa20 transitive deps
  - Vault encrypt/decrypt uses x25519-dalek + chacha20poly1305 directly
  - Existing vault.age files decrypt correctly with new implementation
  - New vault.age files decrypt correctly with age CLI tool (format compatible)
  - age v1 test vectors pass (from age spec)
  - Multi-recipient encrypt/decrypt works
  - cargo check and all secrets tests pass
---
# refactor: Replace age crate with RustCrypto primitives

> Implement age v1 wire format with x25519-dalek + chacha20poly1305 directly. Eliminate 114 transitive deps from supply chain.

## Problem

The `age` crate (0.11.2) pulls 114 transitive dependencies. Patina uses it
only for X25519 encryption with known recipients — no passphrase encryption,
no i18n error messages, no scrypt. The 114 crates include:

- **i18n stack (~25 crates)**: fluent, i18n-embed, rust-embed, unic-langid,
  intl-memoizer, lazy_static, smallvec, tinystr, yoke, zerofrom, zerovec
- **Passphrase KDF (~5 crates)**: scrypt, pbkdf2 — never used (no passphrases)
- **Alternative ciphers**: salsa20 — never used
- **Parser/serialization**: nom, cookie-factory — for format parsing
- **File traversal**: walkdir — for embedded i18n locale files
- **Misc**: futures, parking_lot, pin-project, slab, type-map, etc.

Each of these 114 crates is a supply chain attack surface for Patina's most
security-critical path: the code that protects user secrets.

## Goal

Replace the `age` library with ~100 lines implementing the age v1 wire format
using audited RustCrypto primitives directly. Vault files remain interoperable
with the `age` CLI for recovery/debugging.

## Status

Draft. Blocked by `vault-mother-consolidation` (vault code must live in one
place before replacing the crypto layer).

## Non-Goals

- Inventing a custom vault format (keep age v1 for interop)
- Replacing RustCrypto primitives (audited, minimal, correct)
- Supporting age passphrases, SSH keys, or plugins (never needed)

## Current State

**Dependencies used by vault code:**
```
age (0.11.2) — 114 transitive crates
  ├── age-core (X25519, format)
  ├── i18n-embed + fluent (~25 crates) — UNUSED
  ├── scrypt + pbkdf2 (~5 crates) — UNUSED
  ├── salsa20 — UNUSED
  ├── nom + cookie-factory — format parsing
  ├── walkdir + rust-embed — i18n resource loading
  └── futures, parking_lot, etc. — infrastructure
```

**What Patina actually calls:**
- `x25519::Identity::generate()` — generate keypair
- `x25519::Identity::from_str()` — parse secret key
- `x25519::Identity::to_public()` — derive public key
- `x25519::Recipient::from_str()` — parse public key
- `age::Encryptor::with_recipients()` — encrypt for recipients
- `age::Decryptor::new()` — decrypt with identity
- `ArmoredReader` / `ArmoredWriter` — ASCII armor encoding

## Target State

**Replacement dependencies:**
```
x25519-dalek (2.0.1)     — audited (Quarkslab), used by Rustls/Signal
chacha20poly1305 (0.10.1) — audited (NCC Group), 35.8M downloads
hkdf (0.12.4)            — RustCrypto (for file key wrapping)
sha2 (0.10.9)            — RustCrypto (HMAC in age header)
base64                   — armor encoding
```

Most of these are already direct deps (chacha20poly1305, hkdf, sha2 for
identity.enc). Only `x25519-dalek` is new. Total unique transitive deps
drops from ~114 to ~30 (heavy sharing among RustCrypto crates).

## Solution

### Age v1 wire format implementation

The age v1 spec (https://age-encryption.org/v1) for X25519 recipients:

```
age-encryption.org/v1
-> X25519 <ephemeral_share_base64>
<wrapped_file_key_base64>
--- <header_mac_base64>
<payload: ChaCha20Poly1305 encrypted, 64KB chunks>
```

Implementation in `mother/src/secrets_authority_backend/age_format.rs`:

1. **Encrypt**: Generate ephemeral X25519 keypair → ECDH with each recipient →
   HKDF-SHA256 to derive wrapping key → wrap file key with ChaCha20Poly1305 →
   encrypt payload in 64KB chunks → HMAC header → armor output

2. **Decrypt**: Parse header → find matching recipient stanza → ECDH with
   identity → unwrap file key → verify header HMAC → decrypt payload chunks

3. **Key types**: Wrap `x25519_dalek::StaticSecret` / `PublicKey` with
   Bech32 encoding (age1... / AGE-SECRET-KEY-1...) for format compatibility

4. **Armor**: base64 encoding with `-----BEGIN AGE ENCRYPTED FILE-----` /
   `-----END AGE ENCRYPTED FILE-----` wrapping, 76-char line width

### Audit agent concern: "~50-80 lines is optimistic"

Acknowledged. The happy path for single-recipient is ~50 lines, but a correct
and interoperable implementation requires:
- Multi-recipient stanza generation and parsing
- 64KB chunk boundaries for payload encryption (each chunk has its own nonce)
- HMAC verification of the header (prevents tampering with recipient stanzas)
- Bech32 encoding for key serialization (age1... format)
- Armor line-wrapping at 76 characters
- Proper nonce construction (chunk counter as 11-byte big-endian + last-chunk flag)

Realistic estimate: ~200-300 lines for the format module, plus ~100 lines of tests.

### Test vectors

The age specification includes test vectors. Implementation must pass:
- Official X25519 test vectors from the age test suite
- Roundtrip: encrypt with new code, decrypt with `age` CLI
- Roundtrip: encrypt with `age` CLI, decrypt with new code
- Multi-recipient: encrypt for 2+ recipients, each can decrypt
- Edge cases: empty payload, large payload (multiple chunks), single-byte payload

## Implementation Order

1. Add `x25519-dalek` to `mother/Cargo.toml`
2. Implement `age_format.rs` with encrypt/decrypt functions
3. Add comprehensive tests (vectors + interop)
4. Replace `vault.rs` encrypt_bytes/decrypt_bytes to use new module
5. Remove `age` from `mother/Cargo.toml`
6. Remove `age` from root `Cargo.toml` (if not already removed in consolidation spec)
7. Verify `cargo tree` shows zero age-related deps

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Wire format | age v1 (keep) | Interop with age CLI for recovery |
| Key format | Bech32 (age1.../AGE-SECRET-KEY-1...) | Compatibility with existing stored keys |
| Chunk size | 64KB (age spec) | Must match for interop |
| Multi-recipient | Implement from start | Only ~10 extra lines, avoids format break later |
| secrecy crate | Drop (age dependency) | Use zeroize::Zeroizing directly |

### Dependency audit results (carried from parent spec)

| Crate | Maintainer | Audited | Downloads | Risk |
|-------|-----------|---------|-----------|------|
| x25519-dalek | dalek-cryptography | Quarkslab 2019 | High | Low |
| chacha20poly1305 | RustCrypto | NCC Group 2019 | 35.8M | Low |
| hkdf | RustCrypto | No formal audit | Medium | Low |
| sha2 | RustCrypto | No formal audit | 536M | Low |
| rand | rust-random | No (2 patched CVEs) | 740M | Watch |
| zeroize | RustCrypto | No formal audit | High | Low |
| base64 | marshallpierce | No formal audit | 783M | Low |

## Verification

- `cargo tree -p mother | grep -c "^.*age "` — shows 0
- `cargo tree -p mother | grep fluent` — returns nothing
- `cargo tree -p mother | grep scrypt` — returns nothing
- Unit tests: all age v1 test vectors pass
- Interop test: new code ↔ age CLI roundtrip
- Existing `cargo test -- secrets` still pass
- `patina secrets add` / `patina secrets run` work correctly

## Exit Criteria

- [ ] `age` removed from all Cargo.toml files
- [ ] Zero age/fluent/i18n/scrypt/salsa20 in `cargo tree`
- [ ] Vault encrypt/decrypt uses x25519-dalek + chacha20poly1305
- [ ] Existing vault.age files decrypt with new code
- [ ] New vault.age files decrypt with `age` CLI
- [ ] Age v1 test vectors pass
- [ ] Multi-recipient works
- [ ] `cargo check` and all tests pass

## Build Readiness

- [x] Dependency audit complete (114 crates enumerated)
- [x] Patina's actual age API surface identified (7 functions)
- [x] Replacement crates identified and vetted
- [x] Audit agent's scope concern addressed (realistic line estimate)
- [ ] age v1 test vectors downloaded/prepared
- [ ] Blocked by vault-mother-consolidation
