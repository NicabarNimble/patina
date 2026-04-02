# Design: Replace age crate with RustCrypto primitives

## Why This Design

The `age` crate (114 transitive deps) is the largest supply chain surface in
Patina's security-critical path. Patina uses <5% of age's functionality.
Replacing it with direct primitive use (x25519-dalek + chacha20poly1305)
eliminates ~84 crates while keeping the age v1 wire format for interoperability.

The replacement crates are audited (Quarkslab for x25519-dalek, NCC Group for
chacha20poly1305) and used by Rustls, Signal, and other security-critical
projects.

## Build Target

New `age_format.rs` module in Mother (~200-300 lines + tests). Drop `age` from
all Cargo.toml files. Vault files remain age-compatible.

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Bech32 for keys | Implement key encoding | Must read/write age1.../AGE-SECRET-KEY-1... |
| Chunk encryption | 64KB chunks per age spec | Required for interop |
| Header HMAC | HMAC-SHA256 | Age spec requirement, prevents stanza tampering |
| Armor format | 76-char base64 lines | Standard age armor |

## Commits

1. `deps(mother): add x25519-dalek dependency` —
   Add `x25519-dalek = "2"` to mother/Cargo.toml. Minimal change, verify
   it resolves cleanly.

2. `feat(mother): implement age v1 format with primitives` —
   New file `mother/src/secrets_authority_backend/age_format.rs`.
   Functions: `encrypt(plaintext, recipients) -> Vec<u8>`,
   `decrypt(ciphertext, identity) -> Vec<u8>`,
   `generate_keypair() -> (SecretKey, PublicKey)`,
   `parse_identity(s) -> SecretKey`, `parse_recipient(s) -> PublicKey`,
   `format_identity(key) -> String`, `format_recipient(key) -> String`.
   Includes armor read/write.

3. `test(mother): age format test vectors and interop` —
   Unit tests: roundtrip, multi-recipient, empty payload, large payload,
   chunk boundary, Bech32 encoding, header HMAC verification failure.
   Integration test: interop with `age` CLI if available (skip if not).

4. `refactor(mother): switch vault.rs to age_format module` —
   Replace imports and calls in vault.rs:
   - `age::Encryptor::with_recipients` → `age_format::encrypt`
   - `age::Decryptor::new` + `.decrypt()` → `age_format::decrypt`
   - `age::x25519::Identity` → `age_format::Identity` or x25519_dalek types
   - `ArmoredReader/Writer` → `age_format::armor`
   Replace imports in identity.rs:
   - `age::x25519::Identity::generate()` → `age_format::generate_keypair()`
   - `age::secrecy::ExposeSecret` → direct access (zeroize instead)

5. `deps: remove age crate from all Cargo.toml` —
   Remove `age` from mother/Cargo.toml and Cargo.toml.
   Verify: `cargo tree | grep "^.*age " | grep -v x25519` shows nothing.
   Verify: `cargo tree | grep fluent` shows nothing.

## Direct Code Targets

### New file
- NEW: `mother/src/secrets_authority_backend/age_format.rs` (~300 lines)

### Vault crypto replacement
- `mother/src/secrets_authority_backend/vault.rs:1-15` — replace age imports
- `mother/src/secrets_authority_backend/vault.rs:155-169` — replace `encrypt_bytes()`
- `mother/src/secrets_authority_backend/vault.rs:171-180` — replace `decrypt_bytes()`
- `mother/src/secrets_authority_backend/vault.rs:182-196` — update `init_vault()` key generation

### Identity key management
- `mother/src/secrets_authority_backend/identity.rs:2-3` — replace `age::secrecy`, `age::x25519`
- `mother/src/secrets_authority_backend/identity.rs:16-21` — replace `get_identity()` return type
- `mother/src/secrets_authority_backend/identity.rs:36-39` — replace `get_recipient()`
- `mother/src/secrets_authority_backend/identity.rs:41-48` — replace `generate_identity()`

### Dependency files
- `mother/Cargo.toml` — add x25519-dalek, remove age
- `Cargo.toml` — remove age (if not already done in consolidation)

## Verification Plan

```bash
# Dep verification
cargo tree -p mother | grep -c "age "          # expect: 0
cargo tree -p mother | grep fluent             # expect: nothing
cargo tree -p mother | grep scrypt             # expect: nothing
cargo tree -p patina-ai | grep -c "age "       # expect: 0

# Functional verification
cargo test -- secrets                          # all pass
cargo test -- age_format                       # new tests pass

# Interop (manual, if age CLI available)
echo "test secret" | age -r <recipient> -o /tmp/test.age
# decrypt with new code → verify "test secret"
# encrypt with new code → decrypt with age CLI → verify
```

## Build Readiness

- [x] Dependency audit with transitive counts
- [x] Replacement crates vetted (audits, download counts)
- [x] Age v1 spec reviewed for implementation scope
- [x] Realistic line estimate (200-300, not 50-80)
- [ ] Age test vectors prepared
- [ ] Blocked by vault-mother-consolidation

## Open Questions

1. **Bech32 implementation**: age uses Bech32 for key encoding (age1.../
   AGE-SECRET-KEY-1...). Do we pull in the `bech32` crate (~3 deps) or
   implement the subset we need (~40 lines)?
   **Recommendation**: Implement the subset. Bech32 encoding/decoding for
   two fixed HRPs (age, AGE-SECRET-KEY-) is straightforward.

2. **STREAM chunking**: The age payload uses the STREAM construction (not raw
   ChaCha20Poly1305). Each 64KB chunk gets its own nonce derived from a counter.
   This is well-documented in the spec but adds complexity beyond naive
   encrypt/decrypt.
   **Action**: Study the STREAM spec section carefully before estimating.

3. **secrecy crate**: Currently used via age for `ExposeSecret`. With age gone,
   do we keep `secrecy` as a direct dep or use `zeroize::Zeroizing` everywhere?
   **Recommendation**: `zeroize::Zeroizing` — already a dep, same purpose,
   one fewer crate.
