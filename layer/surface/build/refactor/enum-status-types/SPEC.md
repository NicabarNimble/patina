---
type: refactor
id: enum-status-types
status: draft
created: 2026-03-01
sessions:
  origin: 20260301-165723
beliefs:
- enum-not-string-for-finite-states
- parse-at-boundary-type-the-interior
exit_criteria:
- id: spec-status-enum
  text: 'SpecFrontmatter.status is SpecStatus enum with 7 variants — all string comparisons replaced with match arms'
  checked: false
- id: belief-status-enum
  text: 'ParsedBelief.status and BeliefEntry.status are BeliefStatus enum with 4 variants'
  checked: false
- id: health-status-enum
  text: 'HealthCheck.status is HealthStatus enum with 3 variants'
  checked: false
- id: activity-level-enum
  text: 'ModuleSignal.activity_level is ActivityLevel enum with 4 variants'
  checked: false
- id: zero-status-string-comparisons
  text: 'rg ''== "draft"|== "ready"|== "active"|== "paused"|== "blocked"|== "complete"|== "abandoned"|== "defeated"|== "scoped"|== "archived"|== "healthy"|== "warning"|== "critical"|== "dormant"'' src/ returns zero (excluding serde rename attrs and test data)'
  checked: false
- id: serde-roundtrip
  text: 'YAML frontmatter (spec, belief) serializes/deserializes correctly with enum types — existing files parse without error'
  checked: false
- id: existing-tests-pass
  text: 'cargo test --workspace passes, pre-push checks pass'
  checked: false
---
# refactor: Replace status: String with typed enums

> 13 struct fields use `status: String` where finite-state enums should
> enforce valid transitions at compile time. 16+ string comparisons in
> spec mutations alone, each a typo waiting to happen.

## Current State

Four clusters of stringly-typed status fields:

**Spec status** (highest impact — 16+ comparison sites):
- `SpecFrontmatter.status: Option<String>` in `src/spec.rs:168`
- Values: draft, ready, active, paused, blocked, complete, abandoned
- Plus legacy "done" used as alias for "complete" in 5+ sites
- 8 structs carry this status through queries.rs, mutations.rs, queue.rs
- Every state transition is a string assignment: `status: "active".to_string()`

**Belief status** (data integrity):
- `ParsedBelief.status: String` in `src/commands/scrape/beliefs/mod.rs:33`
- `BeliefEntry.status: String` in `src/mother/graph.rs:154`
- Values: active, scoped, defeated, archived
- Stored in SQLite, round-trips through serde

**Doctor health** (small, self-contained):
- `HealthCheck.status: String` in `src/commands/doctor.rs:13`
- Values: healthy, warning, critical
- 3-4 comparison sites, all in one file

**Activity level** (trivial):
- `ModuleSignal.activity_level: String` in `src/commands/assay/internal/derive.rs:19`
- Values: high, medium, low, dormant
- 1 comparison site

Three clusters already use enums (reference patterns):
- `VerificationStatus` in `scrape/beliefs/verification/mod.rs`
- `VerbStatus` in `measure/internal.rs`
- `ChildHealth` in `mother/child.rs`

## Target State

Each status cluster becomes a `#[derive(Debug, Clone, Serialize, Deserialize)]`
enum with `#[serde(rename_all = "snake_case")]`. All string comparisons become
match arms. The compiler enforces exhaustive handling.

```rust
// src/spec.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Draft,
    Ready,
    Active,
    Paused,
    Blocked,
    Complete,
    Abandoned,
}
```

Legacy "done" alias: handled via `impl SpecStatus { fn is_terminal(&self) }` —
the 5+ sites checking `== "complete" || == "done"` become `status.is_terminal()`.

## Steps

### Phase 1: SpecStatus (highest value, most comparison sites)

1. Define `SpecStatus` enum in `src/spec.rs` with serde rename
2. Change `SpecFrontmatter.status` from `Option<String>` to `Option<SpecStatus>`
3. Add `impl SpecStatus` with `is_terminal()` (replaces complete/done checks),
   `is_active()`, `as_str()` for display
4. Update `mutations.rs` — all 7 state transitions become enum variants
5. Update `queries.rs` — all 16+ comparisons become match/equality checks
6. Update `queue.rs`, `archive.rs`, `split.rs`, `create.rs`
7. Update `session/internal.rs:1170` (blocker check)
8. Update MCP `SpecArgs.status` filter to use enum

### Phase 2: BeliefStatus

9. Define `BeliefStatus` enum in a shared location (used by scrape + mother)
10. Update `ParsedBelief.status` and `BeliefEntry.status`
11. Update mother/graph.rs database storage (TEXT column, serde round-trip)

### Phase 3: HealthStatus + ActivityLevel (small, mechanical)

12. Define `HealthStatus` enum in `doctor.rs`, update 3-4 comparison sites
13. Define `ActivityLevel` enum in `derive.rs`, update 1 comparison site

## Risks

1. **YAML round-trip breakage.** Spec files on disk have `status: active` as
   plain strings. `#[serde(rename_all = "snake_case")]` handles this — serde
   deserializes "active" → `SpecStatus::Active`. Test with existing spec files.

2. **"done" legacy alias.** 5+ sites check `== "done"` alongside `== "complete"`.
   If any spec files on disk have `status: done`, serde will reject them.
   Scan `layer/surface/` for actual usage before deciding: custom deserializer
   vs data migration.

3. **Database storage for beliefs.** BeliefEntry.status is stored as TEXT in
   SQLite. Enum serde round-trips cleanly (`"active"` → `BeliefStatus::Active`),
   but verify no raw SQL writes status strings outside the typed path.

## Non-Goals

- Changing session status (active/archived) — only 1-2 comparison sites, and
  session archival does string replacement in YAML text. Separate concern.
- Converting `entrenchment: String` (low/medium/high/very-high) — related but
  separate; scope creep.
- Adding state machine validation (e.g., preventing draft→complete skip) —
  that's a follow-up spec, not this one.

## Exit Criteria

- [x] SpecFrontmatter.status is SpecStatus enum with 7 variants
- [ ] ParsedBelief.status and BeliefEntry.status are BeliefStatus enum
- [ ] HealthCheck.status is HealthStatus enum with 3 variants
- [ ] ModuleSignal.activity_level is ActivityLevel enum with 4 variants
- [ ] Zero status string comparisons in src/ (excluding serde attrs and tests)
- [ ] YAML frontmatter round-trips correctly with enum types
- [ ] All tests pass, pre-push checks pass
