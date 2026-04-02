# Design: Consolidate vault code into Mother

## Why This Design

The CLI carries ~1,500 lines of vault/identity/crypto code that it barely uses.
All write operations already go through Mother's IPC. Only two direct vault
access points remain. Deleting the CLI copy eliminates the class of bugs where
two processes manage the same identity/vault files independently — the exact
class that caused the live identity drift bug in session 20260402.

This is the foundation spec. All other vault improvements depend on this:
- `drop-age-crate` modifies Mother's vault code (must be the only copy)
- `secrets-load-all-ipc` replaces the temporary bridge this spec creates
- `single-vault` changes vault paths (only in Mother)
- `keychain-child` extracts Mother's keychain module (must be the only copy)

## Build Target

`src/secrets/` goes from 8 crypto modules (~1,500 lines) to 3 modules (~200
lines). Mother's `secrets_authority_backend/` becomes the single vault code
path. Zero user-facing behavior change.

## Prior Art

- **Session 20260402**: Live identity drift — three different identities across
  global vault, project vault, and identity.enc. Root cause: two independent
  vault code paths that can each generate identities.
- **Session 20260222-054702**: Proved Keychain never works over SSH (-25308).
  CLI-side keychain.rs is doubly dead — unused by CLI commands AND unreliable.
- **Session 20251224**: Original init_vault() bug — always generated new identity.
  Fix had to be applied in two places because of the duplication.
- **Codebase audit (this session)**: 10 files import from secrets module. 8 of 10
  already use `mother::get_global_secret` or `dispatch_secrets_authority`. Only
  `src/commands/secrets.rs` has direct vault bypass (2 call sites).

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Bridge for run_with_secrets | N+1 IPC calls (Status + GetGlobalSecret per name) | Uses existing protocol, no blocking dependency |
| --global flag | Keep | Zero behavior change commitment |
| Session cache | Leave | Separate spec (single-vault) |
| Re-exported types | Verify first, then remove or move to protocol | Can't break external consumers |

## Commits

1. `refactor(secrets): route setup_claude check through mother IPC` —
   `src/commands/secrets.rs:518`: `secrets::get_global_secret()` →
   `mother::get_global_secret()`. Single line, eliminates last direct
   vault read from CLI commands.

2. `refactor(secrets): implement load_all_secrets as IPC bridge` —
   Replace `load_all_secrets()` and `load_env_mappings()` in
   `src/secrets/mod.rs:526-568` with IPC-based implementation using
   `dispatch_secrets_authority`. Calls Status to get names, then
   GetGlobalSecret for each value. Temporary until secrets-load-all-ipc.

3. `refactor(secrets): delete CLI vault/identity/crypto stack` —
   Delete 7 files: vault.rs, identity.rs, storage.rs, encrypted_file.rs,
   keychain.rs, recipients.rs, registry.rs. Update mod.rs submodule
   declarations.

4. `refactor(secrets): slim mod.rs to IPC client` —
   Remove dead functions and pub-use re-exports. Keep: run_with_secrets,
   run_with_secrets_ssh, prompt_for_value, shell_join.

5. `deps: remove CLI-only crypto deps from root Cargo.toml` —
   Verify each with `cargo tree`, remove provably unused. sha2 likely
   stays (ONNX integrity).

## Direct Code Targets

### Commit 1
- `src/commands/secrets.rs:518` — `secrets::get_global_secret` → `mother::get_global_secret`
- `src/commands/secrets.rs:7` — verify `mother` in imports

### Commit 2
- `src/secrets/mod.rs:526-541` — rewrite `load_all_secrets()`
- `src/secrets/mod.rs:544-568` — rewrite `load_env_mappings()`
- Reference: `src/commands/secrets.rs:170-194` for dispatch pattern

### Commit 3
- DELETE: `src/secrets/vault.rs` (271 lines)
- DELETE: `src/secrets/identity.rs` (~190 lines)
- DELETE: `src/secrets/storage.rs` (83 lines)
- DELETE: `src/secrets/encrypted_file.rs` (501 lines)
- DELETE: `src/secrets/keychain.rs` (~280 lines)
- DELETE: `src/secrets/recipients.rs` (~70 lines)
- DELETE: `src/secrets/registry.rs` (~120 lines)

### Commit 4
- `src/secrets/mod.rs` — remove `mod vault; mod identity; ...` declarations
- `src/secrets/mod.rs` — remove `pub use self::vault::VaultStatus` etc.
- `src/secrets/mod.rs` — remove: check_status, add_secret, remove_secret,
  lock_session, export_identity, import_identity, reset_identity,
  add_recipient, remove_recipient, list_recipients, get_global_secret

### Commit 5
- `Cargo.toml` — verify and remove: `age`, `chacha20poly1305`, `hkdf`
  (if only consumed by deleted src/secrets/ modules)

## Verification Plan

Per-commit: `cargo check` + `cargo test -- secrets`

Final:
- `patina secrets` — status display
- `patina secrets setup-claude` — prompt appears, no crash
- `patina secrets run -- env` — injection works
- `patina secrets check` — scanner works
- `grep -r "vault::\|identity::\|keychain::" src/secrets/` — nothing
- `wc -l src/secrets/*.rs` — ~200 total

## Build Readiness

- [x] Full audit complete
- [x] IPC wrappers exist
- [x] Bypass sites identified
- [ ] Re-exported types consumer check

## Open Questions

1. **Re-exported types**: Need `grep -r` verification that `IdentitySource`,
   `VaultStatus`, `infer_env_name`, `is_valid_env_name`, `is_valid_secret_name`
   have no external consumers before deleting.

2. **IPC bridge N+1 performance**: For a user with 20+ secrets,
   `run_with_secrets` would make 21 IPC calls. Acceptable as temporary bridge?
   **Assessment**: Yes — UDS roundtrip is <1ms, so 21 calls < 25ms. Replaced
   by single `LoadAllSecrets` call in follow-up spec.
