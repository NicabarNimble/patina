---
id: adapter-pattern
layer: core
status: active
created: 2025-08-02
revised: 2026-04-03
tags: [architecture, patterns, traits, external-systems, gjengset]
references: [dependable-rust, unix-philosophy, gjengset-lens-type-integrity]
---

# Adapter Pattern

**Purpose:** Define trait boundaries at external system edges. Prove the boundary with real implementations — don't abstract speculatively.

---

## Core Principle

When Patina touches an external system (database, AI interface, embedding model, file format), put a trait boundary in front of it. The trait is the contract. Implementations are black boxes. But only introduce the trait when you have 2+ real implementations — a trait with one implementation is ceremony, not architecture.

This is the Gjengset principle applied to system boundaries: honest signatures, type integrity at the seam, and proof before abstraction.

## Adapter vs Strategy

Not every trait boundary is an adapter. The distinction matters:

- **Adapter**: bridges an external system into Patina's domain. The implementation wraps vendor-specific code. Examples: `EmbeddingEngine` (wraps ONNX), `InterfaceProvider` (wraps Claude/Gemini/OpenCode), `Provider` (wraps OAuth flows).
- **Strategy**: selects among internal algorithms. No external system involved. Example: `Oracle` (retrieval strategies — semantic, temporal, lexical — all internal to Patina).

The Oracle trait explicitly documents this: "This is a strategy pattern (not adapter pattern) because oracles are internal retrieval mechanisms, not external system integrations."

Use adapter when crossing a system boundary. Use strategy when choosing among internal approaches. Both use traits; only adapters isolate external dependencies.

## When to Use

Apply this pattern when:
- 2+ implementations exist today (not "might exist someday")
- An external system may change independently of Patina (DuckDB versions, LLM APIs, embedding models)
- You need to swap implementations without changing calling code

**Real examples in Patina:**

| Trait | Boundary | Implementations |
|-------|----------|-----------------|
| `EmbeddingEngine` | ONNX runtime | `OnnxEmbedder` (symmetric + asymmetric models) |
| `InterfaceProvider` | AI tools | `ClaudeAdapter`, `GeminiAdapter`, `OpenCodeAdapter` |
| `RegistryBackend` | Mother registry | `RepoRegistryBackend` + test mocks |
| `ScryBackend` | Retrieval engine | `RetrievalScryBackend` + test mocks |
| `Provider` | OAuth/credentials | GitHub, Linear, Slack providers |
| `ForgeWriter` | Git forges | GitHub writer |

## When NOT to Use

- Only one implementation exists and no second is planned — use a module, not a trait
- Internal code talking to internal code — modules and function calls, not trait objects
- The "abstraction" just forwards calls — wrapper tax with no benefit
- You're guessing where the seam is — wait until the second implementation proves it

## How to Apply

### 1. Honest Signatures

The trait should declare exactly what it needs. No smuggling dependencies through config bags.

```rust
// ❌ Bad: hides what the function actually depends on
pub fn query(config: &AppConfig) -> Result<Vec<Hit>> {
    let engine = config.get_scry_backend();  // hidden dependency
    engine.query(config.query_text(), 10, None, false)
}

// ✅ Good: dependency is visible in the signature
pub fn query(backend: &dyn ScryBackend, query: &str, limit: usize) -> Result<Vec<Hit>> {
    backend.query(query, limit, None, false)
}
```

### 2. Prove the Boundary

A trait with one implementation is a hypothesis. Two implementations prove the seam is in the right place.

```rust
// Proven boundary: EmbeddingEngine
// - OnnxEmbedder (production, multiple model families)
// - embed()/embed_query()/embed_passage() contract proven by
//   symmetric vs asymmetric model implementations

// Unproven: don't create a trait
// - If you only have SQLite storage and no plan for PostgreSQL,
//   just use SQLite directly. Add the trait when the second backend arrives.
```

### 3. Keep Traits Minimal

3-7 methods is typical. If a trait grows beyond that, it's doing too much — split it or push methods into the implementation.

```rust
// ✅ Good: focused trait
pub trait EmbeddingEngine: Send {
    fn embed(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_query(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>>;
}
// Model-specific logic (prefixes, tokenization) stays in the implementation.
```

### 4. Domain Types at the Boundary

The trait uses Patina's domain types. Implementation-specific types stay behind the boundary.

```rust
// ❌ Bad: leaks implementation type
pub trait Backend {
    fn query(&self) -> rusqlite::Rows;  // caller now depends on rusqlite
}

// ✅ Good: domain type at the boundary
pub trait Backend {
    fn query(&self, q: &str, limit: usize) -> Result<Vec<ScryHit>>;  // Patina's type
}
```

### 5. Combine with Dependable-Rust

Each adapter implementation is a black-box module:

```
src/interface/runtime/claude/
├── mod.rs          # InterfaceProvider impl (public contract)
└── internal/       # Claude-specific logic (templates, paths, manifest)
```

The trait lives in the parent module. Implementations live in their own subdirectories. Nothing in the implementation leaks into the trait.

## Testing

Prefer integration tests with real implementations. Trait boundaries exist to isolate external systems, not to invite mocks. Test the real thing whenever possible — real DuckDB connections, real file I/O, real ONNX inference.

Mocks are a last resort for when the real system is genuinely unavailable in CI (external APIs requiring credentials, third-party services with rate limits). Even then, prefer a lightweight real implementation (in-memory database, local test server) over a mock that fakes behavior.

## Common Mistakes

**1. Abstracting with one implementation**
```rust
// ❌ Bad: trait exists "just in case"
trait CacheBackend { fn get(&self, key: &str) -> Option<String>; }
struct RedisCacheBackend;  // the only implementation
// Just use Redis directly. Add the trait when you need a second backend.
```

**2. Leaking implementation types**
```rust
// ❌ Bad: trait exposes vendor type
trait Storage { fn connection(&self) -> &duckdb::Connection; }
// ✅ Good: trait exposes domain operations
trait Storage { fn store_fact(&self, fact: &Fact) -> Result<()>; }
```

**3. Oversized traits**
```rust
// ❌ Bad: 15 methods — some callers only need 2
trait FullService {
    fn query(&self, ...) -> Result<...>;
    fn index(&self, ...) -> Result<()>;
    fn delete(&self, ...) -> Result<()>;
    fn migrate(&self, ...) -> Result<()>;
    // ... 11 more
}
// ✅ Good: split into focused traits
trait Queryable { fn query(&self, ...) -> Result<...>; }
trait Indexable { fn index(&self, ...) -> Result<()>; }
```

## References

- [Dependable Rust](./dependable-rust.md) — How to structure each implementation as a black box
- [Unix Philosophy](./unix-philosophy.md) — Each implementation does one thing
- [gjengset-lens-type-integrity](../surface/epistemic/beliefs/gjengset-lens-type-integrity.md) — Encode invariants in types, honest signatures
- [boundary-string-internal-enum](../surface/epistemic/beliefs/boundary-string-internal-enum.md) — String at serialization boundary, enum internally
