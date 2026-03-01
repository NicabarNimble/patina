---
type: audit
id: structural-audit-type-soup
scope: src/mcp/server/, src/commands/measure/internal.rs, src/plugin/internal/mod.rs
spec: layer/surface/build/refactor/mcp-typed-handlers/SPEC.md
related: layer/surface/build/refactor/data-architecture-v2/SPEC.md
session: 20260301-090927
created: 2026-03-01
status: complete
findings:
  critical: 1
  important: 2
  minor: 3
  nit: 2
verdict: >
  MCP type soup is the dominant structural debt. Module encapsulation,
  globals, scoring, and validation are clean. Recommend spec mcp-typed-handlers
  for the MCP boundary; measure type soup deferred to data-measure-surface.
---

# Structural Audit — Type Soup & Invariant Analysis

> Systematic scan of patina's 216 Rust source files (73k LOC) against 8
> structural failure modes commonly found in LLM-generated Rust codebases.
> Prompted by an external structural audit spec prompt; findings grounded
> in ripgrep counts and file-level analysis.

## Methodology

Three parallel analysis agents scanned `src/` for:
1. Type soup signals (serde_json::Value, HashMap, .get() chains, .as_*(), .unwrap_or(), .ok())
2. Module structure (pub use, pub(crate), OnceLock, globals, circular deps)
3. Validation drift (validate/parse/normalize functions, error types, unwrap, god structs, magic numbers)

All counts verified via ripgrep. Representative code samples read for context.

---

## Failure Mode Assessment

### 8 modes checked, 3 actionable, 5 clean

| # | Failure Mode | Status | Severity |
|---|-------------|--------|----------|
| 1 | Type soup domain model | **FAIL** | Critical |
| 2 | Inconsistent representation | Clean | — |
| 3 | Encapsulation theater | Clean | — |
| 4 | Validation drift | Clean | — |
| 5 | Dependency graph distress | Clean | — |
| 6 | Metrics system pathology | **WARN** | Important |
| 7 | Tool truthfulness failures | **WARN** | Important |
| 8 | Async / performance footguns | Clean | — |

---

## 1. FINDINGS

### F-001: MCP Handler Type Soup — CRITICAL

**Severity: Critical**
**Scope:** `src/mcp/server/scry.rs`, `src/mcp/server/spec.rs`, `src/mcp/server/assay.rs`

All MCP tool handlers receive `&serde_json::Value` and manually extract
every parameter via `.get("key").and_then(|v| v.as_str()).unwrap_or("")`
chains. Measured burden:

| Module | `.get()` | `.as_*()` | `.unwrap_or()` | Total |
|--------|----------|-----------|----------------|-------|
| scry.rs (1,225 LOC) | 28 | 33 | 42 | **103** |
| spec.rs (446 LOC) | 31 | 28 | 22 | **81** |
| assay.rs (604 LOC) | 6 | 4 | 6 | **16** |
| **Total** | **65** | **65** | **70** | **200** |

**Impact:**
- Missing required params silently default to `""` or `false` — fails deeper with misleading error
- Typos in string keys compile fine, silently return None
- No compile-time verification of parameter names or types

**Remediation:** Spec [[mcp-typed-handlers]] created — `#[derive(Deserialize)]` structs per handler, `serde_json::from_value()` at dispatch boundary.

**Evidence:**
```rust
// src/mcp/server/spec.rs — 31 instances of this pattern
let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
let major = args.get("major").and_then(|v| v.as_bool()).unwrap_or(false);
let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
```

### F-002: Measure Internal Type Soup — IMPORTANT

**Severity: Important**
**Scope:** `src/commands/measure/internal.rs`

The measure command uses `serde_json::Value` for metrics, requiring 78
type soup operations (30 `.get()` + 25 `.as_*()` + 23 `.unwrap_or()`).

```rust
// src/commands/measure/internal.rs — metrics accessed via runtime casting
if let Some(p5) = src.latest_metrics.get("p_at_5").and_then(|v| v.as_f64()) { ... }
let total = s.latest_metrics.get("total_beliefs")?.as_f64()?;
```

**Remediation:** Deferred to [[data-measure-surface]] spec — measure's
JSON structure is being redesigned for LLM consumers. Typing the metrics
should happen as part of that redesign, not independently.

### F-003: Residual .ok() Silent Swallowing — IMPORTANT

**Severity: Important**
**Scope:** Codebase-wide (93 files)

221 `.ok()` occurrences remain after mcp-server-hardening eliminated 18 in
`src/mcp/server/`. Breakdown by category:

| Category | Count | Risk |
|----------|-------|------|
| `filter_map(\|e\| e.ok())` on directory iteration | ~80 | Low (benign — skipping unreadable entries) |
| `current_dir().ok()` and path operations | ~30 | Low (fallback is reasonable) |
| File I/O in non-critical paths | ~40 | Low-Medium |
| Database/event operations | ~20 | Medium (errors lose context) |
| Result-to-Option conversions | ~50 | Variable |

**Remediation:** No single spec needed. The high-risk subset (database ops,
event recording) should be addressed incrementally. The bulk are benign
directory traversal patterns.

### F-004: Plugin Manifest Parsing Type Soup — MINOR

**Severity: Minor**
**Scope:** `src/plugin/internal/mod.rs`

45+ `.get()` chains parsing `toml::Value` for plugin manifests. Different
boundary (TOML, not MCP), but same pattern.

**Remediation:** Consider `#[derive(Deserialize)]` on `PluginManifest` with
serde toml. Lower priority — plugin manifests are parsed once at load time,
not in a hot loop.

### F-005: Unwrap Calls in Non-Test Code — MINOR

**Severity: Minor**
**Scope:** Scattered across 6-10 production sites

338 total `.unwrap()` calls, most in tests. ~10 in production code:
- `src/embeddings/onnx.rs` — embed() operations (could panic if model fails)
- `src/secrets/` — TOML serialization (generally safe)
- `src/plugin/scaffold.rs` — temp directory operations

35 `.expect()` calls provide marginally better diagnostics but still panic.

**Remediation:** Convert production `.unwrap()` to `.context("...")` with
anyhow. No spec needed — mechanical fix when touching these files.

### F-006: BeliefEntry Struct (23 fields) — MINOR

**Severity: Minor**
**Scope:** `src/mother/graph.rs:148`

BeliefEntry has 23 fields spanning identity, metrics, grounding, verification,
and temporal data. Looks like a god struct.

**Assessment: Intentional.** This is a materialized view for cross-project
search, flattened to avoid N+1 queries. The fields decompose cleanly into
5 groups (identity:7, metrics:7, grounding:5, verification:4, temporal:1).
Not a business logic god struct — no methods beyond construction.

**Remediation:** None. Document the field groups if it grows further.

### F-007: ScryOptions Struct (14 fields, 2 deprecated) — NIT

**Severity: Nit**
**Scope:** `src/commands/scry/mod.rs:44`

ScryOptions has 14 fields including `legacy: bool` (deprecated, removed
in v0.12.0) and `full: bool` (deprecated escape hatch). Dead fields
increase cognitive load.

**Remediation:** Remove deprecated fields when next touching scry options.

### F-008: Duplicate normalize_path Functions — NIT

**Severity: Nit**
**Scope:** `src/commands/oxidize/commits.rs:221`, `src/commands/eval/internal/helpers.rs:6`, `src/commands/eval/mod.rs:917`

Three separate `normalize_path()` functions with similar logic.

**Remediation:** Extract to a shared utility if these modules are refactored.
Not urgent — each is tailored to its context.

---

## 2. CLEAN AREAS (No Action Required)

### Module Encapsulation — PASS

- Only **1** glob re-export (`pub use *`) in all of `src/` (`forge/mod.rs`)
- **53** targeted `pub use` re-exports with explicit names
- **69** `pub(crate)` usages across 16 files
- Consistent `mod internal; pub use internal::{...}` pattern per [[dependable-rust]]
- No `pub use internal::*` facade onion pattern

### Global State — PASS

- **7** files use `OnceLock` — all justified:
  - Wasmtime `Engine` singleton (standard pattern from Zed)
  - Database path constants
  - Commit SHA caching in scrape pipeline
- Zero `lazy_static!`, zero `static mut`, zero global registries

### Scoring/Weights — PASS

- **89** float literals in scoring contexts, but:
  - Weight constants defined as module-level `const` (`WEIGHT_MIN`, `WEIGHT_MAX`, `DEFAULT_ALPHA`)
  - EMA formula documented: `weight_new = (1 - α) × weight_old + α × (1.0 + precision)`
  - Edge boost factors are small and documented (1.1-1.2x)
  - No magic numbers in hot paths

### Validation — PASS

- **9** validate functions at boundaries (SQL safety, HTTP URL, schema, CLI)
- **27** parse functions with consistent error propagation
- **5** normalize functions (vector normalization + paths)
- **2** custom error types + anyhow everywhere (appropriate for CLI tool)
- No scattered coerce/normalize sprawl

### Dependency Graph — PASS

- No circular module dependencies
- No feature flag labyrinths
- No conditional compilation as architecture crutch
- Clean module tree with explicit dependencies

---

## 3. SIGNAL MEASUREMENTS

| Signal | Count | Threshold | Status |
|--------|-------|-----------|--------|
| `serde_json::Value` in src/ | 94 | >50 outside boundary → type soup | **FAIL** (concentrated in MCP) |
| `.get("key")` chains | 400+ | >100 → type soup | **FAIL** |
| `.as_str()` calls | 241 | >100 → excessive casting | **FAIL** |
| `.unwrap_or_default()` in non-test | 99 | >200 → silent failure | Pass |
| `.unwrap_or()` in non-test | 184 | >200 → silent fallback | Pass (borderline) |
| `.ok()` in non-test | 221 | >100 → error swallowing | **WARN** (mostly benign) |
| `pub use *` glob re-exports | 1 | >10 → facade theater | Pass |
| `pub(crate)` usage | 69 | >20 → good discipline | Pass |
| `OnceLock/lazy_static` globals | 7 | >10 → global registry risk | Pass |
| Float magic numbers in scoring | 89 | >50 with no constants | Pass (has constants) |
| Custom error types | 2 | <3 → anyhow monoculture | Note (intentional for CLI) |
| `unwrap()` in non-test | ~10 | >20 → panic risk | Pass |

---

## 4. RELATIONSHIP TO EXISTING SPECS

| Concern | Coverage | Status |
|---------|----------|--------|
| MCP .ok() swallowing | [[mcp-server-hardening]] (v0.35.1) | Shipped |
| MCP error codes | [[mcp-server-hardening]] (v0.35.1) | Shipped |
| MCP handler type soup | **[[mcp-typed-handlers]]** (new) | Draft |
| Event type safety | [[data-architecture-v2]] (deferred with trigger) | Tracked |
| ATTACH newtype | [[data-architecture-v2]] (deferred with trigger) | Tracked |
| Measure type soup | [[data-measure-surface]] (future) | Draft |

---

## 5. BELIEFS VERIFIED

- **[[correctness-by-construction-not-convention]]** — The type soup
  violates this: parameter correctness depends on convention (matching
  string keys and type casts) rather than construction (typed structs
  where the compiler enforces field names and types).

- **[[question-mark-on-option-is-silent-swallower]]** — Related pattern:
  `.unwrap_or("")` on missing params is the same class of silent failure
  that this belief warns about. The `?` swallows silently; `.unwrap_or`
  defaults silently. Both hide the real problem.

---

## 6. RECOMMENDATIONS (Priority Order)

1. **Promote and build [[mcp-typed-handlers]]** — highest ROI refactor.
   Eliminates ~200 type soup operations, adds compile-time safety, improves
   error messages for MCP clients. Mechanical work, low risk.

2. **Remove deprecated ScryOptions fields** — opportunistic cleanup when
   next touching scry. Two-line change.

3. **Convert production unwrap() to context()** — opportunistic when
   touching embeddings, secrets, plugin modules.

4. **Plugin manifest Deserialize** — when plugin system evolves further,
   replace toml::Value parsing with derive(Deserialize) on PluginManifest.

5. **data-measure-surface includes measure type soup** — ensure that spec
   addresses the 78 type soup operations in measure/internal.rs.
