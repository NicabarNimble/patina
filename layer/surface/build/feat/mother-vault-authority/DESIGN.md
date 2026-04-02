# Design: Mother as sole vault authority

## Why This Design

The current architecture has the vault crypto stack duplicated across CLI
(`src/secrets/`) and daemon (`mother/src/secrets_authority_backend/`), with
the CLI rarely using its copy. This duplication caused identity drift
(three different identities across global vault, project vault, and
identity.enc) and carries 114 transitive crates through the `age` library
for crypto operations that need only ~5 primitives.

The 1Password `op` model proves this works at scale: agent owns the vault,
CLI is a stateless client, secrets flow through IPC but never persist in
the client process. Mother already serves this role — all CLI write
operations already dispatch through `dispatch_secrets_authority` to Mother.
This spec completes the transition by eliminating the CLI's direct vault
access and the bloated `age` dependency.

## Build Target

Single vault per Mother instance at `~/.patina/mother/vault.age`, using the
age v1 wire format implemented with RustCrypto primitives. CLI becomes a pure
IPC client. Dependency tree drops from 114 crates to ~30.

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Vault location | `~/.patina/mother/vault.age` | Mother's domain, clean ownership |
| Wire format | age v1 spec, implemented with primitives | Interop with `age` CLI for recovery |
| Caching | None — decrypt on demand, zeroize after | Cache solved Touch ID problem that no longer exists |
| Project vaults | Eliminated — single vault | `[[connection-secrets-live-in-global-vault]]` belief |
| Keychain | Extracted to opt-in child | Zero platform deps in core binary |
| `--global` flag | Removed | Single vault makes it meaningless |
| Identity resolution | env var → child backends → encrypted file | Pluggable, encrypted file is universal default |

### Dependency Decisions

**Drop `age` (0.11.2) — 114 transitive crates:**

The `age` crate includes functionality Patina never uses:
- i18n: fluent, i18n-embed, rust-embed, unic-langid, intl-memoizer (~25 crates)
- Passphrase KDF: scrypt, pbkdf2 (~5 crates)
- Alternative ciphers: salsa20
- Parser: nom, cookie-factory
- File traversal: walkdir

Patina uses only: X25519 key exchange, ChaCha20-Poly1305 AEAD, armor encoding.

**Keep (audited RustCrypto primitives):**
- `x25519-dalek` (2.0.1) — Quarkslab audited, used by Rustls and Signal
- `chacha20poly1305` (0.10.1) — NCC Group audited (Dec 2019), 35.8M+ downloads
- `hkdf` (0.12.4) — RustCrypto, pure Rust, depends on audited hmac
- `sha2` (0.10.9) — RustCrypto, 536M downloads
- `rand` (0.8.5) — 740M downloads, OS entropy via getrandom (two historical CVEs in rand_core, both patched)
- `zeroize` (1.8.2) — RustCrypto, memory zeroing with compiler cooperation
- `base64` — already in tree, zero transitive deps (could inline ~30 lines later)

**Extract to child (opt-in):**
- `security-framework` (3.6.0) — macOS Keychain, 6 transitive deps
- `core-foundation` (0.10.1) — macOS CF types, 3 transitive deps

### Prior Art from Session History

These sessions informed the design:

- **20260222-054702**: Empirically proved Keychain never works over SSH (-25308).
  Three approaches tested, all failed. `PATINA_IDENTITY` env var was the actual
  working path. Pivoted to encrypted file storage.
- **20260222-132656**: Designed dual-storage spec through 6 review rounds (1,142
  lines). Established ChaCha20-Poly1305 + HKDF-SHA256 for identity.enc format,
  machine-ID binding, three-layer migration safety nets.
- **20260203-192041**: Established `[[transport-security-by-trust-boundary]]` —
  UDS for local (file permissions = auth), mutual age-key for network. Mother
  as vault authority fits this model.
- **20260218-192625**: Designed launcher token auth. Established
  `get_global_secret()` as read-only helper. Confirmed secrets never touch
  project vaults for connection credentials.
- **20251224-162015**: Found and fixed the original `init_vault()` identity
  regeneration bug (always generated new identity, overwriting existing
  Keychain entry).

### Threat Model

From belief `[[llm-threat-model-unique]]`:
- **Threat**: LLM running `cat ~/.patina/identity.enc` or `env | grep PATINA_IDENTITY`
- **Not the threat**: Hardware security, side-channel attacks, nation-state adversaries
- **Protection**: Secrets flow through Mother's IPC, never appear in LLM context
- **Acceptable**: Plaintext in Mother's process memory during request handling
  (same as 1Password agent, HashiCorp Vault agent)
- **Acceptable**: Plaintext in CLI process memory transiently during env injection
  (same as `op run`, unavoidable for env var injection pattern)

## Commits

### Phase 1: Replace age with primitives

1. `feat(mother): implement age v1 format with x25519-dalek + chacha20poly1305` —
   Add `age_format.rs` to `mother/src/secrets_authority_backend/` implementing
   encrypt/decrypt with the age wire format using primitives directly. Tests
   verify roundtrip and compatibility with age CLI.

2. `refactor(mother): switch vault.rs to age_format module` —
   Replace `age::Encryptor`/`age::Decryptor` calls with the new module.
   Remove `age` from `mother/Cargo.toml`.

3. `deps: remove age crate from root Cargo.toml` —
   Remove `age` from the main crate. Add `x25519-dalek` if not already present.
   Verify `cargo tree | grep -c fluent` shows zero.

### Phase 2: Consolidate vault code into Mother

4. `refactor(secrets): route setup_claude replacing check through IPC` —
   Change `src/commands/secrets.rs:518` from `secrets::get_global_secret()`
   to `mother::get_global_secret()`. This eliminates the last direct vault
   read from the CLI.

5. `refactor(secrets): delete CLI vault/identity/crypto stack` —
   Delete from `src/secrets/`: vault.rs, identity.rs, storage.rs,
   encrypted_file.rs, keychain.rs, recipients.rs, registry.rs.
   Update mod.rs to remove submodule declarations and dead imports.
   Remove crypto deps from root Cargo.toml that are only used by deleted code.

6. `refactor(secrets): slim mod.rs to IPC client` —
   Keep: `run_with_secrets()`, `run_with_secrets_ssh()`, `prompt_for_value()`,
   `shell_join()`. Remove all functions that were wrappers around deleted modules.

### Phase 3: Add LoadAllSecrets IPC operation

7. `feat(protocol): add LoadAllSecrets to SecretsAuthorityOperation` —
   Add variant to `crates/patina-protocol/src/lib.rs`. Add `from_payload`
   and `into_payload` handling.

8. `feat(mother): implement LoadAllSecrets handler` —
   In `mother/src/secrets_authority_api.rs`: decrypt vault, load registry,
   return `{env_var: value}` map, zeroize after response.

9. `refactor(secrets): run_with_secrets uses LoadAllSecrets IPC` —
   Replace `load_all_secrets()` + `load_env_mappings()` direct vault calls
   with single IPC call to Mother. Delete helper functions.

### Phase 4: Single vault + remove cache

10. `refactor(mother): move vault to ~/.patina/mother/vault.age` —
    Update `mother/src/secrets_paths.rs`: vault_path, recipient_path,
    registry_path all under `~/.patina/mother/`. Remove project vault
    path functions.

11. `refactor(mother): remove session cache` —
    Remove `session::get_cached_secrets()` checks from vault operations.
    Each request decrypts fresh. Zeroize plaintext after response.
    Delete `mother/src/secrets_authority_backend/session.rs` if it exists
    in Mother, slim `src/secrets/session.rs` to remove cache client code.

12. `refactor(secrets): remove --global flag and project vault logic` —
    Update `src/commands/secrets.rs`: remove `--global` from
    `SecretsFlags`, remove project_root from add/remove flows.
    Update protocol: simplify `AddSecret`/`RemoveSecret` variants.

### Phase 5: Keychain child extraction

13. `feat(children): create keychain-macos identity backend child` —
    New child crate implementing identity storage trait via
    `security-framework`. Registers with Mother as credential backend.

14. `refactor(mother): remove keychain from core, add child dispatch` —
    Delete `mother/src/secrets_authority_backend/keychain.rs`.
    Update `storage.rs` to check registered children before encrypted file.
    Remove `security-framework`, `core-foundation` from `mother/Cargo.toml`.

## Direct Code Targets

### Phase 1 (age replacement)
- `mother/Cargo.toml` — remove `age`, add `x25519-dalek`
- `mother/src/secrets_authority_backend/vault.rs:155-180` — replace encrypt_bytes/decrypt_bytes
- `mother/src/secrets_authority_backend/vault.rs:1-15` — replace age imports
- `mother/src/secrets_authority_backend/identity.rs:3` — replace `use age::x25519`
- NEW: `mother/src/secrets_authority_backend/age_format.rs` — age v1 format implementation

### Phase 2 (CLI consolidation)
- `src/commands/secrets.rs:518` — change to `mother::get_global_secret()`
- DELETE: `src/secrets/vault.rs` (271 lines)
- DELETE: `src/secrets/identity.rs` (~190 lines)
- DELETE: `src/secrets/storage.rs` (83 lines)
- DELETE: `src/secrets/encrypted_file.rs` (501 lines)
- DELETE: `src/secrets/keychain.rs` (~280 lines)
- DELETE: `src/secrets/recipients.rs` (~70 lines)
- DELETE: `src/secrets/registry.rs` (~120 lines)
- `src/secrets/mod.rs` — strip to ~100 lines (from ~580)
- `Cargo.toml` — remove `age`, `chacha20poly1305`, `hkdf` from root (moved to mother-only)

### Phase 3 (LoadAllSecrets)
- `crates/patina-protocol/src/lib.rs:69-109` — add `LoadAllSecrets` variant
- `mother/src/secrets_authority_api.rs` — add handler (~20 lines)
- `src/secrets/mod.rs:258-300` — rewrite `run_with_secrets` to use IPC
- `src/secrets/mod.rs:306-351` — rewrite `run_with_secrets_ssh` to use IPC
- DELETE: `src/secrets/mod.rs:526-568` — `load_all_secrets()`, `load_env_mappings()`

### Phase 4 (single vault + cache removal)
- `mother/src/secrets_paths.rs:20-26` — vault_path, recipient_path under `mother/`
- `mother/src/secrets_authority_backend/mod.rs:244-256` — remove cache check in get_global_secret
- `src/commands/secrets.rs` — remove `--global` flag, simplify add/remove
- `crates/patina-protocol/src/lib.rs:73-84` — simplify AddSecret/RemoveSecret (remove global, project_root)

### Phase 5 (keychain child)
- NEW: `children/keychain-macos/` — child crate
- DELETE: `mother/src/secrets_authority_backend/keychain.rs`
- `mother/src/secrets_authority_backend/storage.rs` — child dispatch before encrypted_file fallback
- `mother/Cargo.toml` — remove security-framework, core-foundation

## Verification Plan

### Per-phase verification

**Phase 1:**
- `cargo check` (mother crate compiles without age)
- Unit test: roundtrip encrypt/decrypt with new age_format module
- Integration test: file produced by new code decryptable by `age` CLI (if installed)
- `cargo tree -p mother | grep fluent` returns nothing

**Phase 2:**
- `cargo check` (full project compiles)
- All existing `cargo test -- secrets` pass
- `patina secrets setup-claude` works (IPC path)
- `patina secrets` status display works

**Phase 3:**
- `patina secrets run -- env | grep SOME_SECRET` injects correctly
- `patina secrets run --ssh host -- cmd` works

**Phase 4:**
- Vault created at `~/.patina/mother/vault.age` (not `~/.patina/vault.age`)
- `patina secrets add test --global` errors (flag removed) or is silently accepted
- No `session.rs` cache code in hot path

**Phase 5:**
- `cargo tree -p mother | grep security-framework` returns nothing
- `patina child add keychain-macos` enables Keychain identity storage
- Without child: `patina secrets` works using encrypted file only

### End-to-end smoke test
```bash
# Clean slate
rm -f ~/.patina/mother/vault.age ~/.patina/mother/recipient.txt

# Add a secret
echo "test-value" | patina secrets add test-secret --stdin

# Verify injection
patina secrets run -- sh -c 'echo $TEST_SECRET'
# Expected: test-value

# Verify status
patina secrets
# Expected: 1 secret, 1 recipient

# Verify age format compatibility (if age CLI available)
patina secrets --export-key --stdout --confirm > /tmp/patina-key.txt
age --decrypt -i /tmp/patina-key.txt ~/.patina/mother/vault.age
rm /tmp/patina-key.txt
```

## Build Readiness

- [x] Problem understood with live reproduction
- [x] Prior art reviewed (6+ sessions, 12+ beliefs)
- [x] Dependency audit complete (114 → ~30 crates)
- [x] Industry pattern validated (1Password, HashiCorp Vault)
- [x] Commit plan with specific file targets
- [x] Phased approach — each phase independently verifiable
- [ ] Phase 1 age format implementation prototyped
- [ ] Keychain child interface designed (trait definition)

## Open Questions

1. **age format subset**: Do we need multi-recipient support immediately, or
   can Phase 1 start with single-recipient and add multi later? Current usage
   is single-recipient (one identity per vault). Multi-recipient exists for
   the `add-recipient` / `remove-recipient` commands.
   **Recommendation**: Implement multi-recipient from the start — it's only
   ~10 extra lines and avoids a breaking format change.

2. **Keychain child interface**: What trait does Mother expose for credential
   backend children? Needs: `has_identity() -> bool`, `get_identity() -> Result<String>`,
   `store_identity(&str) -> Result<()>`. Should this be a WASM child or native?
   **Recommendation**: Native child (Keychain requires FFI to macOS frameworks,
   can't run in WASM sandbox).

3. **Project-scoped secrets naming**: If a user needs per-project secrets in the
   single vault, the convention would be `{project_uid}:secret-name`. Should
   this be enforced in code or just documented?
   **Recommendation**: Document the convention, don't enforce in code. YAGNI
   until a user actually needs project-scoped secrets.

4. **Vault migration on first access**: When Mother starts and finds
   `~/.patina/vault.age` (old location) but no `~/.patina/mother/vault.age`,
   should it auto-migrate?
   **Recommendation**: Yes, one-time auto-migration with log message. Old
   location files are deleted after successful migration.
