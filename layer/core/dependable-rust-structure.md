---
id: dependable-rust-structure
status: active
created: 2025-08-11
references: []
tags: [architecture, rust, black-box, dependable, core]
---

# Dependable Rust - Code Structure

**Purpose:** Keep a tiny, stable external interface and push changeable details behind a private internal implementation module. Easy to review, document, and evolve.

---

## Canonical layout

```
module/
├── mod.rs          # External interface: docs + curated exports (≤150 lines)
└── internal.rs     # Internal implementation details (or `internal/` folder)
```

## External interface (`mod.rs`) rules

* Keep ≤150 lines: module docs, type names, minimal constructors, `pub use` of curated items.
* No references to `internal::` in public signatures.
* Provide a single `Error` enum (`#[non_exhaustive]` if appropriate).
* Add at least one runnable doctest.

## Internal implementation (`internal.rs` or `internal/`) rules

* Default to `pub(crate)`; only the external interface decides what becomes `pub`.
* Keep helpers and heavy logic here; split into files when it grows: `internal/{mod,parse,exec,validate}.rs`.
* Use trait objects or sealed traits internally; export stable traits only when necessary.

## Wiring options

**A) Re‑export items defined in `internal` (fast iteration)**

```rust
// mod.rs
mod internal; // private
pub use internal::{Client, Config, Result, run};
```

**B) Define types in `mod.rs`, impls in `internal` (stable names)**

```rust
// mod.rs
mod internal;
pub struct Client { /* private fields */ }
// heavy impls live in internal.rs
```

## Naming policy

* Default: `internal.rs` (or `internal/`). Team‑approved alternatives: `implementation.rs` or `imp.rs`.
* Reserve `sys/` or `ffi/` for low‑level bindings; `sealed.rs` only for sealing patterns.

## Visibility pattern

```rust
// External interface
mod internal;                 // not `pub`
pub use internal::{Client};   // curate API

// Internal implementation
pub(crate) struct Engine;     // crate‑internal helper
```

## Testing strategy

* **Doctests** in `mod.rs` show intended usage.
* **Unit tests** colocated under `internal/*` for edge cases.
* **Integration tests** in `tests/` exercise only the external interface.

## CI guards (lightweight)

* Fail if `module/mod.rs` > 150 lines.
* Forbid `pub mod internal`.
* Block `internal::` in public signatures.

```bash
# scripts/check_interface.sh
set -euo pipefail
file="$1/mod.rs"; [ $(wc -l < "$file") -le 150 ] || { echo "Interface too large: $file"; exit 1; }
```

## Cross‑language quick bridge (for teammates)

* **C:** `module.h` (small) + `module.c` (guts) ≈ external interface + internal implementation.
* **TS:** `index.ts` barrel (exports) + `internal.ts` (not exported) ≈ external interface + internal implementation.
* **Go:** `api.go` in public package + `internal/` implementation ≈ external interface + internal implementation.