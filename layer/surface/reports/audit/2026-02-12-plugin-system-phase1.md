---
type: audit
id: plugin-system-phase1-audit
scope: src/plugin/, patina-plugin-api/, patina-plugin-models/, wit/
spec: layer/surface/build/feat/plugin-system/SPEC.md
session: 20260212-075642
created: 2026-02-12
status: complete
findings:
  critical: 3
  important: 5
  minor: 5
  nit: 5
verdict: architecturally sound, missing tests/benchmarks/spec amendments
---

# Plugin System Phase 1 — Full WASM Audit

> Audit of the complete WASM plugin system built in sessions 20260211-203416
> and 20260212-061458 (steps 1-12 of SPEC.md Phase 1).

## Files Audited

**Host side (patina binary):**
- `Cargo.toml` — wasmtime =41, wasmtime-wasi =41
- `rust-toolchain.toml` — 1.90
- `wit/deps/patina-host/host.wit` — patina:host package, log interface
- `wit/mother-child.wit` — patina:mother-child world
- `src/plugin/mod.rs` — public interface
- `src/plugin/internal.rs` — wasmtime guts, bindgen, WasmChild adapter
- `src/paths.rs` — plugin module
- `src/lib.rs` — pub mod plugin
- `src/commands/mother/daemon.rs` — WASM child discovery

**Guest side (plugin crates):**
- `patina-plugin-api/Cargo.toml` — wit-bindgen 0.41
- `patina-plugin-api/src/lib.rs` — guest API, MotherChildPlugin trait, register_plugin! macro
- `patina-plugin-models/Cargo.toml` — cdylib
- `patina-plugin-models/src/lib.rs` — ModelsChild implementation
- `patina-plugin-models/plugin.toml` — manifest

**Integration points:**
- `src/mother/child.rs` — MotherChild trait
- `src/commands/mother/registry.rs` — ChildRegistry
- `src/commands/mother/secrets.rs` — SecretsCacheChild (reference)

**Beliefs reviewed:**
- `hoststate-cohabits-with-bindgen`
- `wasm32-wasip2-always-imports-wasi`
- `wit-bindgen-generate-at-crate-root`
- `explicit-init-over-lazy-init-wasm`

---

## 1. SPEC COMPLIANCE

### 1.1 WIT Version Annotations Missing

**Severity: important**
**Location:** `wit/deps/patina-host/host.wit:1`, `wit/mother-child.wit:1`

The spec explicitly defines versioned packages:
- `package patina:host@0.1.0;`
- `package patina:mother-child@0.1.0;`

The implementation omits version annotations:
- `package patina:host;`
- `package patina:mother-child;`

**Suggested fix:** Add `@0.1.0` to both package declarations.

---

### 1.2 host.wit Location Differs From Spec

**Severity: minor**
**Location:** `wit/deps/patina-host/host.wit`

Spec says `wit/host.wit` at the top level. Implementation places it at
`wit/deps/patina-host/host.wit`. This follows the WIT dependency resolution
convention (imported packages live in `deps/`), which is actually the correct
structure for `wasmtime::component::bindgen!` and `wit_bindgen::generate!`.
The spec's layout would not resolve correctly.

**Suggested fix:** Amend spec to document the `deps/` convention.

---

### 1.3 WIT Types Inside World Block vs Outside

**Severity: minor**
**Location:** `wit/mother-child.wit:8-20`

Spec defines `child-health` and `toy` OUTSIDE the `world mother-child {}` block.
Implementation defines them INSIDE. Both are valid WIT -- types inside a world
are scoped to that world (cannot be imported by other worlds). Since no other
world currently needs these types, this works. But if Phase 2's `command` world
ever needs `child-health`, it would need its own copy.

**Suggested fix:** Consider moving types outside the world block to match spec
and enable reuse.

---

### 1.4 wasmtime-wasi Present in Phase 1 (Spec Says Phase 2+)

**Severity: important (spec amendment needed)**
**Location:** `Cargo.toml:100`

The spec explicitly says Phase 1 needs no wasmtime-wasi:

> **Cargo.toml features (Phase 1 -- wasmtime only, no wasmtime-wasi)**

The implementation correctly includes `wasmtime-wasi` because wasm32-wasip2
components always import basic WASI interfaces (belief
`wasm32-wasip2-always-imports-wasi`). The spec's status log acknowledges this
was discovered, but the spec text was never amended.

**Suggested fix:** Amend spec Phase 1 Cargo.toml section to include wasmtime-wasi.
Add note: "Originally deferred to Phase 2, but wasm32-wasip2 requires it."

---

### 1.5 Exit Criteria NOT Met

**Severity: critical**
**Location:** SPEC.md acceptance/exit criteria

Three exit criteria remain unmeasured or unmet:

| Criterion | Status |
|-----------|--------|
| PluginEngine init <100ms | **NOT MEASURED** |
| handle() round-trip <1ms | **NOT MEASURED** |
| At least one MotherChild loaded from WASM in `cargo test` | **NOT MET -- zero tests** |

**Suggested fix:** Add benchmarks and at least one integration test before
closing Phase 1.

---

### 1.6 Spec Claims patina-plugin-api is cdylib

**Severity: nit**
**Location:** SPEC.md "Files Created" section

Spec says: `patina-plugin-api/Cargo.toml -- wit-bindgen, crate-type = ["cdylib"]`.
The implementation correctly does NOT make the API crate cdylib -- only
`patina-plugin-models` is cdylib. The API crate is a library dependency consumed
by plugin crates.

**Suggested fix:** Amend spec: move `crate-type = ["cdylib"]` note to
patina-plugin-models entry.

---

### 1.7 registry.rs NOT Modified (Spec Says It Should Be)

**Severity: nit**
**Location:** SPEC.md "Files Modified" section

Spec lists `src/commands/mother/registry.rs` as modified for "WasmChild adapter".
In reality, WasmChild lives entirely in `src/plugin/internal.rs` and implements
the `MotherChild` trait. The registry needed no modification -- it already
accepts `Box<dyn MotherChild>`.

**Suggested fix:** Remove registry.rs from spec's "Files Modified" list.

---

### 1.8 Missing [package.metadata.component] in API Crate

**Severity: nit**
**Location:** `patina-plugin-api/Cargo.toml`

Spec shows `[package.metadata.component] target = { path = "../wit" }`.
Not present in implementation. Not needed since `wit_bindgen::generate!` uses
an explicit `path: "wit"` parameter. The spec metadata is for `cargo-component`
tooling which isn't used here.

**Suggested fix:** Remove from spec or add as a documentation-only entry.

---

## 2. SAFETY

### 2.1 `unsafe impl Sync for WasmChild` -- Sound but Comment Imprecise

**Severity: important**
**Location:** `src/plugin/internal.rs:294`

```rust
// Safety: Store<HostState> is Send (HostState is Send).
// Mutex provides Sync. The instance is only accessed through the Mutex-guarded store.
unsafe impl Sync for WasmChild {}
```

The safety argument is **sound** but the comment is **imprecise**. The `instance`
field is NOT behind the Mutex -- it's a sibling field accessed directly. The
actual safety argument:

1. `bindings::MotherChild` is `Send + !Sync` (wasmtime generated type)
2. All `call_*()` methods on the instance take `&self` (immutable) plus
   `&mut Store` (mutable)
3. The Mutex on `store` serializes all WASM calls -- no concurrent calls possible
4. The instance itself is immutable between calls

**Suggested fix:** Rewrite safety comment:
```rust
// Safety: bindings::MotherChild is Send + !Sync. However, its call_*()
// methods take &self (immutable) and require &mut Store (mutable). The
// Mutex on store serializes all WASM calls, preventing concurrent access.
```

---

### 2.2 Mutex Lock unwrap() vs Poison Recovery

**Severity: important**
**Location:** `src/plugin/internal.rs:304,312,317,329,343`

All five `WasmChild` trait methods use `.lock().unwrap()`:
```rust
let mut store = self.store.lock().unwrap();
```

If a panic occurs between lock acquisition and drop (e.g., allocation failure
triggered by malicious WASM), the Mutex becomes poisoned. All subsequent calls
to that child would panic, potentially crashing the daemon.

Contrast with `SecretsCacheChild` (`secrets.rs:47,67,90,102`) which uses
`.unwrap_or_else(|e| e.into_inner())` for poison recovery.

**Suggested fix:** Replace `.lock().unwrap()` with
`.lock().unwrap_or_else(|e| e.into_inner())` consistently, or add a comment
explaining why poison recovery isn't needed here.

---

### 2.3 `static mut PLUGIN` -- Correct for WASM, Future Edition Warning

**Severity: minor**
**Location:** `patina-plugin-api/src/lib.rs:88`

```rust
static mut PLUGIN: Option<Box<dyn MotherChildPlugin>> = None;
```

Correct for WASM (single-threaded, no concurrent access). The
`#[allow(static_mut_refs)]` on line 99 suppresses the lint. However,
`static mut` is deprecated in Rust 2024 edition. When the project upgrades
from edition 2021, this will need migration to `UnsafeCell` or similar.

**Suggested fix:** No action needed now. Note that edition 2024 migration will
require refactoring guest-side singleton.

---

### 2.4 `#[export_name = "init"]` ABI Correctness

**Severity: nit (no issue)**
**Location:** `patina-plugin-api/src/lib.rs:172-175`

```rust
#[export_name = "init"]
extern "C" fn __patina_plugin_init() { ... }
```

Correct. The WIT world defines `export init: func();`, `wit_bindgen::generate!`
skips it (via `skip: ["init"]`), and the macro generates it manually with the
right export name. The `extern "C"` ABI is correct for WASM exports.

---

### 2.5 Potential Re-entrancy Risk in Host Functions

**Severity: minor (future risk)**
**Location:** `src/plugin/internal.rs:46-55`

Currently the only host function (`patina::host::log::Host::log()`) calls
`eprintln!()` -- no re-entrancy risk. But if future host functions (Phase 2+)
try to access the store or call other WASM methods, the Mutex would deadlock
because the calling WASM method already holds the lock.

**Suggested fix:** Document the invariant: "Host function implementations MUST
NOT acquire the store Mutex or call WASM methods on the same instance."

---

## 3. ARCHITECTURE

### 3.1 Dependable-Rust Pattern -- Correct

**Severity: nit (no issue)**
**Location:** `src/plugin/mod.rs`, `src/plugin/internal.rs`

`mod.rs` is 11 lines: docs + `mod internal; pub use internal::{...}`. All
implementation in `internal.rs`. Textbook dependable-rust.

---

### 3.2 paths.rs -- No I/O, Correct

**Severity: nit (no issue)**
**Location:** `src/paths.rs:178-199`

Plugin module (`children_dir()`, `plugins_dir()`, `work_dir()`) is pure path
construction. No `exists()`, no `read_dir()`, no I/O. Correct.

---

### 3.3 Information Loss in ChildHealth Mapping

**Severity: important**
**Location:** `src/plugin/internal.rs:319-323`

```rust
bindings::ChildHealth::Degraded => ChildHealth::Degraded("degraded".into()),
bindings::ChildHealth::Unhealthy => ChildHealth::Unhealthy("unhealthy".into()),
```

The WIT `child-health` enum is `{ healthy, degraded, unhealthy }` -- no reason
string. The Rust `ChildHealth` is `Degraded(String)` and `Unhealthy(String)`.
The adapter hardcodes the variant name as the reason, losing any diagnostic
information.

The WASM plugin has no way to communicate **why** it's degraded or unhealthy.

**Suggested fix:** For Phase 2, consider changing WIT to a record:
`record child-health { status: health-status, reason: option<string> }`.
For now, document this as a known limitation.

---

### 3.4 Double JSON Serialization in handle()

**Severity: minor (by design)**
**Location:** `src/plugin/internal.rs:329-339`

The flow is: `ChildRequest.payload: Value` -> `serde_json::to_string()` ->
WASM string -> WASM string -> `serde_json::from_str()` ->
`ChildResponse.payload: Value`. This double-serializes JSON across the WASM
boundary.

This is correct by design -- the WIT interface uses `string` for payload (not
a structured type), and JSON is the wire format. The overhead is proportional
to payload size. For small payloads (model names), negligible.

---

## 4. ERROR HANDLING

### 4.1 WASM Trap After Poison -- Child Becomes Permanently Unusable

**Severity: important**
**Location:** `src/plugin/internal.rs:304,329`

If a panic occurs while the Mutex is held (see 2.2), all subsequent calls to
that child panic. The daemon stays alive (each request is a thread), but the
child is permanently unusable until daemon restart.

Even without poison, a WASM trap may leave the plugin's linear memory in an
inconsistent state. There's no mechanism to detect a "broken" child and
disable/restart it.

**Suggested fix:** Consider marking a child as unhealthy after a trap, and/or
providing a mechanism to re-instantiate a WASM child without daemon restart.

---

### 4.2 tick() Silently Swallows Errors

**Severity: minor**
**Location:** `src/plugin/internal.rs:343-355`

```rust
fn tick(&mut self) -> Vec<Toy> {
    let mut store = self.store.lock().unwrap();
    match self.instance.call_tick(&mut *store) {
        Ok(wasm_toys) => ...,
        Err(_) => vec![],  // silently swallowed
    }
}
```

A WASM trap during tick is silently swallowed. The heartbeat loop never knows
the child failed. No logging, no health degradation.

**Suggested fix:** Log the error:
`Err(e) => { eprintln!("[plugin:{}] tick failed: {}", self.name, e); vec![] }`

---

### 4.3 Error Handling in daemon.rs -- Good Overall

**Severity: nit (no issue)**

- Malformed plugin.toml -> clear error, daemon continues
- Missing .wasm -> not scanned
- .wasm without .toml -> manifest load fails, error printed, continues
- Capability denied -> clear error listing denied caps, continues
- init fails -> error propagates, child not loaded, continues
- PluginEngine::new() fails -> all WASM disabled, compiled-in children still work

All good.

---

## 5. EDGE CASES

### 5.1 Duplicate Child Names -- Silent Override

**Severity: important**
**Location:** `src/commands/mother/registry.rs:64-69`

```rust
.find(|c| c.read().unwrap().name() == child_name)
```

`ChildRegistry` finds the first child matching a name. If two WASM children
(or a compiled-in + WASM child) share a name, the second is silently
unreachable. No warning at registration time.

**Suggested fix:** Add a duplicate-name check in `register()`:
```rust
if self.children.iter().any(|c| c.read().unwrap().name() == child.name()) {
    return Err(anyhow!("duplicate child name: {}", child.name()));
}
```

---

### 5.2 Orphaned .toml Files Not Detected

**Severity: nit**
**Location:** `src/commands/mother/daemon.rs:532`

The scan only finds `.wasm` files. A `.toml` without a matching `.wasm` is
silently ignored. This isn't a bug, but a diagnostic hint ("found manifest
without matching .wasm") would help troubleshooting.

---

### 5.3 Non-UTF8 File Names in children_dir

**Severity: nit**
**Location:** `src/commands/mother/daemon.rs:532`

`path.extension().and_then(|e| e.to_str()) == Some("wasm")` -- correctly
handles non-UTF8 OsStr by returning None (file is skipped). Fine.

---

## 6. PERFORMANCE

### 6.1 No Benchmarks Exist

**Severity: critical (blocks exit criteria)**
**Location:** N/A

The spec has three performance-related exit criteria, none measured:

1. **PluginEngine::new() <100ms** -- Not measured. Involves
   `wasmtime::Engine::new()` (cranelift config init). Typical wasmtime engine
   creation is <10ms, but should be measured.

2. **handle() round-trip <1ms** -- Not measured. Involves Mutex lock + JSON
   serialize + WASM call + JSON deserialize + Mutex unlock. For the models
   child's tiny payloads, should easily be <1ms, but needs measurement.

3. **Component compilation time** -- Not measured. `Component::new(engine,
   wasm_bytes)` compiles the 156KB WASM. Cranelift JIT for small components
   is typically <50ms.

**Suggested fix:** Add timing instrumentation to `PluginEngine::new()`,
`load_component()`, and `instantiate_child()`. Either `cargo bench` or
`eprintln!` with `Instant::now()`.

---

### 6.2 Component Compiled Per-Load, Not Cached

**Severity: minor (future concern)**
**Location:** `src/plugin/internal.rs:212-214`

```rust
pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
    Component::new(wasm_engine(), wasm)
}
```

Every daemon restart recompiles WASM from scratch. wasmtime supports
pre-compiled components via `Component::serialize()` /
`unsafe Component::deserialize()` which can reduce startup from ~50ms to ~1ms
per component.

Not needed for Phase 1 (one small component), but becomes relevant as more
children are added.

**Suggested fix:** Note as Phase 2 optimization opportunity.

---

## 7. TESTS

### 7.1 Zero Tests for Plugin System

**Severity: critical**
**Location:** `src/plugin/` (no tests), `patina-plugin-api/` (no tests),
`patina-plugin-models/` (no tests)

No `#[test]`, no `#[cfg(test)]`, no integration tests anywhere in the plugin
system. All verification was manual (build + install + `patina mother start`
+ `curl`).

The spec exit criterion explicitly requires:
> "At least one MotherChild loaded from WASM in CI test (`cargo test`)"

**Missing tests:**

| Test | Priority |
|------|----------|
| `PluginManifest::from_path()` -- valid manifest | high |
| `PluginManifest::from_path()` -- missing [plugin] section | high |
| `PluginManifest::from_path()` -- missing required fields | high |
| `check_capabilities()` -- all granted | high |
| `check_capabilities()` -- some denied | high |
| Integration: load models.wasm, call handle(), verify response | critical |
| Integration: plugin crash isolation (WASM trap doesn't crash host) | high |
| Duplicate child name detection | medium |

**Suggested fix:** Write tests before closing Phase 1. At minimum: manifest
parsing unit tests + one integration test loading the WASM binary.

---

## Summary Table

| Severity | Count | Key Items |
|----------|-------|-----------|
| **Critical** | 3 | Exit criteria not met, no benchmarks, zero tests |
| **Important** | 5 | Spec needs amendment (wasmtime-wasi), Mutex poison risk, unsafe Sync comment, duplicate names, ChildHealth info loss |
| **Minor** | 5 | WIT types in world block, static mut future migration, re-entrancy docs, tick error swallowed, component caching |
| **Nit** | 5 | host.wit location, spec cdylib claim, registry.rs not modified, metadata.component, orphaned .toml |

## Verdict

**The implementation is architecturally sound.** The code follows
dependable-rust, paths.rs is clean, the WasmChild adapter properly bridges
WASM<->native, and error handling gracefully degrades. The three critical
findings are all about **what's missing** (tests, benchmarks, spec amendments),
not about what's wrong with the code.

### Recommended Next Session Priority

1. Write tests (critical -- blocks Phase 1 exit)
2. Add benchmarks (critical -- blocks Phase 1 exit)
3. Amend SPEC.md (important -- 4 inaccuracies to correct)
4. Fix Mutex poison handling (important -- robustness)
5. Fix unsafe Sync comment (important -- correctness of documentation)
6. Add WIT version annotations (important -- spec compliance)
