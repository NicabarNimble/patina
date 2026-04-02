---
type: feat
id: secrets-load-all-ipc
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
parent: mother-vault-authority
blocked_by:
  - vault-mother-consolidation
exit_criteria:
  - LoadAllSecrets variant added to SecretsAuthorityOperation in patina-protocol
  - Mother handler decrypts vault, loads registry, returns env_var->value map
  - run_with_secrets uses single LoadAllSecrets IPC call
  - run_with_secrets_ssh uses single LoadAllSecrets IPC call
  - Temporary N+1 IPC bridge from vault-mother-consolidation replaced
  - Response contract documented (collision semantics, validation, scope)
  - cargo check and all secrets tests pass
---
# feat: Add LoadAllSecrets IPC operation

> New SecretsAuthorityOperation variant for run_with_secrets to get all secrets via Mother IPC instead of direct vault decrypt.

## Problem

After `vault-mother-consolidation`, `run_with_secrets` will use a temporary
N+1 IPC bridge: call `Status` to get secret names, then `GetGlobalSecret` for
each value. This works but is inelegant and scales linearly with secret count.

The proper solution is a single IPC operation that returns all secrets as
an env-var map, matching the 1Password `op run` pattern: agent decrypts,
returns env map, client injects into subprocess.

## Goal

Add `LoadAllSecrets` to the `SecretsAuthorityOperation` protocol. Mother
decrypts the vault once, combines with registry mappings, returns the complete
env-var-to-value map. The CLI's `run_with_secrets` becomes a single IPC call.

## Status

Draft. Blocked by `vault-mother-consolidation`.

## Non-Goals

- Filtering secrets by project (separate concern for `single-vault`)
- Streaming secrets (vault is small, full map is fine)
- Caching the response in Mother (decrypt-on-demand, see `single-vault`)

## Target Shape

```rust
// In patina-protocol
SecretsAuthorityOperation::LoadAllSecrets {
    project_root: Option<String>,  // kept for backward compat during transition
}

// Response
{
    "status": "ok",
    "secrets": {
        "GITHUB_TOKEN": "ghp_xxx",
        "CLAUDE_CODE_OAUTH_TOKEN": "sk-ant-xxx"
    },
    "count": 2
}
```

### Response contract

- **Keys**: Environment variable names (from registry mapping, e.g., `GITHUB_TOKEN`)
- **Values**: Decrypted secret values
- **Scope**: All secrets from the vault that have a registry entry with an env mapping.
  Secrets without registry entries (orphaned vault entries) are excluded.
- **Collisions**: If global and project registries map different secrets to the same
  env var name, project wins (override semantics, matching current `load_env_mappings`
  behavior). After `single-vault` spec, this becomes moot (single registry).
- **Validation**: Env var names are validated at add-time (registry enforces
  `is_valid_env_name`). No re-validation at load time.
- **Empty vault**: Returns `{"status": "ok", "secrets": {}, "count": 0}`
- **No vault**: Returns `{"status": "ok", "secrets": {}, "count": 0}` (same as empty)
- **Decrypt failure**: Returns 400 error (same as other vault operations)
- **Max payload**: No explicit limit. Practical ceiling: ~100 secrets, each <1KB.
  Total response <100KB. If this grows, revisit.

### Zeroize scope (audit agent concern #6)

"Zeroize after serve" means: Mother calls `zeroize()` on owned `String` and
`HashMap` allocations holding plaintext after the HTTP response is serialized.
This is **best-effort** — plaintext may persist in:
- serde JSON serialization buffers (transient, overwritten by next alloc)
- UDS kernel socket buffers (cleared on socket close)
- CLI process memory (dropped after subprocess spawn)

This matches the 1Password agent's guarantees. Full memory isolation would
require a separate process with `mlock`/`mprotect`, which is out of scope.

## Solution

1. Add `LoadAllSecrets` variant to `SecretsAuthorityOperation` in `patina-protocol`
2. Add `from_payload` / `into_payload` handling for the new variant
3. Add handler in `mother/src/secrets_authority_api.rs`
4. Add backend method in `mother/src/secrets_authority_backend/mod.rs`
5. Update CLI `run_with_secrets` to use single IPC call
6. Remove temporary N+1 bridge from consolidation spec

## Implementation Order

1. Protocol addition (patina-protocol)
2. Mother handler + backend method
3. CLI `run_with_secrets` rewrite
4. Remove bridge code
5. Tests

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Return format | env_var -> value map | CLI needs env vars, not secret names |
| project_root param | Keep for now | Backward compat; removed in single-vault |
| Zeroize guarantee | Best-effort on owned allocs | Industry standard, full isolation out of scope |
| Orphaned secrets | Excluded from response | Only secrets with registry entries are useful |

## Verification

- `cargo test -- secrets` pass
- `patina secrets run -- sh -c 'echo $SOME_VAR'` — correct value
- `patina secrets run --ssh host -- cmd` — correct injection via stdin
- Mother log shows single LoadAllSecrets call (not N+1)
- Empty vault: `patina secrets run -- env` runs without error

## Exit Criteria

- [ ] `LoadAllSecrets` in `SecretsAuthorityOperation`
- [ ] Mother handler returns env_var->value map
- [ ] `run_with_secrets` uses single IPC call
- [ ] `run_with_secrets_ssh` uses single IPC call
- [ ] N+1 bridge removed
- [ ] Response contract documented
- [ ] Tests pass

## Build Readiness

- [x] Current run_with_secrets code analyzed (mod.rs:258-370)
- [x] Protocol pattern established (SecretsAuthorityOperation enum)
- [x] Response contract specified (collision, validation, scope)
- [x] Zeroize guarantees scoped honestly
- [ ] Blocked by vault-mother-consolidation
