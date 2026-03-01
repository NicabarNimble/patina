---
type: refactor
id: enum-status-types
status: active
created: 2026-03-01
sessions:
  origin: 20260301-165723
beliefs:
- enum-not-string-for-finite-states
- parse-at-boundary-type-the-interior
exit_criteria:
- id: spec-status-enum
  text: SpecFrontmatter.status is SpecStatus enum with 7 variants — all string comparisons replaced with match arms
  checked: false
- id: belief-status-enum
  text: ParsedBelief.status and BeliefEntry.status are BeliefStatus enum with 4 variants
  checked: false
- id: health-status-enum
  text: HealthCheck.status is HealthStatus enum with 3 variants
  checked: false
- id: activity-level-enum
  text: ModuleSignal.activity_level is ActivityLevel enum with 4 variants
  checked: false
- id: zero-status-string-comparisons
  text: "Zero status string comparisons in control-flow code: blockers, queue logic, resume gating, session shutdown, state transitions. Output-only display fields (MutationResult.new_status, MutationDetail::Resume.previous_status) excluded — they mirror the enum for display only."
  checked: false
- id: list-filters-parse-enum
  text: "ListFilters.status parses user input into SpecStatus before evaluation. Unknown status values return a validation error, not a silent empty result."
  checked: false
- id: serde-roundtrip
  text: YAML frontmatter (spec, belief) serializes/deserializes correctly with enum types — existing files parse without error
  checked: false
- id: existing-tests-pass
  text: cargo test --workspace passes, pre-push checks pass
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
the 6 sites checking `== "complete" || == "done"` become `status.is_terminal()`.
No spec files on disk currently carry `status: done`; the `#[serde(alias = "done")]`
on `Complete` is defensive only and requires no data migration.

Known "done" comparison sites (6 total):
1. `queries.rs:145` — `get_ready_specs()` blocker resolution
2. `queries.rs:195` — `show_ready_specs()` unblocked display filter
3. `queries.rs:348` — `get_blocked_specs()` blocker resolution
4. `mutations.rs:720` — `resume_spec_value()` blocker check
5. `queue.rs:101` — `recommend_next()` blocker check
6. `session/internal.rs:1170` — session-end unblocked display

## Steps

### Phase 1: SpecStatus (highest value, most comparison sites)

Three commits, smallest-to-largest blast radius:

**Commit 1 — Define enum (zero-risk, no field change):**
1. Define `SpecStatus` enum in `src/spec.rs` with serde rename + `#[serde(alias = "done")]`
2. Add `impl SpecStatus` with `is_terminal()`, `as_str()`, `FromStr`

**Commit 2 — Write path (field change + mutations + DB boundary):**
3. Change `SpecFrontmatter.status` from `Option<String>` to `Option<SpecStatus>`
4. Update `mutations.rs` — all 7 state transitions become enum variants
5. Update `archive.rs` — `FoundSpec.status` and `LoadedSpec.status` become
   `Option<SpecStatus>`, parsed at DB boundary via `FromStr` (ADR-4)
6. Update `split.rs` — status checks use enum

**Commit 3 — Read/display path (query structs + filters):**
7. Update `queries.rs` — `SpecInfo.status`, `ReadySpec.status`, `Blocker.status`,
   `BlockedSpec.status` become `SpecStatus` (serde snake_case output unchanged)
8. Update `queue.rs`, `session/internal.rs:1170` (blocker checks)
9. Update `ListFilters.status` to `Option<SpecStatus>` — parse user input at CLI/MCP
   boundary, return validation error for unknown statuses
10. Update MCP `SpecArgs.status` filter to use enum

Output-only fields stay `String`: `MutationResult.new_status`,
`MutationDetail::Resume.previous_status` — they mirror the enum for display only.

### Phase 2: BeliefStatus

11. Define `BeliefStatus` enum in a shared location (used by scrape + mother)
12. Update `ParsedBelief.status` and `BeliefEntry.status`
13. Update mother/graph.rs database storage (TEXT column, serde round-trip)

### Phase 3: HealthStatus + ActivityLevel (small, mechanical)

14. Define `HealthStatus` enum in `doctor.rs`, update 3-4 comparison sites
15. Define `ActivityLevel` enum in `derive.rs`, update 1 comparison site

## Risks

1. **YAML round-trip breakage.** Spec files on disk have `status: active` as
   plain strings. `#[serde(rename_all = "snake_case")]` handles this — serde
   deserializes "active" → `SpecStatus::Active`. Test with existing spec files.

2. **"done" legacy alias.** 6 sites check `== "done"` alongside `== "complete"`.
   Grounded: zero spec files on disk carry `status: done` (verified via grep).
   `#[serde(alias = "done")]` on Complete is defensive for DB/tag data only.

3. **Database storage for beliefs.** BeliefEntry.status is stored as TEXT in
   SQLite. Enum serde round-trips cleanly (`"active"` → `BeliefStatus::Active`),
   but verify no raw SQL writes status strings outside the typed path.

## Non-Goals

- Changing session status (active/archived) — only 1-2 comparison sites, and
  session archival does string replacement in YAML text. Separate concern.
- Converting `entrenchment: String` (low/medium/high/very-high) — related but
  separate; scope creep.
- Converting `SpecMilestoneEntry.status: String` — milestone statuses are a
  different domain concept (version progress, not lifecycle state) and live in
  the same file. Explicitly out of scope to avoid confusion.
- Adding state machine validation (e.g., preventing draft→complete skip) —
  that's a follow-up spec, not this one.

## Exit Criteria

- [ ] SpecFrontmatter.status is SpecStatus enum with 7 variants
- [ ] ParsedBelief.status and BeliefEntry.status are BeliefStatus enum
- [ ] HealthCheck.status is HealthStatus enum with 3 variants
- [ ] ModuleSignal.activity_level is ActivityLevel enum with 4 variants
- [ ] Zero status string comparisons in control-flow code (6 "done" sites, ~46 total).
      Output-only display fields (MutationResult.new_status, etc.) excluded.
- [ ] ListFilters.status parses into SpecStatus; unknown values return validation error
- [ ] YAML frontmatter round-trips correctly with enum types
- [ ] All tests pass, pre-push checks pass
