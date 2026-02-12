---
type: fix
id: plugin-system-audit-remediation
status: ready
created: 2026-02-12
sessions:
  origin: 20260212-075642
blocked_by: []
blocks: []
related:
  - layer/surface/build/feat/plugin-system/SPEC.md
  - layer/surface/reports/audit/2026-02-12-plugin-system-phase1.md
beliefs:
  - hoststate-cohabits-with-bindgen
  - wasm32-wasip2-always-imports-wasi
  - compiler-enforced-safety
  - spec-driven-design
  - dependable-rust
---

# fix: Plugin System — Audit Remediation

> Address Phase 1 findings from the 2026-02-12 full WASM audit.
> 14 findings are Phase 1 scope. 4 findings pushed outbound to the
> original plugin-system spec for later phases.
> Goal: 100% coverage. If a finding is wrong or inapplicable during
> build, document why and mark it resolved.

## Problem

The Plugin System Phase 1 build (steps 1-12) is functionally complete and
architecturally sound, but a full audit surfaced 18 findings. Of those,
14 are Phase 1 scope (this spec) and 4 are Phase 2+ scope (pushed to
[[plugin-system]] spec as discoveries).

The three critical findings concern **what's missing** — not what's broken:

1. Zero tests (spec exit criterion requires `cargo test` with WASM)
2. Zero benchmarks (spec requires <100ms engine init, <1ms handle)
3. Spec text has 4 inaccuracies from build discoveries

## Source

Full audit report:
`layer/surface/reports/audit/2026-02-12-plugin-system-phase1.md`

Original spec:
`layer/surface/build/feat/plugin-system/SPEC.md`

## Pushed Outbound

These 4 findings are Phase 2+ scope, pushed to [[plugin-system]] spec
discoveries section per [[specs-push-discoveries-outbound]]:

| ID | Finding | Why not Phase 1 |
|----|---------|-----------------|
| M1 | WIT types inside world block | Only matters when Phase 2 `command` world needs shared types |
| M2 | static mut edition migration | Future toolchain concern, not any phase |
| M3 | Re-entrancy invariant for host functions | Risk only emerges when Phase 2+ adds host functions beyond log |
| M5 | ChildHealth info loss (no reason string) | Audit itself says "Phase 2 WIT revision" |

---

## Findings and Fixes

### Critical (3)

#### C1: Zero tests — write plugin system tests

**Audit ref:** 7.1

Tests to write:

| Test | Location | What it proves |
|------|----------|----------------|
| `PluginManifest::from_path()` — valid manifest | `src/plugin/internal.rs` | Happy path parsing |
| `PluginManifest::from_path()` — missing [plugin] | `src/plugin/internal.rs` | Error on bad TOML |
| `PluginManifest::from_path()` — missing required fields | `src/plugin/internal.rs` | Error on incomplete manifest |
| `check_capabilities()` — all granted | `src/plugin/internal.rs` | host_log passes |
| `check_capabilities()` — some denied | `src/plugin/internal.rs` | Unknown caps rejected |
| Integration: load models.wasm, call handle() | `tests/` or `src/plugin/internal.rs` | End-to-end WASM round-trip |
| Duplicate child name detection | After I5 fix | Registry rejects dupes |

The integration test requires a compiled `patina_plugin_models.wasm` binary
available at test time. Options:
- **A)** Pre-compile and check into `tests/fixtures/` (simple, deterministic)
- **B)** Build from source in a build script (complex, fragile)
- **C)** Compile in CI as a preceding step, test reads from target dir

Pick whichever works. The test must run in `cargo test --workspace`.

#### C2: No benchmarks — measure performance exit criteria

**Audit ref:** 6.1

Measure and document:

| Metric | Spec threshold | Where to measure |
|--------|---------------|------------------|
| `PluginEngine::new()` | <100ms | Instrument with `Instant::now()` |
| `handle()` round-trip | <1ms | End-to-end: lock + serialize + WASM call + deserialize |
| `Component::new()` compilation | document only | 156KB WASM cranelift JIT time |
| `instantiate_child()` total | document only | Component + WasiCtx + Store + init + name |

Output: timing results added to the audit report or a new benchmark report.
If thresholds are met (expected), document the numbers and close. If not,
investigate before closing.

#### C3: Spec exit criteria unmet — close the loop

**Audit ref:** 1.5

After C1 and C2 are complete, verify all original SPEC.md exit criteria:

- [ ] Round-trip latency <1ms (C2)
- [ ] At least one WASM child in `cargo test` (C1)
- [ ] `wasmtime::Engine::new()` time measured (C2)

---

### Important (5)

#### I1: Amend SPEC.md — 4 inaccuracies

**Audit ref:** 1.4, 1.6, 1.7, 1.8

Add a status log entry to SPEC.md documenting these amendments:

| Section | Issue | Fix |
|---------|-------|-----|
| Phase 1 Cargo.toml | Says "no wasmtime-wasi" | Add wasmtime-wasi with note about wasm32-wasip2 |
| Files Created | API crate listed as cdylib | Move cdylib to models crate entry |
| Files Modified | registry.rs listed | Remove — WasmChild lives in plugin/internal.rs |
| Files Created | metadata.component section | Remove or mark as optional/unused |

Also note in the amendment that `wit/host.wit` is actually
`wit/deps/patina-host/host.wit` per WIT dependency resolution convention.

Do NOT rewrite the spec body — add a status log entry referencing the audit.
The spec text is historical record of the original contract.

#### I2: WIT version annotations

**Audit ref:** 1.1

Add `@0.1.0` to both WIT package declarations:

```
// wit/deps/patina-host/host.wit
package patina:host@0.1.0;

// wit/mother-child.wit
package patina:mother-child@0.1.0;
```

Verify both host-side `wasmtime::component::bindgen!` and guest-side
`wit_bindgen::generate!` still compile with versioned packages.

#### I3: Fix Mutex poison handling in WasmChild

**Audit ref:** 2.2

Replace all 5 instances of `.lock().unwrap()` in WasmChild methods with
`.lock().unwrap_or_else(|e| e.into_inner())` to match the pattern already
used in `SecretsCacheChild`.

Files: `src/plugin/internal.rs:304,312,317,329,343`

#### I4: Fix unsafe Sync comment

**Audit ref:** 2.1

Replace the safety comment at `src/plugin/internal.rs:292-293`:

```rust
// Safety: bindings::MotherChild is Send + !Sync. Its call_*() methods
// take &self (immutable) and require &mut Store (mutable). The Mutex
// on store serializes all WASM calls, preventing concurrent access.
// The instance is effectively immutable between calls.
unsafe impl Sync for WasmChild {}
```

#### I5: Duplicate child name detection

**Audit ref:** 5.1

Change `ChildRegistry::register()` to check for name conflicts:

```rust
pub fn register(&mut self, child: Box<dyn MotherChild>) -> Result<()> {
    let name = child.name().to_string();
    if self.children.iter().any(|c| c.read().unwrap().name() == name) {
        anyhow::bail!("duplicate child name: {}", name);
    }
    self.children.push(Arc::new(RwLock::new(child)));
    Ok(())
}
```

Update callers in `daemon.rs` to handle the Result (log error, skip child).

---

### Minor (1)

#### M4: Log tick() errors

**Audit ref:** 4.2

In `WasmChild::tick()`, replace the silent error swallow:

```rust
Err(e) => {
    eprintln!("[plugin:{}] tick failed: {}", self.name, e);
    vec![]
}
```

---

### Nit (5)

#### N1: Amend spec — host.wit location

**Audit ref:** 1.2

Covered by I1 spec amendment (note deps/ convention).

#### N2: Spec cdylib placement — covered by I1

Already addressed in I1 spec amendments.

#### N3: Spec registry.rs — covered by I1

Already addressed in I1 spec amendments.

#### N4: Spec metadata.component — covered by I1

Already addressed in I1 spec amendments.

#### N5: Orphaned .toml diagnostic

**Audit ref:** 5.2

In the WASM children scan loop in `daemon.rs`, after scanning `.wasm` files,
add a diagnostic for orphaned `.toml` files:

```rust
// After the .wasm scan loop:
if children_dir.exists() {
    if let Ok(entries) = std::fs::read_dir(&children_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if !path.with_extension("wasm").exists() {
                    eprintln!("[mother] orphaned manifest (no .wasm): {}", path.display());
                }
            }
        }
    }
}
```

---

## Exit Criteria

14 findings. If a finding is wrong or inapplicable during build, mark it
with a note explaining why and check it off.

### Critical
- [ ] C1: Plugin system tests written and passing in `cargo test --workspace`
- [ ] C1: At least one integration test loads WASM and calls handle()
- [ ] C2: PluginEngine::new() measured, documented (<100ms threshold)
- [ ] C2: handle() round-trip measured, documented (<1ms threshold)
- [ ] C2: Component compilation time measured, documented
- [ ] C3: All original SPEC.md exit criteria verified met

### Important
- [ ] I1: SPEC.md amended with status log entry (4 inaccuracies + host.wit location)
- [ ] I2: WIT version annotations added (@0.1.0), compilation verified
- [ ] I3: Mutex .lock().unwrap() replaced with poison recovery (5 sites)
- [ ] I4: unsafe Sync comment rewritten with precise safety argument
- [ ] I5: Duplicate child name check in ChildRegistry::register()

### Minor
- [ ] M4: tick() error logging added

### Nit
- [ ] N1-N4: Covered by I1 spec amendment
- [ ] N5: Orphaned .toml diagnostic added to daemon.rs

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`
- [ ] All new tests pass
- [ ] No regressions in existing 334+ tests

---

## Files to Change

```
# Tests (new)
src/plugin/internal.rs                  # Add #[cfg(test)] module
tests/plugin_integration.rs             # Or equivalent integration test location

# WIT fixes
wit/deps/patina-host/host.wit           # Add @0.1.0
wit/mother-child.wit                    # Add @0.1.0

# Host-side fixes
src/plugin/internal.rs                  # I3 (mutex), I4 (comment), M4 (tick log)
src/commands/mother/daemon.rs           # N5 (orphaned .toml), I5 caller update
src/commands/mother/registry.rs         # I5 (duplicate name check)

# Spec amendment
layer/surface/build/feat/plugin-system/SPEC.md  # I1 (status log entry)

# Benchmark results
layer/surface/reports/audit/2026-02-12-plugin-system-phase1.md  # C2 (append results)
```

---

## Build Order

Dependencies flow downward — complete top items before bottom:

1. **I2** — WIT version annotations. Must compile before Rust changes.
2. **I3 + I4 + M4** — `src/plugin/internal.rs` fixes (one file, batch).
3. **I5** — `registry.rs` + `daemon.rs` (register returns Result).
4. **N5** — `daemon.rs` orphaned .toml diagnostic.
5. **C1** — Tests (depends on all code changes above being in place).
6. **C2** — Benchmarks (can run after C1, or in parallel).
7. **I1** — Spec amendment (do last, references benchmark numbers from C2).
8. **C3** — Final verification of all original exit criteria.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-12 | ready | Created from audit session 20260212-075642. 14 Phase 1 findings as exit criteria. 4 Phase 2+ findings pushed outbound to [[plugin-system]] spec. |
