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

**Context:** 6 comparison sites check `== "complete" || == "done"` (see SPEC.md
for the full list). The "done" value might exist in the spec_deps DB table from
historical data.

**Grounding:** `rg '"done"' layer/surface/` and `rg 'status: done' layer/`
both return zero matches. No spec files on disk carry `status: done`. The alias
is **defensive only** — it protects against DB or tag data that may contain
"done" but requires no data migration.

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

**Context:** Both spec and belief statuses are stored as TEXT in SQLite.
`FoundSpec.status` and `LoadedSpec.status` (archive.rs) read from the
patterns table. `BeliefEntry.status` reads from mother/graph.rs.

**Decision:** Parse at the DB boundary — `FoundSpec.status` and
`LoadedSpec.status` become `Option<SpecStatus>`. The conversion happens
in `find_spec()` and `load_spec()` via `FromStr`:

```rust
impl std::str::FromStr for SpecStatus {
    type Err = SpecStatusError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "ready" => Ok(Self::Ready),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "blocked" => Ok(Self::Blocked),
            "complete" | "done" => Ok(Self::Complete),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(SpecStatusError { got: s.to_string() }),
        }
    }
}
```

Unknown values surface a **clear error** rather than silently defaulting.
This is the [[parse-at-boundary-type-the-interior]] belief: the DB row is
the boundary, the interior flows typed data only.

For `BeliefStatus`, use the same pattern but with a safe default since
belief data is less critical to control flow:

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

No new dependencies needed. Keep `.get::<_, String>()` at the DB boundary,
then immediately parse.

### ADR-5: Option<SpecStatus> vs SpecStatus

**Context:** `SpecFrontmatter.status` is currently `Option<String>` with
`skip_serializing_if = "Option::is_none"`. Some spec files might not have
a status field (though all should).

**Decision:** Keep `Option<SpecStatus>` with the same skip_serializing_if.
Default to `SpecStatus::Draft` when None at the call site (matches current
behavior where missing status implies draft). Don't change the optionality
semantics — that's a separate concern.

### ADR-6: Query/display structs use SpecStatus

**Context:** `SpecInfo.status`, `ReadySpec.status`, `Blocker.status`, and
`BlockedSpec.status` are `pub` structs serialized to JSON for MCP. They
could stay `String` (minimal blast radius) or become `SpecStatus`.

**Decision:** Use `SpecStatus` in all query/display structs. Serde's
`rename_all = "snake_case"` serializes identically to the current string
values, so JSON output is backward-compatible. Benefits:
- Eliminates mixed string/enum types in the codebase
- Helpers like `is_terminal()` work on query results without conversion
- Compiler enforces exhaustive matching in display logic

Exception: `MutationResult.new_status` and `MutationDetail::Resume.previous_status`
stay `String` — they are output-only display fields that mirror the enum
via `.as_str().to_string()`. Converting them adds no safety value.

### ADR-7: ListFilters.status parses at CLI/MCP boundary

**Context:** `ListFilters.status: Option<String>` accepts user input from
CLI args and MCP tool parameters. Currently passed through as a raw string
for comparison.

**Decision:** Change to `Option<SpecStatus>`. Parse user input at the
CLI/MCP boundary using `FromStr`. Unknown status values return a validation
error (e.g., "invalid spec status 'actve'") rather than silently producing
an empty result set. This catches typos at the input boundary rather than
deep in the filter logic.

## Lessons from First Attempt (session 20260301-165723)

Attempted Phase 1 mid-session with shrinking context window. Reverted.
Key findings from the partial implementation:

1. **Cascading type change is wider than expected.** Changing
   `SpecFrontmatter.status` from `Option<String>` to `Option<SpecStatus>`
   triggers 15 compiler errors across 5 files. The cascade:
   - `SpecFrontmatter.status` (spec.rs) → used by `SpecInfo.status` (queries.rs)
   - `LoadedSpec.status` and `FoundSpec.status` (archive.rs) are convenience
     copies from DB reads — they stay `Option<String>` since they come from
     SQLite TEXT columns
   - `status_map: HashMap<String, String>` in queries.rs needs to either
     stay String (DB-sourced) or convert at boundary
   - `MutationResult.new_status: String` is a display field — stays String

2. **The DB boundary is the tricky part.** `FoundSpec` reads status from
   SQLite as String. Converting to SpecStatus at the DB read boundary
   (ADR-4) is the right approach but adds a `from_str` conversion to
   `find_spec()` and `load_spec()`. This is the "parse at boundary"
   belief in action.

3. **Commit strategy should be: define enum first, migrate later.**
   Don't change the field type and fix all consumers in one commit.
   Instead: commit 1 defines SpecStatus (no field change), commit 2
   changes the field and fixes all consumers together. This way commit 1
   is zero-risk.

4. **This refactor touches the spec system itself** — the machinery that
   manages all other specs. Needs full context window and test coverage
   before pushing. Don't attempt in tail-end of a session.

## Commits

Phase 1 — three commits, smallest-to-largest blast radius:

1. `spec: define SpecStatus enum with serde rename, "done" alias, and FromStr` (zero-risk: no field change)
2. `spec: migrate SpecFrontmatter.status + mutations + DB boundary to SpecStatus` (write path: mutations.rs, archive.rs, split.rs)
3. `spec: migrate query structs, ListFilters, and blocker checks to SpecStatus` (read path: queries.rs, queue.rs, session/internal.rs, MCP filter)

Phase 2–3:

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
