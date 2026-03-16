# Design: runtime-hardening

## Commit Plan

8 commits, each self-contained and testable. Commits 1-4 are independent bug fixes, 5 is docs, 6-7 are structural cleanup, 8 is verification.

### Commit 1: `fix(broker): fail-safe child cleanup on Drop — prevent orphan leak`

**Problem:** `child.fetch()?` in `broker/mod.rs:94` early-returns, skipping `child.shutdown()`. Dropped `NativeChild` detaches the child process. Rust's `Drop` on `std::process::Child` does NOT kill — it detaches.

**API constraints:**
- `ChildConnection::wait(mut self)` consumes self — cannot call from `Drop(&mut self)`
- `ChildConnection::kill(&mut self)` — fine for Drop
- `shutdown()` calls `conn.request("pipe/shutdown")` which does I/O — can block if child is wedged

**Design principle:** Drop is a safety net, not the happy path. Drop must not block on I/O. Graceful `pipe/shutdown` remains the caller's responsibility via explicit `shutdown()`.

**Changes to `crates/patina-pipe/src/harness.rs`:**

```rust
/// Get the child process PID (for test verification and logging).
pub fn pid(&self) -> u32 {
    self.process.id()
}

/// Fail-safe cleanup: kill the child process and reap it.
///
/// Does NOT attempt graceful pipe/shutdown — this is for Drop and
/// error-path cleanup where the child may be wedged. Non-consuming
/// (takes &mut self, not self) so it works from Drop.
pub fn cleanup(&mut self) {
    // Kill — may fail if already exited, that's fine
    let _ = self.process.kill();
    // Reap — try_wait is non-blocking, prevents zombie
    let _ = self.process.try_wait();
}
```

**Changes to `src/broker/lifecycle.rs`:**

```rust
impl Drop for NativeChild {
    fn drop(&mut self) {
        self.conn.cleanup();
    }
}
```

**Test:** Spawn test-child via harness, wrap in NativeChild, record `pid()`, drop without calling shutdown, verify `kill(pid, 0)` fails (process is gone).

---

### Commit 2: `fix(connect): atomic creation — vault first, rollback on failure`

**Problem:** `store.rs` writes TOML at line 113, then vault at line 118. Vault failure leaves orphan TOML.

**Refresh hazard:** `refresh_connection()` in `commands/connect.rs:280` calls `create()` to overwrite both vault and TOML. If we switch to vault-first and TOML write then fails, naive rollback (`remove_secret`) would delete the *new* secret — but the *old* secret was already overwritten by `add_secret`. The user loses both credentials.

**Change:** In `src/connect/internal/store.rs::create()`:

```rust
pub(crate) fn create(record: &ConnectionRecord, credential: &str) -> Result<(), ConnectError> {
    let dir = paths::connections::connections_dir();
    let path = paths::connections::connection_path(&record.identity.name);

    // Ensure connections directory exists
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| ConnectError::IoError { ... })?;
    }

    // Snapshot previous credential if this is an update (for rollback)
    let previous_credential = crate::secrets::get_global_secret(&record.auth.secret_ref)
        .ok()
        .flatten();

    // 1. Write vault FIRST (requires Touch ID — most likely failure point)
    crate::secrets::add_secret(
        &record.auth.secret_ref, credential, None, true, None,
    ).map_err(|e| ConnectError::IoError { ... })?;

    // 2. Write TOML (less likely to fail — just fs::write)
    let toml_str = toml::to_string_pretty(record).map_err(|e| ConnectError::IoError { ... })?;
    if let Err(e) = fs::write(&path, &toml_str) {
        // Rollback: restore previous credential or remove new one
        match previous_credential {
            Some(old_value) => {
                // Update path: restore old secret
                let _ = crate::secrets::add_secret(
                    &record.auth.secret_ref, &old_value, None, true, None,
                );
            }
            None => {
                // Create path: remove new secret
                let _ = crate::secrets::remove_secret(&record.auth.secret_ref, true, None);
            }
        }
        return Err(ConnectError::IoError {
            detail: format!("writing {} (vault rolled back): {}", path.display(), e),
        });
    }

    Ok(())
}
```

**Note:** The snapshot reads the vault (may trigger Touch ID once) but since we're about to write to it anyway, this is not an extra prompt — `add_secret` also decrypts.

**Test:** Verify vault-first order. Verify that the create vs update rollback paths are distinct.

---

### Commit 3: `fix(connect): compute_status — Connected after create, Missing when registry entry absent`

**Problem:** `last_validated` is never set, so `compute_status` always returns `Unchecked`. `Missing` is never produced.

**Changes:**

**A) `src/commands/connect.rs` — set `last_validated` conditionally on create:**
```rust
// Line ~136: change from
last_validated: None,
// to — only set when account probe succeeded (credential was live-validated)
last_validated: if result.account_id.is_some() { Some(now.clone()) } else { None },
```

Rationale: OAuth flow always probes the account (`GET /user`), so `account_id` is always `Some` on success — `last_validated` gets set. Manual flow calls `probe_account` too, but it can fail (bad token, network issue) — `account_id` is `None` in that case, so `last_validated` stays `None` and the connection shows as `Unchecked`. This is honest: "stored" is not "validated."

**B) `src/connect/internal/store.rs` — registry existence check in `compute_status`:**

```rust
fn compute_status(record: &ConnectionRecord) -> ConnectionStatus {
    if record.auth.last_error.is_some() {
        return ConnectionStatus::Errored;
    }
    if let Some(expires_at) = &record.auth.expires_at {
        let now = chrono::Utc::now().to_rfc3339();
        if expires_at < &now {
            return ConnectionStatus::Expired;
        }
    }
    // Check secrets REGISTRY for secret_ref (NO decryption, NO Touch ID).
    // This checks the secrets.toml index, not the encrypted vault.age.
    // Limitation: registry could be stale (entry exists but vault corrupted).
    // Acceptable: resolve_auth() is the real runtime gate (fail-closed).
    if !registry_has_secret(&record.auth.secret_ref) {
        return ConnectionStatus::Missing;
    }
    if record.identity.last_validated.is_none() {
        return ConnectionStatus::Unchecked;
    }
    ConnectionStatus::Connected
}

/// Check if a secret name exists in the global secrets registry.
///
/// Reads ~/.patina/secrets.toml (the index file, NOT the encrypted vault).
/// Returns false if registry doesn't exist or secret isn't listed.
fn registry_has_secret(secret_ref: &str) -> bool {
    let registry_path = crate::paths::secrets::registry_path();
    if !registry_path.exists() {
        return false;
    }
    match crate::secrets::registry::SecretsRegistry::load_from(&registry_path) {
        Ok(reg) => reg.contains(secret_ref),
        Err(_) => false, // Can't read registry — assume missing
    }
}
```

**Note on exit criterion wording:** The SPEC.md says "Missing when registry entry absent" not "vault entry absent" because we're checking the registry index, not decrypting the vault. This is deliberately weaker — `resolve_auth` catches vault corruption at runtime.

**Test:** `compute_status` with `last_validated: Some(...)` and registry entry present → `Connected`. Registry entry absent → `Missing`. `last_validated: None` with registry entry present → `Unchecked`. Also: `account_id: None` on manual create → `last_validated` stays `None`.

---

### Commit 4: `fix(broker): source_id uses source name, not child name`

**Problem:** `routing.rs:94` uses `format!("child:{}", child_name)`. Two sources sharing a child collide.

**Changes:**

**A) `src/broker/routing.rs` — add `source_name` parameter to `validate_fact`:**

```rust
pub fn validate_fact(
    fact: &BrokerFact,
    manifest: &ChildManifest,
    child_name: &str,
    source_name: &str,       // NEW
    project_root: &Path,
    schema_cache: &mut HashMap<String, HashSet<String>>,
) -> Result<ValidatedFact> {
    // ...
    let source_id = format!("source:{}", source_name);  // CHANGED
    // ...
}
```

**B) `src/broker/mod.rs` — pass source name through:**

In `write_to_project()`, change the `validate_fact` call to pass `&source.name`.

In `status()`, change the fact count query:
```rust
// Support both old and new prefixes during transition
let source_id_new = format!("source:{}", source.name);
let source_id_old = format!("child:{}", child_name);
let fact_count: i64 = events_conn
    .query_row(
        "SELECT COUNT(*) FROM eventlog WHERE source_id IN (?1, ?2)",
        [&source_id_new, &source_id_old],
        |row| row.get(0),
    )
    .unwrap_or(0);
```

**Test:** Two `ValidatedFact` values from different sources using the same child → distinct `source_id`.

---

### Commit 5: `fix(docs): document sandbox enforcement gap honestly`

**Files:**
- `src/broker/spawn.rs:174` — change log message to include `(NOT YET ENFORCED)`
- `src/broker/spawn.rs:184` — expand comment to state children run unrestricted until sandbox enforcement ships
- `crates/patina-pipe/src/harness.rs:1` — change "test harness" to "harness"

No behavioral changes. Documentation-only.

---

### Commit 6: `refactor: move resolve_child_binary out of broker`

**Problem:** `connect/internal/resolve.rs:23` calls `crate::broker::spawn::resolve_child_binary()`. Connect imports broker, broker imports connect — intra-crate cycle.

**Change:** Extract `resolve_child_binary()` from `src/broker/spawn.rs` to a shared module.

**Candidates for new location:**
- `src/children.rs` — small utility module: "resolve child binary by name." Clear "Do X."
- `paths::children::resolve_binary()` — fits the "where things live" pattern but adds I/O to a module that's currently pure path construction.

**Recommendation:** `src/children.rs` — it's a new module with one public function. `paths.rs` is intentionally I/O-free.

```rust
// src/children.rs
//! Child binary resolution.
//!
//! Resolves child binary paths using the search order from DESIGN.md §7.

use anyhow::{bail, Result};
use std::path::PathBuf;

/// Resolve a child binary by name.
///
/// Search order:
/// 1. ~/.patina/children/{name}/{name} — installed children
/// 2. PATH — system-installed children
/// 3. ./target/release/{name} — development builds
pub fn resolve_binary(name: &str) -> Result<PathBuf> {
    // ... (moved from broker/spawn.rs)
}
```

**Update imports:**
- `src/broker/spawn.rs`: `use crate::children;` → `children::resolve_binary(&auth_plan.child)`
- `src/connect/internal/resolve.rs`: `use crate::children;` → `children::resolve_binary(&record.auth.child)`
- `src/lib.rs`: add `pub mod children;`

**Test:** Existing `resolve_nonexistent_binary` test moves to `src/children.rs`.

---

### Commit 7: `fix(broker): reject lake destination at parse time`

**Problem:** `broker/mod.rs:148` calls `bail!()` for `Destination::Lake` at runtime. User gets a crash with no guidance when running `patina mother run` on a lake-configured source.

**Change:** In `src/broker/sources.rs::parse_destination()`:

```rust
"lake" => {
    let name = dest.lake.ok_or_else(|| {
        anyhow::anyhow!(
            "source '{}': destination type 'lake' requires a 'lake' field",
            source_name
        )
    })?;
    // Lake destinations are not yet supported — reject at parse time
    // with an actionable message. Keep Destination::Lake in the enum
    // for forward compatibility.
    anyhow::bail!(
        "source '{}': lake destination '{}' is not yet supported.\n  \
         Use destination.type = \"project\" or remove the [destination] section.\n  \
         Lake support is planned — see spec-lakehouse.",
        source_name,
        name
    )
}
```

**Also:**
- Remove `route_to_lake()` stub from `src/broker/mod.rs` (dead code once parse rejects it).
- Keep `Destination::Lake` variant in the enum — it's the correct data model, and removing it would break the existing tests that verify lake parsing. The rejection moves from runtime to parse time.

**Test:** Update existing `parse_source_lake_destination` test to expect an error instead of success. Add test verifying the error message is actionable.

---

### Commit 8: `test: verify all hardening fixes + pre-push`

Run `./resources/git/pre-push-checks.sh`. Ensure no regressions across all 8 checks.

## Risk

- **Commit 1 (Drop):** Drop does NOT attempt `pipe/shutdown` — it goes straight to `kill()`+`try_wait()`. This is fail-safe: no I/O blocking, no hang risk. The child process gets SIGKILL, not a graceful close. Acceptable: Drop is the safety net, explicit `shutdown()` in the normal path handles graceful close.
- **Commit 2 (refresh rollback):** Snapshot reads the vault before write (one Touch ID prompt). If the snapshot itself fails, we fall back to no-rollback behavior (same as today). The rollback path re-encrypts the old secret, which is a second vault write — if *that* fails, we log and accept the loss. This is best-effort, not transactional.
- **Commit 3 (registry check):** Checking the registry file rather than decrypting avoids Touch ID, but the registry could be stale (entry listed but vault corrupted, or entry missing but vault has it). Acceptable: `resolve_auth` is the real runtime gate (fail-closed). The status is informational.
- **Commit 4 (source_id migration):** Existing facts keep `child:` prefix. The transition query handles both. Users who want clean counts can re-sync.
- **Commit 6 (resolve_binary move):** Purely mechanical refactor. Risk is low — function moves, imports change, behavior identical.
- **Commit 7 (lake rejection):** This changes behavior — lake configs that previously loaded (then crashed at runtime) now fail at parse time. This is strictly better. Existing lake-destination tests need updating.

## Session Plan

One session, 8 commits. Each commit is independently testable. Commit 8 runs after all others.
