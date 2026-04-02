---
type: refactor
id: vault-mother-consolidation
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
parent: mother-vault-authority
blocked_by: []
exit_criteria:
  - CLI never directly imports vault.rs, identity.rs, storage.rs, encrypted_file.rs, keychain.rs, recipients.rs, or registry.rs from src/secrets/
  - All patina secrets CLI commands produce identical user-facing output and behavior
  - setup_claude replacing_hint uses mother::get_global_secret (IPC) not secrets::get_global_secret (direct)
  - src/secrets/ contains only scanner.rs, session.rs, mod.rs
  - --global flag still accepted (no user-facing change)
  - Project vault operations still work (no user-facing change)
  - cargo check and all secrets tests pass
---
# refactor: Consolidate vault code into Mother

> Delete CLI vault/identity/crypto stack from src/secrets/, route all operations through Mother IPC. Zero behavior change for users.

## Problem

The vault/identity/keychain/storage/encrypted_file stack exists in two places:

1. `src/secrets/` (CLI process) — ~1,500 lines across 7 files
2. `mother/src/secrets_authority_backend/` (daemon process) — ~1,200 lines

Both can generate identities, create vaults, and encrypt/decrypt. This caused a
live bug (session 20260402): the global vault was encrypted with identity
`age1ru4ztm...` but the stored identity resolves to `age1g3say...`, producing
"No matching keys found" on `patina secrets setup-claude`. Three different
identities existed across global vault, project vault, and identity.enc.

The CLI copy is almost entirely unused — all write operations already dispatch
through `dispatch_secrets_authority` to Mother. Only TWO direct usages remain:

1. `src/commands/secrets.rs:518` — `secrets::get_global_secret("claude-oauth")`
   for a `replacing_hint` UI check
2. `src/secrets/mod.rs:264` — `run_with_secrets()` calls `load_all_secrets()`
   which decrypts vaults directly

`mother::get_global_secret()` already exists at `src/mother/mod.rs:136` as an
IPC wrapper, making bypass #1 trivially fixable.

## Goal

Delete the CLI-side vault/identity/crypto code. Mother's
`secrets_authority_backend/` becomes the single source of truth. Zero
user-facing behavior change.

## Status

Draft. Stopgap fix for identity/vault mismatch already committed.

## Non-Goals

- Removing the `age` crate (separate spec: `drop-age-crate`)
- Adding `LoadAllSecrets` IPC operation (separate spec: `secrets-load-all-ipc`)
- Eliminating project vaults or session cache (separate spec: `single-vault`)
- Extracting Keychain to child (separate spec: `keychain-child`)
- Any change to the `patina secrets` CLI interface

## Current State

```
src/secrets/
  mod.rs           — 580 lines, functions that wrap vault/identity modules
  vault.rs         — 271 lines, age encrypt/decrypt (DUPLICATE of mother/)
  identity.rs      — 190 lines, key management (DUPLICATE)
  storage.rs       — 83 lines, orchestration (DUPLICATE)
  encrypted_file.rs — 501 lines, ChaCha20 identity storage (DUPLICATE)
  keychain.rs      — 280 lines, macOS Keychain (DUPLICATE)
  recipients.rs    — 70 lines, recipient parsing (DUPLICATE)
  registry.rs      — 120 lines, secrets registry (DUPLICATE)
  scanner.rs       — secret detection (NOT duplicate, CLI concern)
  session.rs       — session cache client (keeps)
```

## Target State

```
src/secrets/
  mod.rs           — ~200 lines: run_with_secrets, prompt_for_value, IPC bridge
  scanner.rs       — unchanged
  session.rs       — unchanged
```

## Solution

**Step 1:** Fix `src/commands/secrets.rs:518` — change `secrets::get_global_secret()`
to `mother::get_global_secret()`.

**Step 2:** Rewrite `load_all_secrets()` and `load_env_mappings()` as IPC wrappers.
Use existing `dispatch_secrets_authority` with `Status` (get secret names) +
`GetGlobalSecret` (get each value). N+1 calls is fine for ~5 secrets.

**Step 3:** Delete 7 files from `src/secrets/`.

**Step 4:** Slim `mod.rs` — remove dead submodule declarations, dead pub-use
re-exports, dead wrapper functions.

**Step 5:** Remove unused crypto deps from root `Cargo.toml` (verify with `cargo tree`).

## Implementation Order

1. Fix setup_claude (1 line)
2. IPC bridge for load_all_secrets (~40 lines)
3. Delete 7 files
4. Slim mod.rs
5. Clean root Cargo.toml deps

## Compatibility Matrix

This spec is the foundation. Other specs progressively remove features.
This matrix prevents agents from "optimizing ahead" across spec boundaries.

| Feature | After consolidation | After load-all-ipc | After single-vault |
|---------|-------------------|--------------------|--------------------|
| `--global` flag | works | works | accepted silently (deprecated) |
| Project vault | works via daemon | works via daemon | removed (warning) |
| Session cache | works | works | removed |
| `run_with_secrets` | N+1 IPC bridge | single IPC call | single IPC call |
| Protocol `global` field | present | present | removed (old payloads parse) |
| Protocol `project_root` | present | present | removed (old payloads parse) |

**Rule:** Each spec only removes what it explicitly lists. If consolidation
says "project vault still works", no agent removes project vault support
during consolidation, even if single-vault will do so later.

## Resolved Decisions

- **run_with_secrets bridge**: N+1 IPC calls using existing operations. Replaced
  by single `LoadAllSecrets` call in separate spec.
- **--global flag**: Keep accepting. Routes correctly through daemon.
- **Session cache**: Leave intact. Separate concern.
- **Re-exported types** (`IdentitySource`, `VaultStatus`, validation fns): Verify
  no external consumer before deleting. If needed, re-export from protocol crate.

## Verification

- `cargo check` passes (root + mother)
- `cargo test -- secrets` — all 36 tests pass
- `patina secrets` — status works
- `patina secrets setup-claude` — enters prompt, no crash
- `patina secrets run -- env` — secrets injected
- `patina secrets check` — scanner works
- `grep -r "vault::" src/secrets/` — returns nothing
- `wc -l src/secrets/*.rs` — ~200 lines total

## Exit Criteria

- [ ] CLI never directly imports vault/identity/storage/encrypted_file/keychain/recipients/registry
- [ ] All `patina secrets` CLI commands identical behavior
- [ ] setup_claude uses `mother::get_global_secret` (IPC)
- [ ] `src/secrets/` contains only scanner.rs, session.rs, mod.rs
- [ ] `--global` flag still accepted
- [ ] Project vault operations still work
- [ ] `cargo check` and all secrets tests pass

## Build Readiness

- [x] Full codebase audit of src/secrets/ consumers
- [x] Two direct bypass sites identified with line numbers
- [x] mother::get_global_secret IPC wrapper exists
- [x] dispatch_secrets_authority pattern established
- [ ] Verify re-exported types have no external consumers
