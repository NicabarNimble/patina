---
type: refactor
id: move-vault-to-mother
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
blocked_by: []
exit_criteria:
  - src/secrets/ contains zero vault/identity/crypto modules — only scanner.rs, session.rs, mod.rs (thin IPC + run_with_secrets + prompt)
  - Mother secrets_authority_backend is the sole vault/identity code path
  - Every patina secrets CLI command works identically (same flags, same output, same behavior)
  - age crate stays (no crypto changes)
  - Keychain stays in Mother (no extraction)
  - --global flag works
  - Project vault works
  - Session cache works
  - patina secrets run injects secrets correctly (local and SSH)
  - cargo check and all secrets tests pass
---
# refactor: Move vault to Mother — 1:1 parity, thin CLI

> Move all vault/identity/crypto code to Mother. CLI becomes thin IPC client. Keep age crate, keep keychain in Mother, keep all features. Zero behavior change.

## Problem

The vault/identity/crypto stack is duplicated:

- `src/secrets/` — 2,830 lines across 9 files (CLI process)
- `mother/src/secrets_authority_backend/` — 1,623 lines across 9 files (daemon)

The CLI copy is almost entirely unused. Every write operation already dispatches
to Mother via `dispatch_secrets_authority`. Only 5 direct `secrets::` calls
remain in the CLI command handler:

```
secrets::prompt_for_value(name)                    — terminal I/O (keep)
secrets::run_with_secrets_ssh(root, host, command)  — needs vault access (fix)
secrets::run_with_secrets(root, command)             — needs vault access (fix)
secrets::get_global_secret("claude-oauth")           — vault read bypass (fix)
secrets::prompt_for_value("claude-oauth")            — terminal I/O (keep)
```

This duplication caused a live identity drift bug (session 20260402): three
different identities across global vault, project vault, and identity.enc.

## Goal

Delete the CLI vault/identity/crypto code. Mother is the sole vault code path.
CLI becomes thin IPC. **Everything else stays the same:**

- Same age crate
- Same keychain in Mother
- Same `--global` flag
- Same project vault support
- Same session cache
- Same output, same behavior, same flags

This is a code move, not a redesign.

## Status

Draft. No blockers. Stopgap identity mismatch fix already committed.

## Non-Goals

- Replacing the age crate
- Extracting keychain to a child
- Eliminating project vaults
- Removing session cache
- Changing vault file locations
- Changing any CLI flag or output

## Current State

```
src/secrets/
  mod.rs            632 lines  — public API, helpers, run_with_secrets
  vault.rs          342 lines  — age encrypt/decrypt (DUPLICATE)
  identity.rs       195 lines  — key management (DUPLICATE)
  storage.rs        193 lines  — orchestration (DUPLICATE)
  encrypted_file.rs 500 lines  — ChaCha20 identity file (DUPLICATE)
  keychain.rs       280 lines  — macOS Keychain (DUPLICATE)
  recipients.rs     150 lines  — recipient parsing (DUPLICATE)
  registry.rs       265 lines  — secrets registry (DUPLICATE)
  session.rs        273 lines  — session cache client (KEEP)

src/commands/secrets.rs — 14 CLI handlers, 5 direct secrets:: calls
```

What the CLI commands actually do today:

| Command | Current path | Change |
|---------|-------------|--------|
| `patina secrets` (status) | IPC → Mother | none |
| `patina secrets add` | IPC → Mother | none |
| `patina secrets --remove` | IPC → Mother | none |
| `patina secrets setup-claude` | **direct** `secrets::get_global_secret` + IPC | fix: IPC only |
| `patina secrets run` | **direct** `secrets::run_with_secrets` | fix: IPC for vault, local for spawn |
| `patina secrets --export-key` | IPC → Mother | none |
| `patina secrets --import-key` | IPC → Mother | none |
| `patina secrets --reset` | IPC → Mother | none |
| `patina secrets --lock` | IPC → Mother | none |
| `patina secrets add-recipient` | IPC → Mother | none |
| `patina secrets remove-recipient` | IPC → Mother | none |
| `patina secrets list-recipients` | IPC → Mother | none |
| `patina secrets check` | scanner (no vault) | none |
| `patina secrets audit` | scanner (no vault) | none |

Only 2 of 14 commands need changes. The rest are already IPC.

## Target State

```
src/secrets/
  mod.rs      ~120 lines — run_with_secrets (IPC), prompt_for_value, shell_join
  session.rs   273 lines — session cache client (unchanged, talks to Mother serve endpoint)
  scanner.rs         — secret detection in staged/tracked files (unchanged, no vault)

src/commands/secrets.rs — same 14 handlers, 0 direct vault calls
```

Delete: vault.rs, identity.rs, storage.rs, encrypted_file.rs, keychain.rs,
recipients.rs, registry.rs (~1,925 lines removed).

Slim: mod.rs from 632 → ~120 lines (~512 lines removed).

Total removed: ~2,437 lines. Total remaining: ~393 lines + scanner.

## Solution

### Fix 1: setup_claude vault read bypass

`src/commands/secrets.rs:518` calls `secrets::get_global_secret("claude-oauth")`
directly. Replace with `mother::get_global_secret("claude-oauth")` which already
exists at `src/mother/mod.rs:136` and does the same thing via IPC.

One line change. No new code.

### Fix 2: run_with_secrets vault access

`run_with_secrets()` at `src/secrets/mod.rs:258` calls `load_all_secrets()`
which decrypts vaults directly. Two options:

The current IPC surface cannot achieve parity: `get_global_secret` only reads
the global vault. Project vault secrets aren't reachable through any existing
operation. N+1 calls via Status + GetGlobalSecret would silently drop project
secrets — breaking parity.

**Minimal protocol addition required:** Add one operation to
`SecretsAuthorityOperation`:

```rust
LoadSecretsEnvMap {
    project_root: Option<String>,
}
```

Mother handler: decrypt global vault + project vault (if project_root provided),
merge with registries (project overrides global), return `{env_var: value}` map.
Single IPC call, exact parity with current `load_all_secrets` + `load_env_mappings`.

This is the smallest protocol change that achieves parity. It adds one match
arm to `from_payload`, one handler in the API, and one backend method that
reuses existing `decrypt_vault` + registry code already in Mother.

### Fix 3: Delete CLI vault code

Delete 7 files. Update mod.rs: remove submodule declarations, remove dead
functions, remove dead pub-use re-exports. Keep: `run_with_secrets`,
`run_with_secrets_ssh`, `prompt_for_value`, `shell_join`, session.rs.

### Fix 4: Clean root Cargo.toml

Remove deps from root Cargo.toml that were only used by deleted src/secrets/
modules. Verify each with `cargo tree` before removing. The age crate stays
in mother/Cargo.toml — it's Mother's dependency now, not the CLI's.

## Implementation Order

1. Add LoadSecretsEnvMap to protocol + Mother handler + backend
2. Fix setup_claude (1 line)
3. Rewrite run_with_secrets to use LoadSecretsEnvMap IPC
4. Delete 7 files from src/secrets/
5. Slim mod.rs
6. Clean root Cargo.toml
7. Run full test suite

## Resolved Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| age crate | Keep | Not this spec's problem. Consolidation first. |
| Keychain | Stay in Mother | Mother mediates credentials (canon rule 1). cfg-gated on macOS. |
| --global flag | Keep working | Zero behavior change. |
| Project vault | Keep working | Zero behavior change. |
| Session cache | Keep working | Zero behavior change. |
| run_with_secrets bridge | LoadSecretsEnvMap (one new op) | N+1 can't reach project vault. Minimal addition for parity. |
| Re-exported types | Check consumers, move if needed | Can't break external code. |
| Vault file locations | No change | ~/.patina/vault.age stays. .patina/vault.age stays. |

## Verification

**Before/after parity tests (every one must produce identical output):**

```bash
# Status
patina secrets

# Add + remove
echo "testval" | patina secrets add smoke --stdin --global
patina secrets
patina secrets --remove smoke --global

# Setup claude (prompt only, ctrl-C)
patina secrets setup-claude

# Run with secrets (if any exist)
patina secrets run -- env | grep -c .

# Export/import (confirm flag)
patina secrets --export-key --stdout --confirm | head -1

# Scanner
patina secrets check
patina secrets audit

# Recipients
patina secrets list-recipients
```

**Code verification:**
```bash
cargo check
cargo test -- secrets            # all 36 pass
grep -r "vault::" src/secrets/   # nothing
grep -r "identity::" src/secrets/ # nothing
grep -r "keychain::" src/secrets/ # nothing
wc -l src/secrets/*.rs           # ~393 total (down from 2830)
```

## Exit Criteria

- [ ] `src/secrets/` has zero vault/identity/crypto modules
- [ ] Mother `secrets_authority_backend` is the sole vault code path
- [ ] Every `patina secrets` command: same flags, same output, same behavior
- [ ] age crate stays
- [ ] Keychain stays in Mother
- [ ] `--global` works
- [ ] Project vault works
- [ ] Session cache works
- [ ] `patina secrets run` injects correctly (local + SSH)
- [ ] `cargo check` and all tests pass

## Build Readiness

- [x] Full codebase audit: 5 direct `secrets::` calls identified
- [x] 12 of 14 CLI commands already use IPC (zero changes needed)
- [x] `mother::get_global_secret` exists as IPC wrapper
- [x] `dispatch_secrets_authority` pattern is well-established
- [x] Stopgap identity mismatch fix committed
- [ ] Verify re-exported types have no external consumers
