---
type: feat
id: mother-vault-authority
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
child_specs:
  - vault-mother-consolidation
  - drop-age-crate
  - secrets-load-all-ipc
  - single-vault
  - keychain-child
exit_criteria:
  - All child specs completed
  - src/secrets/ reduced to scanner.rs, prompt_for_value(), and IPC client wrappers
  - All existing patina secrets CLI commands work identically from user perspective
  - cargo check and all secrets tests pass
  - Vault files remain age-format compatible (recoverable with age CLI tool)
---
# feat: Mother as sole vault authority (umbrella)

> Consolidate secrets vault into Mother: single vault, drop age crate, decrypt-on-demand, keychain as opt-in child, CLI becomes pure IPC client

**This is an umbrella spec.** Split into 5 child specs after audit review
(scope too wide for one milestone per `[[spec-is-milestone]]`):

| Spec | Type | Blocked by | Scope |
|------|------|-----------|-------|
| `vault-mother-consolidation` | refactor | — | Delete CLI vault code, IPC-only |
| `drop-age-crate` | refactor | consolidation | Replace age with primitives (114 → ~30 deps) |
| `secrets-load-all-ipc` | feat | consolidation | LoadAllSecrets protocol operation |
| `single-vault` | refactor | consolidation, load-all | One vault, no cache, deprecate --global |
| `keychain-child` | feat | consolidation | Keychain as opt-in native child |

The sections below are preserved as the vision document and design context
that informed the child specs. Build against the children, not this umbrella.

---

## Problem

The secrets vault system has three structural problems:

**1. Duplicated vault code causes identity drift.**
The vault/identity/keychain/storage/encrypted_file stack exists in two places:
`src/secrets/` (CLI) and `mother/src/secrets_authority_backend/` (daemon). Both
can generate identities and create vaults. This caused a live bug where the
global vault was encrypted with one identity (`age1ru4ztm...`) but the stored
identity resolves to a different key (`age1g3say...`), producing "No matching
keys found" on every `patina secrets setup-claude` call. Three different
identities existed across global vault, project vault, and identity.enc.

**2. The `age` crate pulls 114 transitive dependencies.**
Patina uses age for X25519 encryption with known recipients — no passphrase
encryption, no i18n error messages, no scrypt. But the `age` crate bundles
fluent/i18n-embed (internationalization), scrypt/pbkdf2 (passphrase KDF),
salsa20, cookie-factory, nom, walkdir, rust-embed, and dozens more. Each of
these 114 crates is a supply chain attack surface for a security-critical path.

**3. Session cache solves a problem that no longer exists.**
The session cache was added to avoid repeated Touch ID prompts when identity
lived in macOS Keychain. But empirical testing (session 20260222-054702) proved
Keychain never worked over SSH (-25308 policy error). The pivot to encrypted
file storage (ChaCha20-Poly1305, machine-bound) removed the biometric gate.
Vault decryption is now sub-millisecond with no user prompt — caching secrets
in Mother's memory is unnecessary risk for zero performance benefit.

## Goal

Mother becomes the sole vault authority. The CLI is a stateless IPC client
for secrets — it never touches `vault.age`, `identity.enc`, or any crypto
key material. This follows the 1Password `op` architecture: agent owns the
vault, CLI talks to the agent, secrets flow through but never persist in
the client.

This enables future capabilities without rework:
- Cross-Mother secret sharing via mutual age-key authentication
- Web/browser access via Mother's existing TCP+bearer-token path
- Other tool integration via Mother's IPC API
- Pluggable credential backends via child architecture

## Status

Draft. Immediate bug fix (identity/vault mismatch recovery in add_secret)
already committed on the patina branch as a stopgap.

## Non-Goals

- Replacing RustCrypto primitives (chacha20poly1305, hkdf, sha2) — these are
  audited, minimal, and reimplementing crypto primitives is strictly riskier
- Multi-vault or per-project vault isolation — single vault with optional
  project-UID namespacing in secret names covers the use case
- Mother as a separate binary — keep as library crate for now, but structure
  code so the boundary is clean for future extraction
- Cloud KMS or hardware token integration — future children, not this spec

## Target Shape

```
patina secrets run -- cmd
    CLI ──IPC──▶ Mother: "LoadAllSecrets"
                 Mother reads identity.enc (machine-bound, no prompt)
                 Mother decrypts vault.age (x25519+chacha20poly1305)
                 Mother returns {ENV_VAR: value} map
                 Mother zeroizes plaintext from memory
    CLI receives env map
    CLI injects env vars, spawns subprocess
    CLI drops map after spawn
```

Architecture matches the industry pattern:
```
1Password:    op CLI  →  1Password agent  →  vault
HashiCorp:    vault CLI  →  vault agent  →  vault storage
Patina:       patina CLI  →  Mother daemon  →  vault.age
```

## Solution

### Phase 1: Drop `age` crate, implement age format with primitives

Replace the `age` library (114 transitive deps) with direct use of:
- `x25519-dalek` — X25519 key exchange (audited by Quarkslab, used by Rustls/Signal)
- `chacha20poly1305` — AEAD encryption (audited by NCC Group, 35M+ downloads)
- `base64` — armor encoding (already in tree)

Implement the age v1 wire format directly (~50-80 lines):
```
age-encryption.org/v1
-> X25519 <ephemeral_public_key>
<encrypted_file_key_base64>
--- <HMAC>
<ChaCha20Poly1305_encrypted_payload>
```

This keeps vault files compatible with the `age` CLI tool for recovery/debugging
while eliminating 114 crates from the dependency tree.

### Phase 2: Consolidate vault code into Mother

Delete from `src/secrets/`:
- `vault.rs` — vault encryption/decryption
- `identity.rs` — identity management
- `storage.rs` — storage orchestration
- `encrypted_file.rs` — ChaCha20-Poly1305 identity encryption
- `keychain.rs` — macOS Keychain integration
- `recipients.rs` — recipient file management
- `registry.rs` — secrets registry

Keep in `src/secrets/`:
- `scanner.rs` — secret detection in staged/tracked files (CLI concern, no vault access)
- `session.rs` — slimmed to IPC client only (no cache)
- `mod.rs` — `run_with_secrets()`, `run_with_secrets_ssh()`, `prompt_for_value()`, IPC wrappers

Mother's `secrets_authority_backend/` becomes the single source of truth for
all vault, identity, and crypto operations.

### Phase 3: Add LoadAllSecrets IPC operation

Add to `SecretsAuthorityOperation` in `patina-protocol`:
```rust
LoadAllSecrets {
    project_root: Option<String>,
},
```

Mother handler:
1. Decrypt vault.age with current identity
2. Load registry for env var mappings
3. Return `HashMap<String, String>` (env_var → value)
4. Zeroize all plaintext from memory after serializing response

CLI's `run_with_secrets` becomes:
1. IPC call to Mother: `LoadAllSecrets`
2. Inject returned env vars into subprocess
3. Drop the map

### Phase 4: Single vault, remove session cache

- Move vault location to `~/.patina/mother/vault.age`
- Update `mother/src/secrets_paths.rs` to reflect new location
- Remove session cache from Mother's secrets backend — each request decrypts
  fresh, serves, zeroizes
- Remove `--global` flag distinction (everything is Mother's vault)
- Project-scoped secrets use naming convention if needed: `{project_uid}:secret-name`

### Phase 5: Extract Keychain to opt-in child

Create a `keychain-macos` child that:
- Implements identity storage trait: `has_identity()`, `get_identity()`, `store_identity()`
- Registers with Mother as a credential backend
- Users opt in: `patina child add keychain-macos`
- `security-framework` + `core-foundation` (9 crates) leave the core binary

Mother's identity resolution becomes:
1. Check `PATINA_IDENTITY` env var (CI/headless escape hatch)
2. Check registered credential backend children
3. Fall back to encrypted file (`identity.enc`, always available)

## Implementation Order

**See child specs for authoritative ordering.** Dependency graph:

```
vault-mother-consolidation (foundation, no blockers)
  ├── drop-age-crate
  ├── secrets-load-all-ipc
  │     └── single-vault (also blocked by consolidation)
  └── keychain-child
```

## Resolved Decisions

**Vault location:** `~/.patina/mother/vault.age` — Mother's house, Mother's vault.

**Wire format:** Keep age v1 format for vault files. Implement with primitives,
not the age library. Maintains CLI recovery escape hatch (`age --decrypt`).

**Session cache:** Remove entirely. Vault decryption with encrypted file identity
is sub-millisecond, no biometric prompt. Cache was solving a Keychain/Touch ID
problem that no longer exists (belief: `[[keychain-never-worked-ssh]]`).

**Project vaults:** Eliminated. Single vault per Mother. Project-scoped secrets
use naming convention (`{uid}:name`) if ever needed. Belief:
`[[connection-secrets-live-in-global-vault]]` already established this direction.

**Keychain:** Extracted to opt-in child. Core binary has zero platform-specific
security deps. Encrypted file is the universal default.

**Dependency stack (final):**
```
Core (always compiled):
  x25519-dalek        — audited (Quarkslab), used by Rustls/Signal
  chacha20poly1305     — audited (NCC Group), RustCrypto
  hkdf                 — RustCrypto, pure Rust
  sha2                 — RustCrypto, 536M downloads
  rand                 — OS entropy, unavoidable
  zeroize              — memory zeroing, RustCrypto
  base64               — armor encoding (could inline later)

Optional child:
  security-framework   — keychain-macos child only
  core-foundation      — keychain-macos child only
```

From 114 transitive crates to ~30 unique (with heavy sharing among RustCrypto).

**`--global` flag:** Removed. With single vault, all secrets are "global" from
Mother's perspective.

## Verification

- All `patina secrets *` CLI commands produce identical user-facing output
- `patina secrets run -- env | grep CLAUDE` injects correctly via IPC
- `patina secrets setup-claude` stores and retrieves token via IPC
- Vault file is valid age format: `age --decrypt -i <exported_key> vault.age` works
- `cargo tree -p patina-ai | grep -c age` shows zero (age crate removed)
- `cargo tree -p patina-ai | grep -c fluent` shows zero (i18n gone)
- Identity mismatch on empty vault auto-recovers (stopgap fix preserved)
- No secrets appear in LLM context window during normal operation

## Exit Criteria

- [ ] Mother is the sole vault decryptor — CLI never touches vault.age or identity.enc
- [ ] `age` crate removed from dependency tree — vault uses x25519-dalek + chacha20poly1305 directly
- [ ] Single vault at `~/.patina/mother/vault.age` — no project-scoped vaults
- [ ] `LoadAllSecrets` IPC operation added to `SecretsAuthorityOperation` protocol
- [ ] `run_with_secrets` and `run_with_secrets_ssh` use IPC, not direct vault decrypt
- [ ] Session cache removed — decrypt-on-demand with zeroize after serve
- [ ] Keychain code extracted from core binary — available as opt-in child
- [ ] `src/secrets/` reduced to scanner.rs, prompt_for_value(), and IPC client wrappers
- [ ] All existing `patina secrets` CLI commands work identically from user perspective
- [ ] `cargo check` and all secrets tests pass
- [ ] Vault files remain age-format compatible (recoverable with `age` CLI tool)

## Build Readiness

- [x] Problem articulated with evidence (live identity drift bug, dependency audit)
- [x] Prior session history reviewed (6+ sessions of secrets design, Feb 2026)
- [x] Beliefs consulted (keychain-never-worked-ssh, connection-secrets-live-in-global-vault, llm-threat-model-unique, transport-security-by-trust-boundary)
- [x] Dependency audit complete (114 crates → ~30, audited crates identified)
- [x] Architecture validated against industry (1Password op, HashiCorp Vault agent)
- [ ] Design doc filled with commit plan and code targets
