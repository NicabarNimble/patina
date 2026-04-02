# Design: Single vault per Mother with decrypt-on-demand

## Why This Design

Multiple vaults with separate recipients caused the identity drift bug.
Session cache solved a Touch ID problem that no longer exists. The `--global`
flag is meaningless when Mother owns one vault. Simplifying to one vault with
decrypt-on-demand eliminates entire categories of bugs and complexity.

## Build Target

`~/.patina/mother/vault.age` as the single vault. Auto-migration from old
location. No session cache. ~100 lines changed across paths, cache removal,
and migration logic.

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Migration trigger | First vault access | Lazy, no startup cost if vault unused |
| Migration verification | Decrypt test before deleting old | Prevent data loss |
| Project vault handling | Warning only, no auto-migrate | Low usage, manual is safer |

## Commits

1. `refactor(mother): move vault paths to ~/.patina/mother/` —
   Update `secrets_paths.rs`. Remove project vault path functions.

2. `feat(mother): auto-migrate vault from old location` —
   In vault access path: check new location, fall back to old, copy+verify,
   delete old. Idempotent. Log migration.

3. `refactor(mother): remove session cache` —
   Remove `session::get_cached_secrets()` calls. Each request decrypts fresh.
   Delete or slim session module.

4. `refactor(secrets): deprecate --global flag` —
   Accept the flag in CLI but don't pass it through. No warning yet (silent
   acceptance). Remove `global: bool` from protocol variants. Use
   `#[serde(default)]` for backward compat.

5. `refactor(mother): warn on project vault presence` —
   When Mother sees `.patina/vault.age` in a project root, log warning
   suggesting manual re-add.

## Direct Code Targets

### Commit 1
- `mother/src/secrets_paths.rs:20-38` — update vault/recipient/registry paths,
  remove project_vault_path, project_recipients_path, project_registry_path

### Commit 2
- `mother/src/secrets_authority_backend/vault.rs` — add migration check in
  decrypt_vault or add_secret before first vault access
- `mother/src/secrets_authority_backend/mod.rs` — migration entry point

### Commit 3
- `mother/src/secrets_authority_backend/mod.rs:244-246` — remove cache check
  in get_global_secret
- `mother/src/secrets_authority_backend/session.rs` — delete or strip

### Commit 4
- `src/commands/secrets.rs` — remove `--global` from flag routing
- `crates/patina-protocol/src/lib.rs:73-84` — remove `global`, `project_root`
  from AddSecret/RemoveSecret, add `#[serde(default)]` annotations
- `mother/src/secrets_authority_api.rs` — simplify dispatch (no global/project logic)

## Verification Plan

```bash
# Migration test
mkdir -p ~/.patina
echo "test" > ~/.patina/vault.age  # simulate old vault
# Run any secrets command → should migrate
patina secrets
ls ~/.patina/mother/vault.age  # should exist
ls ~/.patina/vault.age          # should be gone

# Decrypt-on-demand
PATINA_LOG=1 patina secrets run -- env 2>&1 | grep cache  # no cache hits

# --global compat
patina secrets add test --global --stdin <<< "val"  # works, no error
patina secrets --remove test                         # works without --global
```

## Build Readiness

- [x] Migration contract specified
- [x] Compatibility strategy designed
- [ ] Blocked by vault-mother-consolidation + secrets-load-all-ipc

## Open Questions

1. **Migration timing**: Should migration happen at Mother daemon startup or
   lazily on first vault access? Lazy is simpler and has no startup cost.
   **Recommendation**: Lazy.

2. **identity.enc location**: Should this also move to `~/.patina/mother/`?
   Currently at `~/.patina/identity.enc`. It's conceptually Mother's resource.
   **Recommendation**: Leave at `~/.patina/` for now. Moving it requires
   updating encrypted_file.rs in Mother AND handling the keychain-child's
   expectations. Separate concern.
