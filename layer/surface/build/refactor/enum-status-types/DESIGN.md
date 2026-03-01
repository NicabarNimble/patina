# Design: Replace status: String with typed enums

## Approach

Follow the existing `VerificationStatus` pattern (scrape/beliefs/verification/mod.rs):
derive Serialize/Deserialize with `rename_all`, implement `as_str()` for display,
use exhaustive match everywhere.

### Enum definitions

All enums get the same derive set for consistency:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
```

`Copy` because all variants are fieldless — cheap to pass by value.

### ADR-1: Where to define SpecStatus

**Context:** `SpecFrontmatter` lives in `src/spec.rs` (the crate root lib).
It's used by `src/commands/spec/`, `src/mcp/server/spec.rs`, and
`src/commands/session/internal.rs`. The enum must be accessible to all.

**Decision:** Define `SpecStatus` in `src/spec.rs` alongside `SpecFrontmatter`.
Same file, same module. No new files needed.

### ADR-2: The "done" legacy alias

**Context:** 5+ comparison sites check `== "complete" || == "done"`. The "done"
value may exist in old spec files or in the spec_deps table.

**Detection:** `rg '"done"' layer/surface/` to check for on-disk usage.

**Decision:** Add `#[serde(alias = "done")]` to the `Complete` variant:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Draft,
    Ready,
    Active,
    Paused,
    Blocked,
    #[serde(alias = "done")]
    Complete,
    Abandoned,
}
```

This deserializes both "complete" and "done" to `SpecStatus::Complete`.
Serialization always writes "complete" (the canonical rename). No data migration.

Add `is_terminal()` helper to replace the `== "complete" || == "done"` pattern:

```rust
impl SpecStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Abandoned)
    }
}
```

### ADR-3: Where to define BeliefStatus

**Context:** Used by `ParsedBelief` in `src/commands/scrape/beliefs/mod.rs`
and `BeliefEntry` in `src/mother/graph.rs` (the library crate). Both crates
need the type.

**Decision:** Define `BeliefStatus` in `src/mother/graph.rs` (library crate,
already defines `BeliefEntry`). The binary crate (`src/commands/scrape/beliefs/`)
imports it via `use patina::mother::graph::BeliefStatus;` — same pattern as
importing `BeliefEntry` today.

If `mother/graph.rs` is too heavy, define in a new `src/mother/types.rs` and
re-export. But start simple.

### ADR-4: Database TEXT columns

**Context:** BeliefEntry.status is stored as TEXT in SQLite (`mother/graph.rs`).
Changing the Rust type doesn't change the column type — SQLite is dynamically
typed. Serde handles the conversion.

**Decision:** At the database boundary, read as String then match to enum:

```rust
impl BeliefStatus {
    pub fn from_str_or_default(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "scoped" => Self::Scoped,
            "defeated" => Self::Defeated,
            "archived" => Self::Archived,
            _ => Self::Active, // safe default for unknown values
        }
    }
}
```

No new dependencies needed. Keep `.get::<_, String>()` at the DB boundary.

### ADR-5: Option<SpecStatus> vs SpecStatus

**Context:** `SpecFrontmatter.status` is currently `Option<String>` with
`skip_serializing_if = "Option::is_none"`. Some spec files might not have
a status field (though all should).

**Decision:** Keep `Option<SpecStatus>` with the same skip_serializing_if.
Default to `SpecStatus::Draft` when None at the call site (matches current
behavior where missing status implies draft). Don't change the optionality
semantics — that's a separate concern.

## Commits

1. `spec: define SpecStatus enum with serde rename and "done" alias`
2. `spec: migrate SpecFrontmatter.status to SpecStatus enum`
3. `spec: replace all string comparisons with enum matching`
4. `beliefs: define BeliefStatus enum, migrate ParsedBelief and BeliefEntry`
5. `doctor: define HealthStatus enum, migrate HealthCheck`
6. `assay: define ActivityLevel enum, migrate ModuleSignal`

## Key Files

**Definitions (new enums):**
- `src/spec.rs` — SpecStatus
- `src/mother/graph.rs` — BeliefStatus
- `src/commands/doctor.rs` — HealthStatus
- `src/commands/assay/internal/derive.rs` — ActivityLevel

**Highest-touch migrations:**
- `src/commands/spec/internal/mutations.rs` — 7 state transitions, ~10 comparisons
- `src/commands/spec/internal/queries.rs` — ~6 comparisons, filtering functions
- `src/commands/spec/internal/queue.rs` — blocker resolution
- `src/commands/scrape/beliefs/mod.rs` — ParsedBelief construction
- `src/mother/graph.rs` — BeliefEntry DB read/write

**Reference implementations (existing enums):**
- `src/commands/scrape/beliefs/verification/mod.rs` — VerificationStatus
- `src/commands/measure/internal.rs` — VerbStatus
- `src/mother/child.rs` — ChildHealth
