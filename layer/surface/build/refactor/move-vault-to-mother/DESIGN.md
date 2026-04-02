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

Delete 7 files, slim 1, fix 2 call sites. ~2,437 lines removed.
No new protocol operations. No new dependencies. No new files.

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

### 1. `refactor(secrets): route setup_claude check through mother IPC`

One line:
```
src/commands/secrets.rs:518
- let replacing_hint = matches!(secrets::get_global_secret("claude-oauth"), Ok(Some(_)));
+ let replacing_hint = matches!(mother::get_global_secret("claude-oauth"), Ok(Some(_)));
```

Verify: `patina secrets setup-claude` shows same prompt. `mother` already
imported at line 7.

### 2. `refactor(secrets): rewrite load_all_secrets as IPC bridge`

Replace `src/secrets/mod.rs:526-568` (`load_all_secrets` + `load_env_mappings`)
with IPC-based versions:

```rust
fn dispatch_load_all_secrets(project_root: Option<&Path>) -> Result<HashMap<String, String>> {
    // 1. Call dispatch_secrets_authority("status", {}) to get secret names + env mappings
    // 2. For each secret name, call dispatch_secrets_authority("get_global_secret", {name})
    // 3. Build name→value map
    // Note: dispatches through Mother, not direct vault access
}

fn dispatch_load_env_mappings(project_root: Option<&Path>) -> Result<HashMap<String, String>> {
    // Status response already includes secret_names
    // Registry env mappings come from status or a separate call
    // Build name→env_var map
}
```

Then update `run_with_secrets` and `run_with_secrets_ssh` to call these
instead of the old direct-decrypt helpers.

**Key detail:** The current `run_with_secrets` calls `session::get_secrets_with_cache(|| load_all_secrets(root))`.
The session cache wraps the vault access. With the IPC bridge, the session
cache still works — it caches the IPC results, not the vault decrypt. No
change to session.rs needed.

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

### Commit 1
- `src/commands/secrets.rs:518` — one line change

### Commit 2
- `src/secrets/mod.rs:258-300` — update run_with_secrets to use IPC bridge
- `src/secrets/mod.rs:306-351` — update run_with_secrets_ssh to use IPC bridge
- `src/secrets/mod.rs:526-568` — replace load_all_secrets + load_env_mappings

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

1. **N+1 bridge for run_with_secrets**: The status response includes
   `secret_names` but not values. Getting values requires one
   `get_global_secret` call per secret. For N secrets, that's N+1 IPC calls.
   
   Current protocol doesn't have a "get all secret values" operation.
   We could add one, but that's a protocol change — against the "minimum
   delta" goal of this spec.
   
   **Assessment**: N+1 over UDS is <15ms for 10 secrets. Acceptable as-is.
   A `LoadAllSecrets` operation is a clean follow-up if needed.

2. **session.rs dependency on secrets types**: `session.rs` may import types
   from deleted modules (e.g., `HashMap<String, String>` for cached secrets).
   Need to check.
   **Action**: Read session.rs imports before commit 3.
