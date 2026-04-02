# Design: Add LoadAllSecrets IPC operation

## Why This Design

`run_with_secrets` needs all secrets as env vars. After consolidation, the
temporary bridge makes N+1 IPC calls (one per secret). A single `LoadAllSecrets`
operation is cleaner, faster, and matches the `op run` pattern: one request,
one response, one injection.

## Build Target

New protocol variant, Mother handler, CLI consumer. ~60 lines of new code
across 4 files.

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Collision semantics | Project overrides global | Matches current load_env_mappings behavior |
| Response keys | Env var names, not secret names | CLI needs env vars for subprocess injection |
| Orphaned secrets | Excluded | Secrets without registry env mapping aren't injectable |

## Commits

1. `feat(protocol): add LoadAllSecrets to SecretsAuthorityOperation` —
   Add variant, from_payload/into_payload, tests.

2. `feat(mother): implement LoadAllSecrets handler` —
   Backend method: decrypt vault, load registry, build env map, zeroize.
   API handler: route, serialize response.

3. `refactor(secrets): run_with_secrets uses LoadAllSecrets` —
   Replace bridge with single `dispatch_secrets_authority("load_all_secrets")`.
   Parse response into `HashMap<String, String>`. Same for SSH variant.

4. `refactor(secrets): remove N+1 bridge code` —
   Delete the temporary `load_all_secrets` and `load_env_mappings` IPC wrappers
   from consolidation spec.

## Direct Code Targets

### Commit 1
- `crates/patina-protocol/src/lib.rs:69-109` — add `LoadAllSecrets` variant
- `crates/patina-protocol/src/lib.rs:111+` — add from_payload/into_payload cases

### Commit 2
- `mother/src/secrets_authority_api.rs` — add `"load_all_secrets"` match arm
- `mother/src/secrets_authority_backend/mod.rs` — add `load_all_secrets()` fn:
  ```rust
  fn load_all_secrets(project_root: Option<&Path>) -> Result<HashMap<String, String>> {
      // decrypt vault(s), load registry/registries, build env_var -> value map
      // zeroize vault_data after building map
  }
  ```

### Commit 3
- `src/secrets/mod.rs:258-300` — rewrite `run_with_secrets()`
- `src/secrets/mod.rs:306-351` — rewrite `run_with_secrets_ssh()`
- `src/commands/secrets.rs` — add response struct for LoadAllSecrets

### Commit 4
- `src/secrets/mod.rs` — remove bridge functions

## Verification Plan

```bash
# Add a test secret
echo "hello" | patina secrets add test-ipc --stdin --global

# Verify injection
patina secrets run -- sh -c 'echo $TEST_IPC'
# Expected: hello

# Verify SSH path (if SSH available)
patina secrets run --ssh localhost -- sh -c 'echo $TEST_IPC'

# Cleanup
patina secrets --remove test-ipc --global

# Empty vault
patina secrets run -- echo "no secrets injected"
# Expected: runs without error
```

## Build Readiness

- [x] Protocol pattern understood
- [x] Response contract specified
- [x] Zeroize scope documented
- [ ] Blocked by vault-mother-consolidation

## Open Questions

None — this is a well-scoped protocol addition with clear contract.
