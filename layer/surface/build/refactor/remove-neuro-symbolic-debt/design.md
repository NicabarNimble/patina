# Design: Remove Neuro-Symbolic Tech Debt

## Origin Story

In November 2025, the `neuro-symbolic-knowledge-system` branch explored an ambitious architecture:

| Date | Session | What Was Built |
|------|---------|----------------|
| Nov 3, 2025 | `20251103-063514` | `BeliefStorage` - SQLite + USearch hybrid |
| Nov 3, 2025 | `20251103-111458` | `ObservationStorage`, `SemanticSearch` wrapper |
| Nov 6, 2025 | `20251106-111208` | `ReasoningEngine` - Embedded Scryer Prolog |

**The vision**: Observations scraped from sessions → vector DB → Prolog validates evidence → beliefs auto-created.

**What happened instead**:
- Beliefs became markdown files (skill-driven, human-authored)
- Scry uses `QueryEngine` (retrieval module), not `SemanticSearch`
- Prolog reasoning was never called from anywhere
- The exploration diverged from the production path

---

## What's Unused

### 1. `src/reasoning/` - Prolog Engine (Never Called)

```
src/reasoning/
├── mod.rs                    (8 lines - exports)
├── engine.rs                 (448 lines - ReasoningEngine)
├── confidence-rules.pl       (213 lines - Prolog)
└── validation-rules.pl       (167 lines - Prolog)
```

**Total: ~836 lines**

No callers in codebase:
```bash
$ grep -r "ReasoningEngine\|reasoning::" src/commands/ src/mcp/
# Only found in manifest.rs as a comment about future integration
```

### 2. `src/storage/` - Belief/Observation Storage (Superseded)

```
src/storage/
├── mod.rs                    (27 lines - exports)
├── types.rs                  (55 lines - Belief, Observation structs)
├── beliefs.rs                (330 lines - BeliefStorage)
└── observations.rs           (~300 lines - ObservationStorage)
```

**Total: ~712 lines**

Only caller is `src/query/semantic_search.rs` (also unused).

### 3. `src/query/semantic_search.rs` - Unused Wrapper

```
src/query/
├── mod.rs                    (exports SemanticSearch - unused)
└── semantic_search.rs        (454 lines)
```

**Total: ~460 lines**

The active search system is `src/retrieval/` (QueryEngine with oracles).

### 4. Test Files for Dead Code

```
tests/neuro_symbolic_integration.rs   (~100 lines)
tests/semantic_search_integration.rs  (~80 lines)
```

**Total: ~180 lines**

These test the unused modules.

---

## Dependencies to Remove

### `scryer-prolog = "0.10.0"`

- **Only used by**: `src/reasoning/engine.rs`
- **Impact**: Significant compile-time dependency (Prolog interpreter)
- **Safe to remove**: Yes, after deleting reasoning module

---

## What Stays

| Module | Why |
|--------|-----|
| `src/retrieval/` | Active - powers `patina scry` |
| `src/commands/scrape/` | Active - populates eventlog |
| `src/commands/scry/` | Active - query interface |
| `src/commands/persona/` | Active - cross-project knowledge |
| `src/embeddings/` | Active - used by retrieval oracles |

---

## Deletion Order (Dependencies)

```
Phase 1: Remove leaf modules (no dependents)
├── Delete src/reasoning/        # No callers
├── Delete tests/neuro_symbolic_integration.rs
└── Remove scryer-prolog from Cargo.toml

Phase 2: Remove storage (only caller is semantic_search)
├── Delete src/storage/
└── Delete tests/semantic_search_integration.rs

Phase 3: Remove semantic_search wrapper
├── Delete src/query/semantic_search.rs
├── Update src/query/mod.rs (remove export)
└── Update src/lib.rs (remove storage, reasoning exports)

Phase 4: Verify
├── cargo build --release
├── cargo test --workspace
└── cargo clippy --workspace
```

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Hidden runtime caller | Low | Grep shows no callers outside test code |
| Breaking change | Low | These modules aren't exported in public API |
| Losing useful code | Low | Git history preserves everything |

---

## Verification Commands

```bash
# Before deletion - confirm no callers
grep -r "BeliefStorage\|ObservationStorage" src/commands/ src/mcp/
grep -r "SemanticSearch" src/commands/ src/mcp/
grep -r "ReasoningEngine" src/commands/ src/mcp/

# After deletion - confirm builds
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets

# Check binary size reduction
ls -la target/release/patina
```

---

## Session References

- `20251103-063514` - BeliefStorage implementation
- `20251103-111458` - ObservationStorage, SemanticSearch
- `20251106-111208` - ReasoningEngine with Scryer Prolog
- `20260120-165543` - This session (identifying tech debt)
