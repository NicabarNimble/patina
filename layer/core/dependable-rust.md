---
id: dependable-rust
layer: core
status: active
created: 2025-08-11
tags: [architecture, rust, black-box, module-pattern]
references: [unix-philosophy]
---

# Dependable Rust

**Purpose:** Keep a tiny, stable external interface and push changeable details behind a private internal implementation module. Easy to review, document, and evolve.

---

## Core Principle

Keep your public interface small and stable. Hide implementation details in `internal.rs` or `internal/` and never expose them in public signatures. This creates black-box modules that can be completely rewritten internally without breaking users.

**Not a line count rule - a design principle.**

## When to Use

Apply this pattern when you need to isolate change or manage complexity:

**Use internal.rs for:**
- ✅ Adapters bridging external systems (hide vendor details)
- ✅ Complex modules with many private helpers
- ✅ Code you expect to rewrite often (isolate churn from interface)

**Don't split when:**
- ✅ Simple commands (sequential steps, no abstraction needed)
- ✅ Naturally small modules (one file is clearer)
- ✅ Procedural code with no hidden complexity

## How to Apply

### 1. Canonical Layout

```
module/
├── mod.rs          # External interface: docs + curated exports
└── internal.rs     # Internal implementation details (or `internal/` folder)
```

### 2. External Interface Rules (`mod.rs`)

- Keep interface small: module docs, type names, minimal constructors, `pub use` statements
- No references to `internal::` in public signatures
- Provide a single `Error` enum (`#[non_exhaustive]` if appropriate)
- Add at least one runnable doctest showing usage

### 3. Internal Implementation Rules (`internal.rs` or `internal/`)

- Default to `pub(crate)`; only the external interface decides what becomes `pub`
- Keep helpers and heavy logic here
- Split into files when it grows: `internal/{mod,parse,exec,validate}.rs`
- Use trait objects or sealed traits internally; export stable traits only when necessary

### 4. Wiring Options

**A) Re-export items defined in `internal` (fast iteration)**

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

### 5. Visibility Pattern

```rust
// External interface (mod.rs)
mod internal;                 // not `pub`
pub use internal::{Client};   // curate API

// Internal implementation (internal.rs)
pub(crate) struct Engine;     // crate-internal helper
```

## The "Do X" Test

Before creating a module, ensure you can clearly state what it does in one sentence:

**Good "Do X" (Clear modules):**
- ✅ "Parse TOML into data structure"
- ✅ "Compress files using gzip"
- ✅ "Calculate hash of data"

**Bad "Do X" (Unclear scope):**
- ❌ "Manage workspaces" (too vague - what does "manage" mean?)
- ❌ "Handle project initialization" (multiple responsibilities)
- ❌ "Integrate with build tools" (unbounded scope)

**When "Do X" is unclear:**
1. **Split it**: Break into multiple black boxes
2. **Layer it**: Stable core + replaceable adapters
3. **Accept it**: Some code is glue - optimize for clarity over permanence

## Testing Strategy

- **Doctests** in `mod.rs` show intended usage
- **Unit tests** colocated under `internal/*` for edge cases
- **Integration tests** in `tests/` exercise only the external interface

## Enforcement

Check for pattern violations:

**What to check:**
- ✅ No `pub mod internal` (keeps internal private)
- ✅ No `internal::` in public function signatures (prevents leakage)
- ✅ Internal types not exposed in public API

**When to check:**
- During code review
- When interface feels cluttered
- When adding new public API

## Common Mistakes

**1. Exposing internal types in public API**
```rust
// ❌ Bad: leaks implementation
pub fn get_engine(&self) -> &internal::Engine

// ✅ Good: opaque or re-exported type
pub fn get_engine(&self) -> &Engine  // re-exported from internal
```

**2. Making internal module public**
```rust
// ❌ Bad: users can bypass your API
pub mod internal;

// ✅ Good: keep private
mod internal;
pub use internal::{SelectedTypes};
```

**3. Splitting when unnecessary**
```rust
// ❌ Bad: artificial split for simple procedural code
// mod.rs (30 lines of boilerplate)
// internal.rs (40 lines of sequential steps)

// ✅ Good: keep simple code simple
// mod.rs (70 lines - clear, sequential, no abstraction needed)
```

## Cross-Language Quick Bridge

For teammates familiar with other languages:

- **C:** `module.h` (small) + `module.c` (guts) ≈ `mod.rs` + `internal.rs`
- **TypeScript:** `index.ts` (exports) + `internal.ts` (not exported) ≈ `mod.rs` + `internal.rs`
- **Go:** `api.go` in public package + `internal/` ≈ `mod.rs` + `internal.rs`

## References

- [Unix Philosophy](./unix-philosophy.md) - Decomposition principle (systems → tools)
- [Adapter Pattern](./adapter-pattern.md) - When building external bridges
- [Eskil Steenberg Rust](../surface/eskil-steenberg-rust.md) - Philosophy behind stable APIs
