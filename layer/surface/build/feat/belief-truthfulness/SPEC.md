---
type: feat
id: belief-truthfulness
status: active
created: 2026-02-15
sessions:
  origin: 20260215-083121
  amended: [20260216-064229, 20260216-073845]
related:
- layer/core/patina-identity.md
beliefs:
- beliefs-are-the-product
- belief-identity-is-slug-not-hash
- measure-the-measurement
---

# feat: Belief Truthfulness — Staleness Detection and Health Scoring

> Beliefs go stale. The audit command shows today's metrics but can't detect
> drift over time. Add staleness detection, health scoring, and a `--stale`
> flag to surface beliefs that need attention.

## Problem

100+ beliefs exist (129 as of 2026-02-16). `patina belief audit` computes per-belief metrics (citations,
evidence, verification, grounding) — but all metrics are point-in-time snapshots.
There is no mechanism to detect:

1. **Staleness** — a belief hasn't been cited, verified, or referenced in N months
2. **Drift** — a belief's verification queries used to pass but now fail
3. **Isolation** — a belief has no grounding connections and no citations (floating)
4. **Contradiction** — two beliefs with `attacks` relationships both active

The health_warnings system (`src/commands/belief/mod.rs:101-125`) catches some of
these today: `no-evidence`, `unverified`, `unused`, `no-applications`, `floating`,
`verify-contested`, `verify-error`. But it's all static — no temporal dimension.

## What Exists Today

### Belief audit (`src/commands/belief/mod.rs`)

`run_audit()` reads from the `beliefs` table, displays metrics in columnar format.
Sort modes: `use`, `truth`, `weak`, `grounding`. The `--warnings-only` flag filters
to beliefs with health warnings. The `--grounding` flag runs E4.6a semantic neighbor
search.

**`BeliefRow.health_warnings()`** returns static checks:
- `no-evidence` — evidence_count == 0
- `unverified` — evidence exists but none verified
- `unused` — no citations (by beliefs or sessions)
- `no-applications` — applied_in == 0
- `verify-contested` — verification queries failed
- `verify-error` — verification queries errored
- `floating` — no grounding connections

### Verification engine (`src/commands/scrape/beliefs/verification/`)

Full infrastructure: `VerificationQuery` (sql, assay, temporal types),
`VerificationResult` (pass/contested/error), `VerificationAggregates`.
Stored in `belief_verifications` table with data_freshness tracking.

Verification types:
- **SQL** — direct database queries (`internal/exec.rs`)
- **Assay** — structural queries via assay engine (`internal/assay.rs`)
- **Temporal** — co-change queries (`internal/temporal.rs`)
- **Safety** — query sandboxing, no writes allowed (`internal/safety.rs`)

### Belief metrics (`src/commands/scrape/beliefs/mod.rs:44-67`)

`BeliefMetrics` struct computes:
- Use: `cited_by_beliefs`, `cited_by_sessions`, `applied_in`
- Truth: `evidence_count`, `evidence_verified`, `defeated_attacks`, `external_sources`
- Grounding: `grounding_score`, `grounding_code_count`, `grounding_commit_count`,
  `grounding_session_count`, `grounding_forge_count`

### Belief schema (`src/commands/scrape/beliefs/mod.rs:70-100`)

```sql
beliefs (
    id, statement, persona, facets, confidence, entrenchment, status,
    extracted, revised, file_path,
    -- E4 metrics
    cited_by_beliefs, cited_by_sessions, applied_in,
    evidence_count, evidence_verified, defeated_attacks, external_sources, endorsed,
    -- E4.6a grounding
    grounding_score, grounding_code_count, grounding_commit_count,
    grounding_session_count, grounding_forge_count
)
```

## What To Build

### Phase A: Staleness Detection in Scrape

Add temporal awareness to belief metrics during scrape:

1. **Last-activity tracking** — four nullable component columns on the beliefs
   table, plus a computed `last_activity` column as their MAX.

   **All dates stored as ISO 8601 `YYYY-MM-DD` TEXT.** This ensures MAX()
   comparison works across all sources. Sources that provide higher precision
   (timestamps, RFC 3339) are truncated to date-only. **All timestamp-to-date
   conversions use `chrono::Utc`** (consistent with `exec.rs:299` and codebase
   convention). This avoids off-by-one date issues near UTC midnight when
   local timezone differs from UTC.

   - `last_file_touch TEXT` — `std::fs::metadata(path).modified()` during
     `parse_belief_file()`, converted to `YYYY-MM-DD` (free — already reading file).
     **Caveat:** file mtime resets on `git clone` and may be touched by formatters
     or tooling. This is the weakest signal — treat as fallback when other signals
     are NULL. `last_frontmatter_revision` is the authoritative content-change signal.
   - `last_frontmatter_revision TEXT` — from `revised` YAML field (already `YYYY-MM-DD`
     at `scrape/beliefs/mod.rs:257-263`)
   - `last_session_citation TEXT` — during `cross_reference_beliefs()` (line 528-541),
     the code already iterates session files checking `contains(bid)`. Extract date
     from filename using regex `^(\d{4})(\d{2})(\d{2})-\d{6}` → `YYYY-MM-DD`.
     Track MAX per belief. Zero extra I/O, just a string parse in the existing loop.
     **Non-matching filenames** (e.g., `session_summary_july27.md`, `*-init.md`
     suffixed files) are silently skipped for date extraction — they still count
     as citations in `cited_by_sessions` but don't contribute to the timestamp.
   - `last_verification_run TEXT` — set to today's date (`YYYY-MM-DD`) only for
     beliefs that HAVE verification queries (i.e., `verification_total > 0`).
     Beliefs with no `## Verification` section get NULL — they have no verification
     signal. This is per-belief freshness, not per-scrape.
   - `last_activity TEXT` — computed after all signals collected. Algorithm:
     ```
     strong = MAX(last_frontmatter_revision, last_session_citation, last_verification_run)
     last_activity = strong ?? last_file_touch ?? NULL
     ```
     `last_file_touch` is a **fallback only** — it participates in `last_activity`
     exclusively when ALL THREE strong signals are NULL. This prevents file mtime
     (reset by `git clone`, touched by formatters) from masking genuine staleness.
     After a fresh clone, beliefs without recent revisions, citations, or
     verification will correctly appear stale — `last_file_touch = today` does
     NOT override the absence of real activity signals.

     If all four component signals are NULL, `last_activity` is NULL (belief has
     no temporal signal — will appear stale).

   Storing pre-fallback components keeps audit output explainable (future
   `--show-activity` can display why a belief is stale) and avoids re-deriving
   during display.

2. **Staleness threshold** — belief is "stale" if last_activity > N days ago.
   Configurable via `.patina/config.toml` `[beliefs] stale_days = 90`.

   **Prerequisite:** `ProjectConfig` (`src/project/internal.rs:18-38`) has no
   `BeliefsSection`. Add one:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct BeliefsSection {
       #[serde(default = "default_stale_days")]
       pub stale_days: u32,
   }
   fn default_stale_days() -> u32 { 90 }
   ```
   Add `#[serde(default)] pub beliefs: BeliefsSection` to `ProjectConfig`.
   Re-export from `project/mod.rs`. Existing config.toml files without a
   `[beliefs]` section will deserialize with the default (90) — no migration
   needed. `patina init` templates should include the section for discoverability.

   **Config access in scrape:** In `run()` (`beliefs/mod.rs`), load config
   via `crate::project::load(Path::new("."))` at the top of the function.
   Extract `stale_days` for freshness computation and health score. If config
   load fails (e.g., no `.patina/` directory), fall back to `BeliefsSection`
   default (90 days).

   **Config access in audit:** In `run_audit()` (`belief/mod.rs`), load
   config via the same `crate::project::load(Path::new("."))` path. Use
   `stale_days` for the `--stale` filter threshold. Each command loads
   config independently — no coupling between scrape and audit.

   Audit summary emits freshness stats: "32/128 beliefs stale (>90d),
   median activity age 143d" for immediate signal without hardcoding
   policy. Median computed over beliefs with non-NULL `last_activity`
   only — NULL-activity beliefs are excluded from the median (their age
   is unknown, not infinite) but counted in the stale total (since NULL
   last_activity → stale per the threshold definition).

3. **Verification drift** — snapshot-before-drop approach.

   **Constraint:** `create_tables()` (`verification/internal/exec.rs:257`) does
   `DROP TABLE IF EXISTS belief_verifications` on every scrape. There are no
   "previous results" to compare against — `data_freshness` is just the string
   "full" or "incremental", not a temporal marker.

   **Implementation:** At the start of `create_tables()`:
   ```sql
   -- Crash recovery: drop stale _prev from a previous crashed scrape
   DROP TABLE IF EXISTS belief_verifications_prev;
   -- Guard: skip on first-ever scrape (table doesn't exist yet)
   ALTER TABLE belief_verifications RENAME TO belief_verifications_prev;
   ```
   If the RENAME fails (first scrape, no prior table), skip drift detection
   entirely — there's nothing to compare against. After the new verification
   run completes, diff (within the same SQLite connection — no concurrent
   access risk since SQLite serializes writers):
   ```sql
   SELECT p.belief_id, p.label
   FROM belief_verifications_prev p
   JOIN belief_verifications c ON p.belief_id = c.belief_id AND p.label = c.label
   WHERE p.last_status = 'pass' AND c.last_status != 'pass'
   ```
   Collect drifted belief IDs into a `Vec<String>`. The drift result is applied
   via **post-insert UPDATE** (Phase 3b), not via `insert_belief()`. This
   decouples drift detection from the insert pipeline and works identically
   for both full and incremental scrapes.

   **Phase 3b: Apply drift flags** (runs in `run()` after Phase 3 inserts):
   ```sql
   UPDATE beliefs SET verification_drifted = 0;  -- reset all
   UPDATE beliefs SET verification_drifted = 1
     WHERE id IN (SELECT DISTINCT p.belief_id
       FROM belief_verifications_prev p
       JOIN belief_verifications c ON p.belief_id = c.belief_id AND p.label = c.label
       WHERE p.last_status = 'pass' AND c.last_status != 'pass');
   DROP TABLE IF EXISTS belief_verifications_prev;
   ```
   The UPDATE-all-then-set pattern works for both full scrape (beliefs just
   inserted with DEFAULT 0, then flagged) and incremental scrape (skipped
   beliefs reset to 0, then re-flagged if still drifted). No modification
   to `ParsedBelief` or `BeliefMetrics` needed — drift is a DB-only concern.
   The `verification_drifted` column is NOT passed through `insert_belief()`;
   it uses `DEFAULT 0` on INSERT and is set exclusively by Phase 3b UPDATE.

   **Crash recovery:** A crash between RENAME and verification completion
   loses one cycle's drift comparison. This is acceptable — drift is an
   informational signal, not a constraint, and the next successful scrape
   restores the comparison baseline. The `DROP IF EXISTS _prev` at the
   start of `create_tables()` cleans up stale _prev tables from incomplete
   runs.

   **Scope of comparison:** The INNER JOIN on `(belief_id, label)` compares
   only queries present in both the previous and current runs. Deleted or
   renamed queries are not drift — they are editorial changes by the belief
   author. Only queries that existed before AND still exist now participate
   in pass→non-pass comparison. Similarly, newly added queries have no
   previous baseline and are excluded.

   This delivers drift detection with minimal pipeline change. An append-only
   history table is deferred until there's proven need for trend analysis.

   New column: `verification_drifted INTEGER DEFAULT 0` on beliefs table.

**Phase 3a: Push temporal updates for skipped beliefs** (incremental only).
During incremental scrape, Phase 3 skips `insert_belief()` for already-known
beliefs. But temporal signals change even when the belief file doesn't — a
new session may cite it, or verification results may differ. To keep staleness
signals current, run targeted UPDATEs for skipped beliefs:

```rust
// For each belief skipped by Phase 3 (already in DB):
UPDATE beliefs SET
  last_file_touch = ?1,
  last_frontmatter_revision = ?2,
  last_session_citation = ?3,
  last_verification_run = ?4,
  last_activity = ?5,
  health_score = ?6,
  contested_by = ?7
WHERE id = ?8
```

This follows the existing pattern at `beliefs/mod.rs:1068-1081` where
verification aggregates are pushed via direct UPDATE for skipped beliefs.
Health score must be computed BEFORE Phase 3 so both insert and update
paths use the same values.

**Code paths:**
- `src/commands/scrape/beliefs/mod.rs` — extend `BeliefMetrics` with four
  `last_*` fields plus `last_activity`. Add columns to
  `create_materialized_views()`. Populate `last_file_touch` in
  `parse_belief_file()`, `last_session_citation` in `cross_reference_beliefs()`,
  `last_frontmatter_revision` from existing `revised` field. Compute
  `health_score` before Phase 3. Phase 3a pushes temporal fields + health_score
  + contested_by for skipped beliefs. Phase 3b applies drift UPDATE.
- `src/commands/scrape/beliefs/verification/internal/exec.rs` — in
  `create_tables()`, rename before drop (snapshot for drift comparison).

### Phase B: Health Score

**Depends on Phase A** — requires `last_activity` and `stale_days` config.

Compute a single 0.0-1.0 health score per belief from the existing metrics:

```
health = w_use * use_score + w_truth * truth_score + w_fresh * freshness_score

where:
  use_score    = min(1.0, (cited_by_beliefs + cited_by_sessions) / 3)
  truth_score  = evidence_verified / max(1, evidence_count)
  freshness    = 1.0 - min(1.0, days_since_activity / stale_days)
```

Weights: `w_use = 0.3, w_truth = 0.4, w_fresh = 0.3` — hardcoded constants in
`src/commands/scrape/beliefs/mod.rs`. Move to `[beliefs]` config fields only
when real score distributions motivate tuning. Until then, changing weights
means editing code — intentionally raising the bar for premature optimization.
Linear freshness curve — don't optimize the math until real distributions are
observed.

**NULL last_activity:** If all four activity signals are NULL, freshness = 0.0
(maximally stale). Health score still computes from use + truth dimensions.

**Zero-evidence beliefs:** `truth_score = 0 / max(1, 0) = 0.0`. With weights
0.3 + 0.4(0) + 0.3, max possible score is 0.6 (perfect use + freshness, no
truth). **This is intentional** — beliefs without evidence are hypotheses and
SHOULD score lower. The `no-evidence` warning already flags these; the health
score reinforces it quantitatively. If this proves too punitive in practice,
adjust `w_truth` in the hardcoded constants — don't add special-case logic.
Moving weights to config is a future decision gated on observed distributions.

Store as `health_score REAL` column on beliefs table. Compute during scrape,
expose via:
- `--sort health` sort mode in belief audit — **ascending order** (lowest
  health first), matching `--sort weak` convention. Users want to see the
  beliefs needing attention at the top.
- `--stale` flag filters beliefs where `last_activity` exceeds `stale_days`
  (the simple, honest definition — matches Phase A's staleness threshold exactly).
  This is NOT tied to the freshness component of health_score. Composable with
  `--warnings-only` via AND: `--stale --warnings-only` shows stale beliefs that
  also have other warnings.
- `low-health` warning in `health_warnings()` when health_score < 0.4
- `verify-drifted` warning in `health_warnings()` when `verification_drifted = 1`.
  This distinguishes "previously passing, now failing" (drift) from "always
  failing" (`verify-contested`). Both may fire for the same belief — they are
  not mutually exclusive.

Compute and expose only — health score does not gate any automated action.
Collect feedback from real audit usage before refining weights or adding
nonlinear curves.

**Code path:** `src/commands/belief/mod.rs` — add `health` sort mode (ascending),
`--stale` filter, `low-health` warning, `verify-drifted` warning. Read
`verification_drifted` and `health_score` columns in `BeliefRow` (with
`has_*` column-existence checks for graceful degradation on pre-scrape DBs).
`src/commands/scrape/beliefs/mod.rs` — compute health_score before Phase 3
so both insert and Phase 3a update paths use the same value.

### Phase C: Contradiction Detection

Detect when two active beliefs have `attacks` relationships and both are active:

1. During belief scrape, `extract_file_metrics()` (`scrape/beliefs/mod.rs:323-361`)
   already parses `## Attacked-By` sections and counts defeated attacks. Extend
   it to also collect non-defeated attacker IDs into a new `BeliefMetrics` field:
   `attacked_by_ids: Vec<String>`.

   **Attacked-By entry formats** (from actual belief files):
   - Structured: `- [[belief-id]] (status: active, confidence: 0.3, scope: "...")`
   - Unstructured: `- plain text description` (no wikilink — skip these)

   Extract `[[belief-id]]` via the existing `\[\[([^\]]+)\]\]` regex pattern
   (already used in `verify_evidence_section()` at line 368). Only collect IDs
   from entries that do NOT contain `status: defeated`.

2. Also parse `## Attacks` sections (same format as `## Attacked-By`, already
   present in belief files — see `sync-first.md:53-56`). Collect non-defeated
   target IDs into `attacks_ids: Vec<String>`.

3. In `cross_reference_beliefs()`, after all beliefs are parsed, build the
   bidirectional contest map: if belief A's `## Attacks` lists B (non-defeated)
   AND B's `ParsedBelief.status == "active"`, then A contests B. Symmetrically,
   if B's `## Attacked-By` lists A (non-defeated) AND A's
   `ParsedBelief.status == "active"`, then B is contested by A. Merge both
   directions — both A and B get flagged.

4. **Data flow:** Add `contested_by: Vec<String>` to `BeliefMetrics`. In
   `cross_reference_beliefs()`, after building the bidirectional contest map,
   populate each belief's `metrics.contested_by` with the IDs of active beliefs
   that contest it. In `insert_belief()`, serialize as
   `metrics.contested_by.join(",")` for the `contested_by TEXT` column (empty
   string if no contests). This follows the existing pattern: metrics are
   computed during cross-reference, stored in `BeliefMetrics`, serialized
   during insert.

5. New column `contested_by TEXT` on beliefs table (comma-separated belief IDs).
   **Escaping:** belief IDs are kebab-case slugs (e.g., `sync-first`) — commas
   never appear in IDs, so comma separation is safe. New warning in
   `health_warnings()`: `contested-by:{other-belief-id}`, read from this
   column at display time. Uses the existing ALTER TABLE migration pattern
   (`scrape/beliefs/mod.rs:131-152`).

**Code path:** `src/commands/scrape/beliefs/mod.rs` — add `attacked_by_ids`,
`attacks_ids`, and `contested_by` fields to `BeliefMetrics`. Extend
`extract_file_metrics()` to parse both `## Attacked-By` (collect attacker IDs)
and `## Attacks` (collect target IDs). Build bidirectional contest map and
populate `metrics.contested_by` in `cross_reference_beliefs()`. Serialize
in `insert_belief()`. `src/commands/belief/mod.rs` — add `contested-by:`
warning to `health_warnings()`, reading from `contested_by` column.

## Exit Criteria

1. `patina belief audit --stale` shows beliefs with no activity in 90+ days
2. `patina belief audit --sort health` ranks beliefs by computed health score
3. Verification drift is detected and surfaced as `verify-drifted` warning
4. Active attack pairs flagged in audit warnings

## Expected Behavior on First Use

**Staleness (`--stale`):** On a young project where all beliefs were created
within `stale_days` (default 90), `--stale` returns no results. This is
correct — no beliefs ARE stale yet. The flag becomes useful as the project
ages and beliefs stop being revised or cited. Mention this in `patina version`
release notes so users don't think the flag is broken.

**Drift (Phase A.3):** First scrape has no baseline (`_prev` table). Drift
detection starts producing results on the second scrape when a comparison
baseline exists.

**Contests (Phase C):** Contest detection only fires when beliefs have
populated `## Attacks` or `## Attacked-By` sections with `[[wikilink]]`
references to other active beliefs. Empty sections (the current default)
produce zero results.

## Verification Plan

### Migration safety
- All 8 new columns use the existing `ALTER TABLE ... ADD COLUMN` ignore-if-exists
  pattern at `scrape/beliefs/mod.rs:131-152`. No destructive migration. New
  columns must ALSO appear in the `CREATE TABLE IF NOT EXISTS` statement at
  `scrape/beliefs/mod.rs:74-100` so fresh databases get them without needing
  the ALTER TABLE path.
- `insert_belief()` (`scrape/beliefs/mod.rs:862-967`) must be updated to include
  7 new columns in the INSERT statement — `last_file_touch`, `last_frontmatter_revision`,
  `last_session_citation`, `last_verification_run`, `last_activity`, `health_score`,
  `contested_by`. The 8th column (`verification_drifted`) uses `DEFAULT 0` on INSERT
  and is set exclusively by Phase 3b UPDATE — it is NOT passed through `insert_belief()`.
- Existing `patina belief audit` must not break when new columns are NULL
  (pre-scrape state). All new `BeliefRow` fields default to 0/empty/NULL.

### Unit tests (add to `scrape/beliefs/mod.rs::tests`)
- `test_last_activity_max` — MAX of 4 signals, NULL handling, all-NULL → NULL
- `test_session_filename_parsing` — `20260215-230959.md` → `2026-02-15`,
  `session_summary_july27.md` → skip, `20250812-123323-init.md` → `2025-08-12`
- `test_health_score_computation` — zero-evidence (max 0.6), all-healthy (near 1.0),
  all-stale (freshness 0.0), NULL last_activity (freshness 0.0)
- `test_verification_drift_detection` — pass→contested flags drift, pass→pass no flag,
  no _prev table → skip, error→pass → no flag (only pass→non-pass counts),
  deleted query (in _prev but not current) → no flag (editorial, not drift)
- `test_attacked_by_parsing` — extract `[[id]]` from structured entries, skip
  unstructured entries, skip `status: defeated` entries
- `test_contested_bidirectional` — A attacks B, B attacked-by A, both active → both flagged

### Integration test
- Full scrape + audit cycle: `cargo build --release && cargo install --path . &&
  patina scrape --rebuild && patina belief audit --sort health --stale`
- Verify: no panics, new columns populated, summary stats line present,
  `--stale` filter works, `--sort health` orders correctly

## Non-Goals

- Automated belief archival (user decides what to do with stale beliefs)
- LLM-based belief challenge generation
- Belief promotion/demotion automation
- Cross-project staleness (that's cross-project-beliefs spec territory)

## Evidence

| Claim | Source |
|-------|--------|
| 100+ beliefs (129 as of 2026-02-16), all static metrics | `patina belief audit` output |
| health_warnings has 7 static checks | `src/commands/belief/mod.rs:101-125` |
| BeliefMetrics has no temporal fields | `src/commands/scrape/beliefs/mod.rs:44-67` |
| beliefs table has no last_activity | `src/commands/scrape/beliefs/mod.rs:70-100` |
| Verification types: sql, assay, temporal | `src/commands/scrape/beliefs/verification/internal/` |
| **belief_verifications is DROP+CREATE every scrape** | `verification/internal/exec.rs:257-258` |
| data_freshness is "full"/"incremental", not temporal | `scrape/beliefs/mod.rs:1047` + `exec.rs:299` |
| `revised` frontmatter already parsed | `scrape/beliefs/mod.rs:257-263` |
| cross_reference_beliefs reads all session files | `scrape/beliefs/mod.rs:528-541` |
| Session filenames are timestamps (YYYYMMDD-HHMMSS.md) | `layer/sessions/` directory convention |
| extract_file_metrics parses Attacked-By for defeated count | `scrape/beliefs/mod.rs:350-354` |
| Attacked-By uses `[[id]] (status: active/defeated)` format | `layer/surface/epistemic/beliefs/sync-first.md:60` |
| ProjectConfig has no BeliefsSection for stale_days | `src/project/internal.rs:18-38` |
| Wikilink regex already exists in verify_evidence_section | `scrape/beliefs/mod.rs:368` |
| `## Attacks` sections exist with `[[id]]` entries | `layer/surface/epistemic/beliefs/sync-first.md:53-56` |
| Non-standard session filenames exist (`*-init.md`, freeform) | `layer/sessions/` (20+ non-standard files) |
| Belief IDs are kebab-case slugs (no commas) | `layer/surface/epistemic/beliefs/` naming convention |
| SQLite serializes writers (no concurrent access risk) | SQLite WAL mode documentation |
| File mtime resets on git clone | Git behavior (does not preserve mtime) |

### Amendment History

- **2026-02-16** (session 20260216-064229): Deep code read revealed belief_verifications
  is DROP+CREATE every scrape — original drift detection approach was impossible.
  Amended Phase A with snapshot-before-drop strategy. Added 4 component columns for
  last_activity explainability. Fixed Phase C to reuse existing parsing infrastructure
  (reuses parsing, adds one `contested_by TEXT` column). Added `low-health` warning
  to Phase B. Grounded all claims against actual line numbers. UX review confirmed
  terminal width constraint: show `last_activity` in main table, component columns
  behind future `--verbose` flag only. Second pass: normalized all dates to
  ISO 8601 `YYYY-MM-DD`, added BeliefsSection config prerequisite, first-scrape
  guard for RENAME, NULL last_activity → freshness 0.0, Attacked-By parsing
  format documented from real belief files, Phase B explicit dependency on A.
  Third pass (10-concern review): aligned --stale with stale_days (not freshness
  component), documented last_file_touch as weakest signal (git clone resets mtime),
  specified session filename regex with skip-on-mismatch for non-standard names,
  clarified last_verification_run is per-belief (NULL when no queries), added crash
  recovery (DROP _prev on startup) and SQLite writer serialization note, defined
  verification_drifted reset semantics (cleared each scrape, only pass→non-pass),
  noted serde default handles missing config section, called out zero-evidence
  health cap (0.6 max) as intentional policy, added bidirectional contest detection
  via ## Attacks parsing, documented comma-safety of kebab-case IDs, added full
  verification plan with 6 unit tests and integration test.
- **2026-02-16** (session 20260216-073845): Fourth pass (11-concern review from
  outside agent + code audit). Resolved drift detection data flow: post-insert
  UPDATE approach (Phase 3b) decouples drift from insert pipeline,
  verification_drifted is DB-only (DEFAULT 0 on INSERT, set by UPDATE), 7 new
  insert_belief() params not 8. Crash recovery: acknowledged as acceptable
  one-cycle signal loss. INNER JOIN scope: deleted/renamed queries are editorial
  not drift. Health weights: hardcoded constants, configurable only when real
  distributions motivate it. Config plumbing: both scrape run() and audit
  run_audit() load ProjectConfig independently via crate::project::load().
  Phase C data flow: contested_by: Vec<String> on BeliefMetrics, serialized in
  insert_belief(), follows existing cross-reference → insert pattern. Timezone:
  mandated chrono::Utc for all timestamp-to-date conversions. Median age:
  computed over non-NULL last_activity beliefs only, NULL excluded from median
  but counted in stale total. CREATE TABLE: new columns must appear in both
  CREATE TABLE and ALTER TABLE paths. Belief count: aligned problem statement
  and evidence table to dated snapshot (129 as of 2026-02-16).
