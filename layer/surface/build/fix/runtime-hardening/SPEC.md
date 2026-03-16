---
type: fix
id: runtime-hardening
status: draft
created: 2026-03-09
sessions:
  origin: 20260309-170938
related:
- patina-connect
exit_criteria:
- child process leak fixed — NativeChild Drop does fail-safe kill+reap, test proves no orphan
- connection creation atomic — vault-first, rollback-aware for both create and refresh paths
- compute_status produces Connected when account was probed, Missing when registry entry absent
- source_id uses source name not child name — two sources sharing a child get distinct counts
- sandbox gap documented — code comments and log messages state enforcement is deferred, not active
- harness.rs module doc updated — no longer says "test harness"
- resolve_child_binary moved out of broker — connect no longer imports crate::broker
- lake destination rejected at parse time with actionable message, not runtime bail
- all existing tests pass, new tests for each fix
---
# fix: Runtime Hardening — Error Paths, Atomicity, Process Cleanup, Dependency Cycle

> Fix 8 issues found in post-ship audit of broker + connect + pipe: child process leak, non-atomic connection creation, unreachable status variants, source_id collision, unenforced sandbox (document boundary), harness mislabel, connect→broker dependency cycle, lake destination runtime crash.

## Problem

The `patina-connect` spec shipped with all 9 exit criteria passing (happy path verified E2E). A post-ship audit found 8 issues across broker, connect, and pipe — mostly in error paths and status reporting:

1. **Child process leak (Medium):** `broker/mod.rs:94` — `child.fetch()?` early-returns on error, `child.shutdown()` at line 116 is never reached. Rust's `Drop` on `std::process::Child` detaches, it does not kill. Orphan child processes accumulate.

2. **Non-atomic connection creation (Medium):** `connect/internal/store.rs:113` writes TOML first, then `store.rs:118` writes vault credential. If vault write fails (Touch ID cancelled, vault corruption), a durable TOML record points at a nonexistent credential. `resolve_auth()` then fails with `CredentialMissing` — fail-closed works, but the user has a broken connection with no obvious recovery.

3. **Unreachable `ConnectionStatus::Connected` (Medium):** `compute_status()` in `store.rs:221` returns `Unchecked` when `last_validated` is `None`. New connections are created with `last_validated: None` (`commands/connect.rs:136`) and nothing in the codebase ever sets it. `Connected` is unreachable. `Missing` is defined in the model but never produced.

4. **Source ID collision (Medium):** `routing.rs:94` stamps facts with `source_id = format!("child:{}", child_name)`. `broker/mod.rs:207` counts facts by child name. Two sources sharing the same connection (same child binary) get the same `source_id`, so `patina mother sources` double-counts.

5. **Sandbox overstated (High — documentation):** `spawn.rs:184-186` computes sandbox profile but `let _ = &sandbox_profile;` discards it. `harness.rs:218` comments confirm no sandbox application. The prior audit and SPEC.md describe sandboxing as if it's enforced. This is architectural intent, not enforced reality. The fix here is documentation honesty — actual sandbox enforcement is a separate spec.

6. **Harness mislabel (Low):** `patina-pipe/src/harness.rs:1` says "test harness" but this module is the production spawn path used by `broker/spawn.rs`.

7. **Connect→broker dependency cycle (Low):** `connect/internal/resolve.rs:23` calls `crate::broker::spawn::resolve_child_binary()`. The connect module imports from the broker module, while the broker module imports from connect. Both directions exist in the same crate, so Rust allows it, but it violates dependable-rust: connect should not know about broker internals.

8. **Lake destination runtime crash (Low):** `broker/mod.rs:148` calls `bail!()` for `Destination::Lake`. A user who configures `destination.type = "lake"` in `sources.toml` gets a runtime crash on `patina mother run` with no guidance. This should be caught at parse time or at least give an actionable error.

## Root Cause

The connect spec focused on the happy path (acquisition → persistence → consumption → E2E). Error paths, process lifecycle cleanup, status correctness, and module boundary hygiene were not covered by exit criteria.

## Fix

### Commit 1: `fix(broker): fail-safe child cleanup on Drop — prevent orphan leak`

**Files:** `src/broker/lifecycle.rs`, `crates/patina-pipe/src/harness.rs`

- Add `ChildConnection::pid()` method returning the child's PID (for test verification).
- Add `ChildConnection::cleanup(&mut self)` — non-consuming: `kill()` then `try_wait()`. Does not attempt graceful `pipe/shutdown` (Drop must not block on I/O).
- Implement `Drop` for `NativeChild`: call `conn.cleanup()`. Log on error, never panic.
- Graceful `pipe/shutdown` remains the normal-path responsibility of explicit `shutdown()` calls — Drop is the safety net, not the happy path.
- Add test: spawn test-child, drop without shutdown, use `pid()` to verify process is gone.

### Commit 2: `fix(connect): atomic creation — vault first, rollback on failure`

**Files:** `src/connect/internal/store.rs`

- Reorder: write vault credential first, then write TOML.
- If TOML write fails after vault write: remove the vault entry (best-effort rollback).
- Handle refresh path: `refresh_connection()` calls `create()` which overwrites the vault entry. If TOML write then fails, naive rollback would delete the previous valid secret too. Fix: distinguish create vs update — on update, rollback restores the old secret, not deletes.
- Add test: verify write order through code inspection and TOML round-trip.

### Commit 3: `fix(connect): compute_status — Connected after create, Missing when registry entry absent`

**Files:** `src/connect/internal/store.rs`, `src/commands/connect.rs`

- Set `last_validated` to `now` only when account probe succeeded (`result.account_id.is_some()`). OAuth always probes. Manual only sets it if `probe_account` returned a login. A credential that was stored but never verified against the API stays `Unchecked`.
- In `compute_status()`: check secrets registry file (NOT vault decryption — no Touch ID) for `secret_ref` existence → produce `ConnectionStatus::Missing` when registry entry absent. This is weaker than checking the vault itself — the registry could be stale — but `resolve_auth` remains the real gate at runtime (fail-closed).
- Order: Errored → Expired → Missing → Unchecked → Connected.

### Commit 4: `fix(broker): source_id uses source name, not child name`

**Files:** `src/broker/routing.rs`, `src/broker/mod.rs`

- Change `routing.rs:94` from `format!("child:{}", child_name)` to `format!("source:{}", source_name)`.
- Thread `source_name` through `validate_fact()`.
- Update `broker/mod.rs` status query — handle both `source:` and `child:` prefixes during transition.
- Add test: two facts from different sources using the same child get distinct `source_id` values.

### Commit 5: `fix(docs): document sandbox enforcement gap honestly`

**Files:** `src/broker/spawn.rs`, `crates/patina-pipe/src/harness.rs`

- `spawn.rs:174`: change log message to include `(NOT YET ENFORCED)`.
- `spawn.rs:184`: expand comment to state children run unrestricted until sandbox enforcement ships.
- `harness.rs:1`: change "Mother-side test harness" to "Mother-side harness for spawning and communicating with native children."
- No behavioral changes.

### Commit 6: `refactor: move resolve_child_binary out of broker`

**Files:** `src/broker/spawn.rs`, `src/connect/internal/resolve.rs`, new shared module

- Extract `resolve_child_binary()` from `src/broker/spawn.rs` to a shared location (e.g., `src/children.rs` or `paths::children`).
- Update `broker/spawn.rs` and `connect/internal/resolve.rs` to import from the new location.
- Dependency now flows: connect → shared ← broker. No cycle.

### Commit 7: `fix(broker): reject lake destination at parse time`

**Files:** `src/broker/sources.rs`, `src/broker/mod.rs`

- In `parse_destination()`: when `dest_type == "lake"`, return actionable error:
  `"source '{}': lake destinations are not yet supported. Use destination.type = \"project\" or remove the [destination] section."`
- Keep `Destination::Lake` variant in the enum (it's the right data model for when lakehouse ships), but reject at parse time until implementation exists.
- Remove `route_to_lake()` stub from `broker/mod.rs` (dead code once parse rejects it).

### Commit 8: `test: verify all hardening fixes + pre-push`

- Ensure each fix has at least one test proving the bug is gone.
- Run `./resources/git/pre-push-checks.sh`.

## Exit Criteria

1. Child process leak fixed — `NativeChild` `Drop` kills child, test proves no orphan after drop
2. Connection creation atomic — vault-first write order, rollback TOML on vault failure
3. `compute_status` produces `Connected` after successful create, `Missing` when vault entry absent
4. `source_id` uses source name not child name — two sources sharing a child get distinct fact counts
5. Sandbox gap documented — code comments and log messages state enforcement is deferred
6. `harness.rs` module doc updated — no longer says "test harness"
7. `resolve_child_binary` moved out of broker — connect no longer imports `crate::broker`
8. Lake destination rejected at parse time with actionable message, not runtime bail
9. All existing tests pass + new tests for each fix
