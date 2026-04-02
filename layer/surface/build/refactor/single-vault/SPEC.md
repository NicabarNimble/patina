---
type: refactor
id: single-vault
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
parent: mother-vault-authority
blocked_by:
  - vault-mother-consolidation
  - secrets-load-all-ipc
exit_criteria:
  - Single vault at ~/.patina/mother/vault.age
  - No project-scoped vault files (.patina/vault.age)
  - Session cache removed from Mother — each request decrypts fresh
  - --global flag deprecated (accepted with warning, no behavioral difference)
  - project_root removed from AddSecret/RemoveSecret protocol variants
  - Migration from old vault location is automatic, idempotent, and handles partial failure
  - cargo check and all secrets tests pass
---
# refactor: Single vault per Mother with decrypt-on-demand

> Eliminate project vaults, move to ~/.patina/mother/, remove session cache, deprecate --global flag. One vault, one identity, zero mismatch risk.

## Problem

**Multiple vaults cause identity mismatch.** The current design has a global
vault (`~/.patina/vault.age`) and per-project vaults (`.patina/vault.age`).
Each can be encrypted with a different recipient, creating the identity drift
bug that triggered this spec family. Belief `[[connection-secrets-live-in-global-vault]]`
already established that credentials belong in the global vault.

**Session cache is unnecessary.** The cache was added to avoid repeated Touch ID
prompts when identity lived in macOS Keychain. Session 20260222-054702 proved
Keychain never works over SSH. The pivot to encrypted file storage removed the
biometric gate. Vault decryption with `identity.enc` is sub-millisecond with
no user prompt. Caching secrets in Mother's memory is unnecessary risk for
zero performance benefit.

**`--global` flag is confusing.** With Mother as sole authority and a single
vault, the distinction between global and project secrets is meaningless.
The flag should be deprecated gracefully.

## Goal

One vault per Mother instance. No project-scoped vaults. Decrypt on demand,
serve, zeroize. `--global` accepted but meaningless.

## Status

Draft. Blocked by `vault-mother-consolidation` and `secrets-load-all-ipc`.

## Non-Goals

- Project-scoped secret namespacing (document convention, don't enforce in code)
- Cloud sync or backup of vault
- Vault sharding or size optimization

## Current State

```
~/.patina/vault.age          — global vault (may or may not exist)
~/.patina/recipient.txt      — global recipient
~/.patina/secrets.toml       — global registry
.patina/vault.age            — project vault (may or may not exist)
.patina/recipients.txt       — project recipients (note: different filename)
.patina/secrets.toml         — project registry
```

Session cache in Mother holds decrypted secrets in memory between requests.

## Target State

```
~/.patina/mother/vault.age      — the vault
~/.patina/mother/recipient.txt  — the recipient
~/.patina/mother/secrets.toml   — the registry
~/.patina/identity.enc          — identity (unchanged location)
```

No project vault files. No session cache. Each IPC request decrypts fresh.

## Solution

### Vault relocation

Update `mother/src/secrets_paths.rs`:
```rust
pub fn vault_path() -> PathBuf { patina_home().join("mother").join("vault.age") }
pub fn recipient_path() -> PathBuf { patina_home().join("mother").join("recipient.txt") }
pub fn registry_path() -> PathBuf { patina_home().join("mother").join("secrets.toml") }
```

Remove project vault path functions.

### Migration (audit agent concern #4)

**Auto-migration on first access:**
1. Mother checks if `~/.patina/mother/vault.age` exists
2. If not, checks `~/.patina/vault.age` (old global location)
3. If old exists: copy vault.age, recipient.txt, secrets.toml to new location
4. Verify new files are readable (decrypt test)
5. Delete old files only after verification
6. Log: "Migrated vault to ~/.patina/mother/"

**Idempotency:** If both exist, new location wins. No migration needed.

**Partial failure:** If copy succeeds but verify fails, delete the new copies
and leave old in place. Log error. User can retry.

**Project vault migration:** Project vaults are NOT auto-migrated. If a project
vault exists, its secrets must be manually re-added to the global vault with
`patina secrets add`. Log a warning: "Project vault at .patina/vault.age is
no longer used. Re-add secrets with: patina secrets add <name>"

**Concurrent access during migration:** Migration only happens at Mother startup
or first vault access. Mother is single-process, so no concurrency concern.

### Session cache removal

Remove `session::get_cached_secrets()` from `get_global_secret()` and any other
vault access paths. Each request:
1. Read `identity.enc` → decrypt identity
2. Read `vault.age` → decrypt vault
3. Serve response
4. Zeroize plaintext (best-effort on owned allocations)

### --global flag deprecation (audit agent concern #2)

Phase 1: Accept `--global` silently (it's always the behavior now).
Phase 2 (future): Print deprecation warning. Remove in next major version.

This resolves the audit agent's compatibility contradiction: commands still
accept `--global`, they just ignore it.

### Protocol simplification

Remove `global` and `project_root` from `AddSecret` and `RemoveSecret`:
```rust
// Before
AddSecret { name, value, env, global: bool, project_root: Option<String> }

// After
AddSecret { name, value, env }
```

**Backward compatibility (audit agent concern #10):** The protocol uses explicit
`from_payload` parsing, not serde derive. Old clients sending `global` and
`project_root` fields are safe because `from_payload` extracts them with
`unwrap_or(false)` and `optional_str_field` respectively — extra/missing fields
are handled gracefully. When these fields are removed from the enum variant,
the `from_payload` match arm simply stops extracting them. Old payloads still
parse. This must be tested: send a payload with `global: true` and
`project_root: "/some/path"` to the new handler, verify it succeeds.

## Implementation Order

1. Update secrets_paths.rs (new vault location)
2. Add migration logic (auto-migrate old → new)
3. Remove session cache
4. Deprecate --global (accept silently)
5. Simplify protocol variants (remove global/project_root)
6. Remove project vault path functions
7. Update tests

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Vault location | `~/.patina/mother/` | Mother's domain |
| identity.enc location | Unchanged (`~/.patina/`) | Shared resource, not vault-specific |
| Migration | Auto on first access, verify before deleting old | Safe, idempotent |
| Project vault migration | Manual only (warning log) | Low usage, avoid silent data movement |
| --global deprecation | Accept silently, warn later | Zero breakage now |
| Protocol compat | serde(default) for extra fields | Old clients work unchanged |
| Project-scoped naming | Document `{uid}:name` convention | YAGNI for enforcement |

### Beliefs consulted
- `[[connection-secrets-live-in-global-vault]]` — credentials belong in global vault
- `[[keychain-never-worked-ssh]]` — no Touch ID gate, no need for cache
- `[[llm-threat-model-unique]]` — transient plaintext in Mother memory is acceptable

## Verification

- Vault created at `~/.patina/mother/vault.age`
- Old vault auto-migrated on first access
- `patina secrets` shows correct status after migration
- `patina secrets add test --global` works (flag accepted)
- `patina secrets add test` works (no flag needed)
- No session cache code in hot path
- `patina secrets run -- env` works (decrypt on demand)
- Project vault warning logged when `.patina/vault.age` exists

## Exit Criteria

- [ ] Single vault at `~/.patina/mother/vault.age`
- [ ] No project vault support in code
- [ ] Session cache removed
- [ ] `--global` accepted silently
- [ ] `project_root` removed from protocol
- [ ] Migration is automatic, idempotent, handles partial failure
- [ ] Tests pass

## Build Readiness

- [x] Vault location decided
- [x] Migration contract specified (concern #4 addressed)
- [x] Compatibility strategy for --global (concern #2 addressed)
- [x] Protocol backward compat via serde(default) (concern #10 addressed)
- [ ] Blocked by vault-mother-consolidation and secrets-load-all-ipc
