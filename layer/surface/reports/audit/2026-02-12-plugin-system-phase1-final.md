---
type: audit
id: plugin-system-phase1-final-audit
scope: src/plugin/, patina-plugin-api/, patina-plugin-models/, patina-plugin-repos/, wit/, src/commands/mother/
spec: layer/surface/build/feat/plugin-system/SPEC.md
related: layer/surface/build/feat/mother-repos/SPEC.md
previous_audit: layer/surface/reports/audit/2026-02-12-plugin-system-phase1.md
remediation: layer/surface/build/fix/plugin-system-audit-remediation/SPEC.md
remediation_spec: layer/surface/build/fix/plugin-system-final-audit-fixes/SPEC.md
session: 20260212-093831
created: 2026-02-12
status: complete
findings:
  critical: 0
  important: 2
  minor: 4
  nit: 3
verdict: Phase 1 is complete. All exit criteria met. Two important findings are Phase 2 deferrals.
---

# Plugin System Phase 1 — Final Audit

> Final gate audit of the COMPLETE Phase 1 plugin system, including repos
> child. Covers everything the previous audit (2026-02-12) covered, plus
> all remediation work and the repos child addition.
>
> Previous audit: 18 findings (14 Phase 1, 4 Phase 2+). All 14 Phase 1
> findings remediated. This audit is a clean-slate re-read of all code.

## Files Audited

**Host side (patina binary):**
- `src/plugin/mod.rs` — public interface (11 lines)
- `src/plugin/internal.rs` — PluginEngine, WasmChild adapter, all tests (918 lines)
- `src/commands/mother/daemon.rs` — WASM discovery, heartbeat, toy spawning (731 lines)
- `src/commands/mother/registry.rs` — ChildRegistry, duplicate name check (145 lines)
- `src/mother/child.rs` — MotherChild trait, Toy, ChildRequest/Response (139 lines)
- `src/paths.rs` — plugin module (lines 179-199)
- `src/lib.rs` — `pub mod plugin` (line 13)

**WIT definitions:**
- `wit/mother-child.wit` — patina:mother-child@0.1.0 (42 lines)
- `wit/deps/patina-host/host.wit` — patina:host@0.1.0 (14 lines)

**Guest-side API:**
- `patina-plugin-api/src/lib.rs` — MotherChildPlugin trait, register_plugin! macro (178 lines)
- `patina-plugin-api/Cargo.toml`

**Models child (first WASM child):**
- `patina-plugin-models/src/lib.rs` — ModelsChild implementation (82 lines)
- `patina-plugin-models/plugin.toml` — manifest
- `patina-plugin-models/Cargo.toml`

**Repos child (second WASM child — NEW since previous audit):**
- `patina-plugin-repos/src/lib.rs` — ReposChild implementation (185 lines)
- `patina-plugin-repos/plugin.toml` — manifest
- `patina-plugin-repos/Cargo.toml`

**Test fixtures:**
- `tests/fixtures/patina_plugin_models.wasm` (156KB)
- `tests/fixtures/patina_plugin_repos.wasm` (178KB)

**Beliefs verified:**
- `[[hoststate-cohabits-with-bindgen]]` — HostState in mod bindings ✓
- `[[wasm32-wasip2-always-imports-wasi]]` — wasmtime-wasi present ✓
- `[[mother-is-the-daemon]]` — toys from tick(), Mother runs them ✓
- `[[coupling-is-complexity]]` — models first (lowest coupling) ✓
- `[[de-risk-runtime-with-simplest-payload]]` — models → repos ordering ✓

---

## 1. SPEC COMPLIANCE

### 1.1 Exit Criteria — All Met

**Severity: no issue**

All original Phase 1 exit criteria verified:

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `handle()` round-trip <1ms | **PASS** (0.002ms) | Benchmark test in `internal.rs:831-917` |
| At least one MotherChild from WASM in `cargo test` | **PASS** (16 tests) | `cargo test -p patina-ai -- plugin` — 16 pass |
| `wasmtime::Engine::new()` time measured | **PASS** (1.36ms) | Benchmark test, threshold <100ms |

All repos child exit criteria verified:

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Repos child implements MotherChild as WASM plugin | **PASS** | `patina-plugin-repos/` crate, 178KB WASM |
| `tick()` detects stale repos and requests toys | **PASS** | Test `wasm_repos_child_tick_returns_toys` |
| Toy system proven end-to-end | **PASS** | 4 integration tests |
| At least one repos child test in `cargo test` | **PASS** | 4 tests in `plugin::internal::tests` |

### 1.2 WIT Versioning — Correct

**Severity: no issue**

Both WIT packages have `@0.1.0` annotations (remediation I2):
- `package patina:host@0.1.0;` in `wit/deps/patina-host/host.wit:1`
- `package patina:mother-child@0.1.0;` in `wit/mother-child.wit:1`
- Import uses versioned syntax: `import patina:host/log@0.1.0;`

### 1.3 WIT Consistency Across Crates — Correct

**Severity: no issue**

WIT files verified identical across all four locations:
- `wit/` (host bindgen source)
- `patina-plugin-api/wit/` (guest bindgen source)
- `patina-plugin-models/wit/` (transitive via api dep)
- `patina-plugin-repos/wit/` (transitive via api dep)

These are separate directory copies, not symlinks. While copies create a
synchronization risk, the files are currently byte-identical.

### 1.4 Spec Text Amendments — Correct

**Severity: no issue**

SPEC.md status log contains the amendment entry (session [[20260212-083400]])
documenting all 5 spec text inaccuracies from the previous audit. The spec
body is preserved as historical record with corrections in the status log.
This follows [[spec-driven-design]] correctly.

---

## 2. CORRECTNESS

### 2.1 Repos Child handle() Round-Trip — Correct

**Severity: no issue**

`report_repo` correctly:
- Parses JSON payload (name, path, last_indexed)
- Upserts into `self.repos` Vec (updates existing by name, or appends)
- Returns `{"status": "registered", "name": ..., "total_repos": ...}`

`check_freshness` correctly:
- Gets current time via `current_time_secs()` (wasm32-wasip2 clocks)
- Computes age as `now.saturating_sub(last_indexed)` — overflow-safe
- Returns per-repo freshness with `stale: bool` flag

### 2.2 Repos Child tick() Toy Generation — Correct

**Severity: no issue**

`tick()` correctly generates toys for stale repos:
- Pull toy: `git -C {path} pull` — correct git invocation
- Scrape toy: `patina scrape --repo {name}` — correct CLI invocation
- Fresh repos (age ≤ threshold) produce no toys — tested

### 2.3 Toy Spawning in daemon.rs — Correct

**Severity: no issue**
**Location:** `src/commands/mother/daemon.rs:121-164`

`spawn_heartbeat()` calls `registry.tick_all()` then `spawn_toy()` for each.
`spawn_toy()` runs each toy in its own thread with:
- stdout suppressed (null)
- stderr piped (captured)
- Exit status logged (success, failure, spawn error)

The repos child's toys (`git -C {path} pull`, `patina scrape --repo {name}`)
would execute correctly through this mechanism.

### 2.4 WasmChild Adapter — Correct

**Severity: no issue**

All five MotherChild trait methods correctly bridge to WASM:
- `name()` → cached string (no WASM call)
- `on_load()` → `call_on_load()` with Result mapping
- `health()` → `call_health()` with enum mapping
- `handle()` → JSON serialize → `call_handle()` → JSON deserialize
- `tick()` → `call_tick()` with Toy mapping

---

## 3. ARCHITECTURE

### 3.1 Dependable-Rust Pattern — Correct

**Severity: no issue**
**Location:** `src/plugin/mod.rs`, `src/plugin/internal.rs`

`mod.rs` is 11 lines: docs + `mod internal; pub use internal::{...}`.
All implementation in `internal.rs`. No `internal::` in public signatures.
Textbook dependable-rust.

### 3.2 Repos Child Follows Models Child Pattern — Correct

**Severity: no issue**

Both children follow identical structure:

| Aspect | Models | Repos |
|--------|--------|-------|
| Cargo.toml | `cdylib`, deps on `patina-plugin-api` + `serde_json` | Same |
| plugin.toml | `[plugin]` section, `world = "mother-child"`, `host_log = true` | Same |
| lib.rs | `#[derive(Default)]` struct, impl MotherChildPlugin, `register_plugin!` | Same |
| Actions | `resolve_model`, `model_status` | `report_repo`, `check_freshness` |
| tick() | default (empty) | Custom (returns toys) |
| health() | default (Healthy) | Custom (checks staleness) |

Repos child adds `tick()` and `health()` overrides — this is the expected
differentiation for a child that monitors state.

### 3.3 WIT Types Inside World Block — Still Inside (Known Phase 2 Deferral)

**Severity: minor (Phase 2 deferral — unchanged from previous audit)**
**Location:** `wit/mother-child.wit:9-20`

`child-health` enum and `toy` record are defined inside `world mother-child {}`.
This scopes them to this world only. The `command` world (Phase 2) would
need its own copies or these types need to move outside.

Already documented as a Phase 2 discovery in [[plugin-system]] spec.
No action needed now.

### 3.4 paths.rs Plugin Module — Correct

**Severity: no issue**
**Location:** `src/paths.rs:179-199`

Pure path construction. No `exists()`, no `read_dir()`, no I/O.
Three functions: `children_dir()`, `plugins_dir()`, `work_dir(name)`.
Correct per paths.rs invariant.

---

## 4. SAFETY

### 4.1 unsafe impl Sync for WasmChild — Sound

**Severity: no issue**
**Location:** `src/plugin/internal.rs:292-296`

Safety comment correctly states the argument (remediation I4):
- `bindings::MotherChild` is `Send + !Sync`
- `call_*()` methods take `&self` (immutable) + `&mut Store` (mutable)
- Mutex on store serializes all WASM calls
- Instance is effectively immutable between calls

The argument is sound. The Mutex ensures no concurrent WASM calls.

### 4.2 Mutex Poison Recovery in WasmChild — Correct

**Severity: no issue**
**Location:** `src/plugin/internal.rs:306,314,319,331,345`

All five sites use `.lock().unwrap_or_else(|e| e.into_inner())` (remediation
I3). Matches the pattern in `SecretsCacheChild`.

### 4.3 RwLock Poison Handling in ChildRegistry — Inconsistent

**Severity: important**
**Location:** `src/commands/mother/registry.rs:29,41,55,67,78,81`

ChildRegistry uses `.read().unwrap()` and `.write().unwrap()` in 6 places:

| Line | Method | Lock Type | Uses |
|------|--------|-----------|------|
| 29 | `register()` | `read().unwrap()` | duplicate name check |
| 41 | `load_all()` | `write().unwrap()` | on_load call |
| 55 | `tick_all()` | `write()` via `if let Ok(...)` | tick call |
| 67 | `health_all()` | `read()` via `.ok()?` | health check |
| 78 | `handle()` | `read().unwrap()` | find child |
| 81 | `handle()` | `read().unwrap()` | call handle |

Lines 55 and 67 already handle poison gracefully (skip on failure). But
lines 29, 41, 78, and 81 will panic on a poisoned RwLock.

If `on_load()` panics (line 41) while holding the write lock, the RwLock
for that child is poisoned. All subsequent `handle()` calls to any child
in the registry would iterate the Vec and hit `read().unwrap()` on the
poisoned entry — panicking and potentially crashing the daemon.

**Recommended fix:** Replace `read().unwrap()` and `write().unwrap()` with
`unwrap_or_else(|e| e.into_inner())` consistently, matching the pattern
already used in WasmChild and SecretsCacheChild. Or use the
`if let Ok(...)` pattern already used in `tick_all()` and `health_all()`.

**Blocks Phase 1 closure:** No — this is a robustness improvement, not a
correctness bug. The registry is immutable after setup (children registered
before daemon starts accepting connections), so poison from `on_load()`
is the only realistic trigger.

### 4.4 static mut PLUGIN in Guest API — Correct for Now (Known Edition Deferral)

**Severity: no issue (already documented)**
**Location:** `patina-plugin-api/src/lib.rs:88`

`static mut` with `#[allow(static_mut_refs)]`. Correct for single-threaded
WASM. Edition 2024 migration noted in [[plugin-system]] spec discoveries.

### 4.5 tick() Error Logging — Correct

**Severity: no issue**
**Location:** `src/plugin/internal.rs:355-358`

`tick()` errors are now logged (remediation M4):
```rust
Err(e) => {
    eprintln!("[plugin:{}] tick failed: {}", self.name, e);
    vec![]
}
```

### 4.6 Toy Spawning — No Rate Limiting

**Severity: minor (future concern)**
**Location:** `src/commands/mother/daemon.rs:121-164`

Every heartbeat (60s) calls `tick_all()`. If a repos child reports N stale
repos, each heartbeat spawns 2N background threads (pull + scrape per repo).
There is no guard against:

1. **Duplicate toys:** If a pull/scrape takes >60s, the next heartbeat will
   spawn the same toys again (the child still considers the repo stale because
   `last_indexed` hasn't been updated by the host).

2. **Thread explosion:** 25 stale repos × 2 toys = 50 threads per heartbeat.
   Unlikely but possible if Mother starts with many stale repos.

**Phase 2 mitigation:** Track in-flight toys. The child could set a
"pending" flag on reported repos, or the host could skip duplicate toy names.
Not a Phase 1 blocker — the daemon is not yet feeding real state to the
repos child.

---

## 5. COMPLETENESS

### 5.1 Test Coverage — Comprehensive

**Severity: no issue**

16 plugin tests covering:

| Category | Count | What's tested |
|----------|-------|---------------|
| Manifest parsing | 6 | Valid minimal, valid full, missing section, missing name, missing world, invalid TOML |
| Capability checking | 3 | All granted, empty caps, denied caps |
| WASM models child | 2 | handle() round-trip, health() |
| WASM repos child | 4 | handle() round-trip, tick() returns toys, fresh repo no toys, health reflects staleness |
| Benchmarks | 1 | Engine init, component compile, instantiate, handle avg |

Additionally, 2 registry tests in `registry.rs`:
- `register_unique_names` — happy path
- `register_duplicate_name_rejected` — duplicate detection

**Missing coverage (minor):**

| Gap | Risk | Priority |
|-----|------|----------|
| `on_load()` / `on_unload()` for WASM children | Low — trivial delegation | nit |
| `model_status` action for models child | Low — same code path as resolve_model | nit |
| Repos child with multiple repos (>1 stale, >1 fresh) | Low — linear iteration | nit |
| Error paths: malformed payload to handle() | Medium — WASM returns Err(String) | minor |

None of these gaps block Phase 1 closure.

### 5.2 WASM Fixtures — Up to Date

**Severity: no issue**

Both fixtures are more recent than their source files:
- `patina_plugin_models.wasm` (Feb 12 08:42) > `patina-plugin-models/src/lib.rs` (Feb 12 06:59)
- `patina_plugin_repos.wasm` (Feb 12 09:27) > `patina-plugin-repos/src/lib.rs` (Feb 12 09:27)

### 5.3 WIT Files Are Copies, Not Symlinks

**Severity: minor (maintenance concern)**
**Location:** `patina-plugin-api/wit/`, `patina-plugin-models/wit/`, `patina-plugin-repos/wit/`

The `wit/` directories in guest crates are full directory copies, not
symlinks to the canonical `wit/` at project root. Currently byte-identical,
but a future WIT change requires updating 4 copies.

The previous audit noted these as symlinks. They are now full copies
(verified with `file` command). This may have changed during the repos
child build or the audit remediation.

**Recommended fix:** Consider restoring symlinks, or add a CI check that
all WIT copies match the canonical `wit/` directory. Not a Phase 1 blocker.

### 5.4 Orphaned Code Check — Clean

**Severity: no issue**

No orphaned code from the build process. No unused imports, no dead
functions, no commented-out code in any audited file.

### 5.5 Specs Accurately Reflect What Was Built

**Severity: no issue**

- [[plugin-system]] Phase 1 exit criteria all checked off with evidence
- [[mother-repos]] Phase 1 acceptance criteria all checked off with evidence
- Status logs document the full build history
- Discoveries section in [[plugin-system]] correctly captures 4 Phase 2+ items

---

## 6. PHASE 2 READINESS

### 6.1 Previous Audit Phase 2 Discoveries — Still Valid

All four Phase 2+ discoveries from the previous audit remain valid and are
correctly documented in [[plugin-system]] spec:

| Discovery | Still valid? | Notes |
|-----------|-------------|-------|
| WIT types inside world block | **Yes** | Unchanged, still inside world |
| Re-entrancy invariant | **Yes** | No new host functions added |
| ChildHealth reason string | **Yes** | Repos child uses `Degraded` without reason (WIT limitation) |
| static mut edition migration | **Yes** | Still edition 2021, no change |

### 6.2 Repos Child Introduces New Phase 2 Concern: Host State Feed

**Severity: important (Phase 2 concern)**

The repos child's host-fed state pattern works for Phase 1 (tests use
hardcoded data), but in production Mother needs to actually feed state:

1. Mother must read `~/.patina/registry.yaml` to discover repos
2. Mother must call `handle("report_repo", ...)` for each repo
3. Mother must know when a repo has been successfully indexed to update
   `last_indexed`

None of this plumbing exists in `daemon.rs` yet. The repos child will
produce zero toys in production until the host-side feeding is implemented.
This is expected for Phase 1 (the design explicitly defers it), but
Phase 2 must include the host-side integration.

**Not a Phase 1 blocker** — Phase 1 scope is explicitly "proves the toy
system end-to-end" via tests, not production integration.

### 6.3 Duplicate Toy Prevention

**Severity: minor (Phase 2 concern)**

See finding 4.6 above. When repos child is fed real state, the heartbeat
loop will need deduplication to avoid spawning the same pull/scrape toys
every 60 seconds for slow-indexing repos.

---

## Summary Table

| Severity | Count | Key Items |
|----------|-------|-----------|
| **Critical** | 0 | — |
| **Important** | 2 | Registry RwLock poison handling (4.3), host state feed needed for Phase 2 (6.2) |
| **Minor** | 4 | WIT types in world block (3.3, Phase 2 deferral), toy rate limiting (4.6), WIT copies not symlinks (5.3), duplicate toy prevention (6.3) |
| **Nit** | 3 | Missing test coverage for on_load, model_status, error paths (5.1) |

## Verdict

**Phase 1 is complete.** All exit criteria are met with evidence. The code
is architecturally sound, follows dependable-rust, and the repos child
correctly extends the pattern established by models child.

The two important findings are:
1. **Registry poison handling** — a robustness improvement, not a correctness
   bug. The registry is immutable after setup, so the realistic risk is low.
2. **Host state feed** — expected Phase 2 work, explicitly deferred.

**No findings block Phase 1 closure.**

### Comparison to Previous Audit

| Metric | Previous Audit | This Audit |
|--------|---------------|------------|
| Critical | 3 (missing tests, benchmarks, exit criteria) | 0 |
| Important | 5 | 2 (1 new: registry poison, 1 Phase 2 awareness) |
| Minor | 5 | 4 (1 new: toy rate limiting, 1 new: WIT copies) |
| Nit | 5 | 3 |
| Total | 18 | 9 |
| Blocks Phase 1 | 3 critical + portions of 5 important | 0 |

All 14 Phase 1 findings from the previous audit have been resolved.
The 4 Phase 2+ discoveries remain correctly documented and deferred.

### Previous Audit Remediation Verification

| Finding | Remediation Status | Verified |
|---------|-------------------|----------|
| C1: Zero tests | 16 tests written | ✓ Verified — all pass |
| C2: No benchmarks | Benchmark test added | ✓ Verified — thresholds met |
| C3: Exit criteria unmet | All checked off | ✓ Verified — evidence present |
| I1: Spec amendments | Status log entry added | ✓ Verified — 5 inaccuracies documented |
| I2: WIT version annotations | `@0.1.0` added | ✓ Verified — both files |
| I3: Mutex poison handling | `unwrap_or_else` in 5 sites | ✓ Verified — all WasmChild methods |
| I4: unsafe Sync comment | Rewritten | ✓ Verified — precise argument |
| I5: Duplicate child name | Check in register() | ✓ Verified — with test |
| M4: tick() error logging | eprintln added | ✓ Verified |
| N5: Orphaned .toml diagnostic | Diagnostic loop added | ✓ Verified |

---

## Appendix: Test Run Output

```
$ cargo test -p patina-ai -- plugin
running 16 tests
test plugin::internal::tests::manifest_valid_minimal ... ok
test plugin::internal::tests::manifest_valid_full ... ok
test plugin::internal::tests::manifest_missing_plugin_section ... ok
test plugin::internal::tests::manifest_missing_name ... ok
test plugin::internal::tests::manifest_missing_world ... ok
test plugin::internal::tests::manifest_invalid_toml ... ok
test plugin::internal::tests::capabilities_all_granted ... ok
test plugin::internal::tests::capabilities_empty ... ok
test plugin::internal::tests::capabilities_denied ... ok
test plugin::internal::tests::wasm_models_child_handle_roundtrip ... ok
test plugin::internal::tests::wasm_models_child_health ... ok
test plugin::internal::tests::wasm_repos_child_handle_roundtrip ... ok
test plugin::internal::tests::wasm_repos_child_tick_returns_toys ... ok
test plugin::internal::tests::wasm_repos_child_fresh_repo_no_toys ... ok
test plugin::internal::tests::wasm_repos_child_health_reflects_staleness ... ok
test plugin::internal::tests::benchmark_plugin_performance ... ok
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 143 filtered out
```
