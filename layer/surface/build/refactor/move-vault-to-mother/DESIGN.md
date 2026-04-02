# Design: Move vault to Mother — 1:1 parity, thin CLI

## Why This Design

The CLI has 2,830 lines of vault/identity/crypto code. 12 of 14 CLI commands
already ignore it and talk to Mother via IPC. Only 2 commands touch the CLI
copy directly. Fixing those 2 and deleting the rest eliminates 2,437 lines and
the entire class of identity-drift bugs — without changing any user-facing
behavior, any dependency, or any protocol.

This is the smallest possible move that makes Mother the sole vault authority.
Everything else (age replacement, single vault, keychain extraction) becomes
optional follow-up work on a clean foundation.

## Build Target

Delete 7 files, slim 1, fix 2 call sites, add 1 protocol operation.
~2,437 lines removed. One new IPC operation (`LoadSecretsEnvMap`) that
reuses existing backend methods. No new dependencies. No new files in CLI.

## Prior Art

- **Session 20260402**: Live identity drift — three different identities across
  global vault, project vault, identity.enc. Caused by two independent vault
  code paths that can each generate identities.
- **Codebase audit (this session)**: 10 files import from secrets module. 8 of
  10 already use IPC. Only `src/commands/secrets.rs` has 5 direct calls, of
  which 2 are `prompt_for_value` (terminal I/O, keep) and 3 need fixing.
- **Session 20260222-054702**: Keychain never works over SSH. The CLI-side
  keychain.rs is doubly dead. But we're not extracting it — just deleting
  the CLI's copy. Mother's copy stays.

## Resolved Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| IPC bridge approach | N+1 calls via existing operations | Zero protocol changes |
| Re-exported types | Verify, then remove or alias | Check before deleting |
| Scanner import | Uses `patina::scanner`, not `secrets::` | No change needed |
| Session import | `session.rs` stays in `src/secrets/` | Session cache client is CLI-side concern |

### What stays in src/secrets/mod.rs (~120 lines)

```rust
// IPC helpers
fn dispatch_load_all_secrets(project_root: Option<&Path>) -> Result<HashMap<String, String>>
fn dispatch_load_env_mappings(project_root: Option<&Path>) -> Result<HashMap<String, String>>

// Subprocess injection (uses IPC helpers above, not vault directly)
pub fn run_with_secrets(project_root: Option<&Path>, command: &[String]) -> Result<i32>
pub fn run_with_secrets_ssh(project_root: Option<&Path>, host: &str, command: &[String]) -> Result<i32>

// Terminal I/O
pub fn prompt_for_value(name: &str) -> Result<String>

// Utility
fn shell_join(args: &[String]) -> String
```

### What stays in src/secrets/session.rs (273 lines, unchanged)

Session cache client. Talks to Mother's serve endpoint. No vault code.

## Commits

### 1. `feat(protocol): add LoadSecretsEnvMap to SecretsAuthorityOperation`

Add variant to `crates/patina-protocol/src/lib.rs`:
```rust
LoadSecretsEnvMap {
    project_root: Option<String>,
},
```

Add `from_payload` / `into_payload` handling (match arm for `"load_secrets_env_map"`).

Add Mother backend method in `mother/src/secrets_authority_backend/mod.rs`:
```rust
pub fn load_secrets_env_map(project_root: Option<&Path>) -> Result<HashMap<String, String>> {
    let mut env_map = HashMap::new();

    // Global vault + registry
    let global_vault_path = paths::secrets::vault_path();
    let global_registry_path = paths::secrets::registry_path();
    if global_vault_path.exists() {
        let vault_data = vault::decrypt_vault(&global_vault_path)?;
        let reg = registry::SecretsRegistry::load_from(&global_registry_path).unwrap_or_default();
        for (name, value) in &vault_data.values {
            if let Some(env_var) = reg.get_env(name) {
                env_map.insert(env_var.to_string(), value.clone());
            }
        }
    }

    // Project vault + registry (overrides global)
    if let Some(root) = project_root {
        let project_vault_path = paths::secrets::project_vault_path(root);
        let project_registry_path = paths::secrets::project_registry_path(root);
        if project_vault_path.exists() {
            let vault_data = vault::decrypt_vault(&project_vault_path)?;
            let reg = registry::SecretsRegistry::load_from(&project_registry_path).unwrap_or_default();
            for (name, value) in &vault_data.values {
                if let Some(env_var) = reg.get_env(name) {
                    env_map.insert(env_var.to_string(), value.clone());
                }
            }
        }
    }

    Ok(env_map)
}
```

Add API handler in `mother/src/secrets_authority_api.rs`:
```rust
"load_secrets_env_map" => match backend.load_secrets_env_map(project_root) {
    Ok(secrets) => HttpResponse::json(200, &serde_json::json!({
        "status": "ok",
        "secrets": secrets,
        "count": secrets.len(),
    })),
    Err(error) => json_error(400, &error.to_string()),
}
```

This reuses existing `decrypt_vault` and `SecretsRegistry` — no new crypto,
no new file formats. Just wiring existing backend methods into a new operation.

### 2. `refactor(secrets): route setup_claude check through mother IPC`

One line:
```
src/commands/secrets.rs:518
- let replacing_hint = matches!(secrets::get_global_secret("claude-oauth"), Ok(Some(_)));
+ let replacing_hint = matches!(mother::get_global_secret("claude-oauth"), Ok(Some(_)));
```

Verify: `patina secrets setup-claude` shows same prompt. `mother` already
imported at line 7.

### 3. `refactor(secrets): rewrite run_with_secrets to use LoadSecretsEnvMap`

Replace `load_all_secrets()` + `load_env_mappings()` calls in `run_with_secrets`
and `run_with_secrets_ssh` with a single IPC call:

```rust
fn load_secrets_via_ipc(project_root: Option<&Path>) -> Result<HashMap<String, String>> {
    let response = dispatch_secrets_authority(
        "load_secrets_env_map",
        serde_json::json!({}),  // project_root added by build_authority_payload
    )?
    .ok_or_else(|| anyhow::anyhow!("Missing secrets authority response"))?;
    
    let secrets: HashMap<String, String> = response
        .get("secrets")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    
    Ok(secrets)
}
```

`run_with_secrets` becomes: call `load_secrets_via_ipc`, inject env vars, spawn.
The returned map is already env_var→value (not name→value), so no registry
lookup needed on the CLI side.

**Session cache**: `session::get_secrets_with_cache(|| load_secrets_via_ipc(root))`
still works — it caches the IPC result, not the vault decrypt. No session.rs
changes needed.

### 3. `refactor(secrets): delete CLI vault/identity/crypto stack`

```
git rm src/secrets/vault.rs         # 342 lines
git rm src/secrets/identity.rs      # 195 lines
git rm src/secrets/storage.rs       # 193 lines
git rm src/secrets/encrypted_file.rs # 500 lines
git rm src/secrets/keychain.rs      # 280 lines
git rm src/secrets/recipients.rs    # 150 lines
git rm src/secrets/registry.rs      # 265 lines
```

Total: 1,925 lines deleted.

### 4. `refactor(secrets): slim mod.rs to thin IPC client`

Remove from `src/secrets/mod.rs`:
- `mod vault;` through `mod registry;` — 7 submodule declarations
- `pub use self::vault::VaultStatus;` — re-export from deleted module
- `pub use self::identity::IdentitySource;` — re-export from deleted module
- `pub use self::registry::{infer_env_name, is_valid_env_name, is_valid_secret_name};` — re-exports
- `pub struct SecretsStatus` + `pub struct VaultStatus` — dead structs
- `pub fn check_status()` — dead (CLI uses dispatch_secrets_authority)
- `pub struct AddResult` + `pub fn add_secret()` — dead
- `pub fn remove_secret()` — dead
- `pub fn lock_session()` — dead
- `pub fn export_identity()` — dead
- `pub fn import_identity()` — dead
- `pub fn reset_identity()` — dead
- `pub fn add_recipient()` — dead
- `pub fn remove_recipient()` — dead
- `pub fn list_recipients()` — dead
- `pub fn get_global_secret()` — dead (replaced by mother::get_global_secret)
- Old `load_all_secrets()` and `load_env_mappings()` — replaced in commit 2

**Before deleting re-exports**, verify no external consumer:
```bash
grep -r "secrets::IdentitySource\|secrets::VaultStatus\|secrets::infer_env_name\|secrets::is_valid_env_name\|secrets::is_valid_secret_name" src/ --include="*.rs"
```

If any hit outside of src/secrets/mod.rs itself, move the type to
patina-protocol or provide a thin re-export from Mother.

632 → ~120 lines. ~512 lines removed.

### 5. `deps: remove CLI-only deps from root Cargo.toml`

After deletion, check what's unused:
```bash
# These were used by src/secrets/ vault code — may now be unused in root
cargo tree -p patina-ai -e features | grep age
cargo tree -p patina-ai -e features | grep chacha20poly1305
cargo tree -p patina-ai -e features | grep hkdf
```

Remove from root Cargo.toml any dep that only the deleted modules consumed.
These deps stay in mother/Cargo.toml — they're Mother's now.

`sha2` likely stays (used for ONNX model integrity outside secrets).
`rand` likely stays (used elsewhere).
`zeroize` likely stays (used by session.rs or elsewhere).

## Direct Code Targets

### Commit 1 (LoadSecretsEnvMap)
- `crates/patina-protocol/src/lib.rs:69-109` — add variant to enum
- `crates/patina-protocol/src/lib.rs:123-167` — add from_payload match arm
- `mother/src/secrets_authority_backend/mod.rs` — add `load_secrets_env_map()` fn
- `mother/src/secrets_authority_api.rs` — add handler match arm
- `mother/src/secrets_authority_api.rs:57-74` — add to trait

### Commit 2 (setup_claude)
- `src/commands/secrets.rs:518` — one line change

### Commit 3 (run_with_secrets IPC)
- `src/secrets/mod.rs:258-300` — rewrite run_with_secrets
- `src/secrets/mod.rs:306-351` — rewrite run_with_secrets_ssh
- `src/secrets/mod.rs:526-568` — replace with load_secrets_via_ipc

### Commit 3
- DELETE: 7 files (see commit 3 above)

### Commit 4
- `src/secrets/mod.rs` — remove ~512 lines of dead code
- `src/lib.rs:23` — `pub mod secrets;` stays, verify re-exports

### Commit 5
- `Cargo.toml` — remove unused deps

## Verification Plan

**Per-commit:** `cargo check` + `cargo test -- secrets`

**After all commits — parity test:**

Run each command before AND after, diff the output. Must be identical:
```bash
patina secrets 2>&1
echo "parity-test" | patina secrets add parity-test --stdin --global 2>&1
patina secrets 2>&1
patina secrets --remove parity-test --global 2>&1
patina secrets list-recipients 2>&1
patina secrets check 2>&1
patina secrets audit 2>&1
```

**Code health:**
```bash
grep -r "vault::\|identity::\|keychain::\|encrypted_file::\|storage::\|recipients::\|registry::" src/secrets/ # nothing
wc -l src/secrets/*.rs  # ~393 (mod.rs ~120 + session.rs 273)
```

## Build Readiness

- [x] Every secrets:: call site identified (5 total, 2 are prompt_for_value)
- [x] 12/14 CLI commands need zero changes
- [x] mother::get_global_secret exists
- [x] dispatch_secrets_authority pattern proven
- [x] Session cache continues to work (wraps IPC, not vault)
- [ ] Re-exported types consumer check

## Open Questions

1. **session.rs dependency on secrets types**: `session.rs` may import types
   from deleted modules. Need to verify imports before commit 4.
   **Action**: `grep "use crate::secrets::" src/secrets/session.rs` before deleting.

2. **registry.get_env()**: The backend method assumes `SecretsRegistry` has a
   `get_env(name) -> Option<&str>` method. Verify this exists, or check
   if it needs to be added (it may currently be `iter()` only).
   **Action**: Check `mother/src/secrets_authority_backend/registry.rs` API.
