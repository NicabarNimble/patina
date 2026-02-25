---
type: explore
id: spec-system-audit-2026-02
status: draft
created: 2026-02-25
sessions:
  origin: 20260225-104204
related:
- spec-next-typed
exit_criteria: []
---
# explore: Spec System Audit — Full Expert Review Findings

> Comprehensive audit of the spec system through Gjengset, Ng, Sutton, and Yegge lenses — captures all findings beyond the top 4 actionable specs

## Question

What are the architectural strengths, weaknesses, and improvement opportunities in Patina's spec system (`src/commands/spec/`) as evaluated through four expert lenses?

**Methodology:** Full code review of all 6 internal modules (~1,400 lines), applied through Jon Gjengset (Rust systems/type safety), Andrew Ng (ML/measurement), Rich Sutton (simplicity/Bitter Lesson), and Steve Yegge (platform/DX) perspectives.

## Findings

### Audit Scope

| Module | Lines | Role |
|--------|-------|------|
| `mod.rs` | ~500 | CLI enum (15 subcommands) + public API delegation |
| `mutations.rs` | ~876 | Core state machine: promote, complete, abandon, pause, resume, block, set |
| `queries.rs` | ~835 | Read operations: check, show, ready, blocked, list + data types |
| `queue.rs` | ~240 | Recommendation engine: next, age computation, dep counts |
| `create.rs` | ~277 | Spec creation: validation, scaffolding, session linking |
| `archive.rs` | ~435 | Archive lifecycle: tag, remove, release integration, find/load |
| `split.rs` | ~152 | Split operation: complete original + draft remainder |

### Top 4 Findings → Separate Specs

These were promoted to their own actionable specs:

1. **[[spec-next-typed]]** (fix) — `next_spec_value()` returns untyped `serde_json::Value` while every other `_value()` function returns typed structs. Gjengset: compiler can't catch schema drift. Sutton: inconsistency in an otherwise clean pattern.

2. **[[spec-history]]** (feat) — No CLI command to view a spec's lifecycle events despite rich git tag history. Ng: can't measure what you can't see. Yegge: platform has the data but doesn't surface it.

3. **[[spec-query-filesystem-truth]]** (refactor) — `get_ready_specs`/`get_blocked_specs` query DB directly while `get_all_specs` uses filesystem truth. Sutton: two representations of the same state will diverge. Gjengset: two code paths that should agree but can't be compiler-verified. Ng: DB freshness is unchecked.

4. **[[spec-scan-efficiency]]** (refactor) — `spec_age_days_from_list` re-reads files per spec in loops, `show_ready_specs` triggers multiple directory walks, `find_spec` walks the tree twice on fallback. Gjengset: parse once, pass the data. Sutton: simplest approach is one scan.

---

### Remaining Findings by Expert

#### Jon Gjengset — Rust Systems & Type Safety

**F-G1: `MutationDetail` uses `#[serde(untagged)]`** (mutations.rs:39)

Untagged enums lose the variant discriminator during deserialization. The `command` field on `MutationResult` acts as a manual discriminator, but the compiler can't enforce the pairing between `command: "promote"` and `MutationDetail::Promote`. This is fine for write-only CLI output but becomes a bug magnet if anything ever round-trips this JSON.

**Severity:** Low — currently write-only. Would become high if MCP clients start deserializing responses.

**Recommendation:** Consider `#[serde(tag = "type")]` (internally tagged) if/when bidirectional serialization is needed. No action required now.

---

**F-G2: `archive_spec_inner` shells out `Command::new("git")` directly** (archive.rs:94)

Every other git operation in the spec module uses `patina::git` helpers. This one function bypasses that abstraction for `git rm -rf`. The comment says "single call site, not worth abstracting" — but the function is destructive (`rm -rf`) and the inconsistency means this path isn't covered by any mocking or abstraction that the rest of the system gets.

**Severity:** Medium — correctness risk is low (the command is straightforward), but it's the only place the spec module breaks its own pattern.

**Recommendation:** Add `patina::git::remove_paths(&[path])` helper. One call site today, but `archive_spec_inner` is called from 3 places (archive, complete, abandon).

---

**F-G3: No `LoadedSpec` ↔ `FoundSpec` unification** (archive.rs)

`find_spec()` returns `FoundSpec` (file_path, status, title). Then `load_spec()` calls `find_spec()`, re-reads the file, and returns `LoadedSpec` (file_path, status, title, content, frontmatter, body). These are conceptually a two-tier load, but implemented as separate structs with overlapping fields. The `FoundSpec` → `LoadedSpec` pipeline would be cleaner as a builder or associated method (`FoundSpec::load() -> LoadedSpec`).

**Severity:** Low — structural cleanliness, not a bug.

**Recommendation:** Consider `impl FoundSpec { fn load(self) -> Result<LoadedSpec> }` in a future cleanup pass.

---

#### Andrew Ng — Measurement & ML Systems

**F-N1: No spec lifecycle analytics**

The state transitions are all tracked via git tags, but there's no command or report that answers:
- What's the average draft→complete cycle time?
- What's the abandon rate?
- How long do specs spend paused vs active?
- Which spec types (feat/fix/refactor) move fastest?

You have enough historical data (100+ completed specs via archive tags) to compute these. The `spec history` command ([[spec-history]]) surfaces per-spec data; a separate `spec stats` or `spec report` would aggregate across all specs.

**Severity:** Medium — you're driving blind on process effectiveness.

**Recommendation:** After [[spec-history]] ships, add aggregate reporting as a follow-up feat spec. The tag data is already there.

---

**F-N2: `next` ranking is hand-tuned, not learned**

The priority weights in `next_spec_value()` (queue.rs:99-168) are hardcoded: active=1, blocked-ready=2, paused-overdue=3, paused=4, ready=5, draft=6. With 158 beliefs and 100+ sessions, you have enough data to validate whether these weights actually predict what the user works on next.

Even a simple "did the user follow the recommendation?" signal — comparing `next_spec_value()` output at session start with which spec was actually worked on — would close the feedback loop.

**Severity:** Low — the current weights are reasonable heuristics. This is an optimization opportunity, not a bug.

**Recommendation:** Log `next` recommendations to session metadata. Compute hit rate as part of spec lifecycle analytics.

---

**F-N3: Exit criteria are boolean checkboxes, not structured assertions**

`check_spec_value()` reads `c.checked` from YAML — a human manually toggles these. There's no automated validation (e.g., "tests pass", "no compiler warnings", "scrape succeeds after changes"). The gate exists but the checking is entirely manual.

**Severity:** Low-Medium — the manual gate is still valuable (it forces conscious acknowledgment), but automated checks would catch the "checked the box but didn't actually verify" failure mode.

**Recommendation:** Consider an `exit_criteria` type field: `manual` (current behavior) vs `command` (runs a shell command and checks exit code). Not worth a spec yet — this is a future enhancement if manual checking proves insufficient.

---

#### Rich Sutton — Simplicity & The Bitter Lesson

**F-S1: 15 subcommands is high surface area**

`create`, `list`, `show`, `promote`, `complete`, `abandon`, `pause`, `resume`, `block`, `split`, `ready`, `blocked`, `next`, `check`, `set`, `archive` — that's 15 separate code paths for what is essentially a state machine with 6 states.

Some of these could collapse:
- `ready`/`blocked`/`list` are three ways to query the same data with different filters
- `next` is `ready` with ranking
- `check` could be a flag on `show` (`--check-exit`)
- `archive` is largely subsumed by `complete` (standalone archive only exists for retroactive cleanup)

**Severity:** Low — the commands are individually correct and well-tested. This is maintenance burden, not correctness.

**Recommendation:** No immediate action. If the spec system grows further, consider collapsing query commands into `list` with smart defaults (`patina spec list --ready`, `patina spec list --blocked`).

---

**F-S2: The `_value()` / display function duplication**

Every operation has two versions: `promote_spec()` (CLI) and `promote_spec_value()` (MCP). The CLI version calls `_value` then formats output. This is the correct architecture, but it means every new feature is implemented once and formatted twice. The formatting functions are largely boilerplate (check json flag → serialize or print).

**Severity:** Low — the pattern is clean and consistent. Maintenance cost is linear with number of commands.

**Recommendation:** A generic `fn emit<T: Serialize>(result: T, json: bool, human_format: impl Fn(&T))` could eliminate the json-check boilerplate. Not worth a spec — do it opportunistically if adding new commands.

---

#### Steve Yegge — Platform Thinking & Developer Experience

**F-Y1: No `patina spec diff <id>` or `patina spec log <id>`**

Beyond the `history` command (lifecycle events), there's no way to see the content evolution of a spec through the CLI. Users can run `git log -- layer/surface/build/*/my-spec/SPEC.md` manually, but the platform doesn't surface it.

**Severity:** Low — git is available and users know it. This is a convenience gap, not a capability gap.

**Recommendation:** Defer unless `spec history` proves popular. The git tag convention makes this straightforward to add later.

---

**F-Y2: `set` only supports 4 fields**

`set_spec_value()` has an explicit allowlist: `beliefs`, `related`, `references`, `target`. Adding a new settable field requires editing both the allowlist and potentially `SpecFrontmatter`. This is fine for a controlled system (you don't want arbitrary frontmatter mutation) but the extension path isn't documented.

**Severity:** Low — the constraint is intentional and correct. 4 fields covers current needs.

**Recommendation:** Add a comment in the code documenting how to add a new settable field (update `VEC_FIELDS`/`SCALAR_FIELDS` + ensure `SpecFrontmatter` has the field).

---

**F-Y3: `archive` is a separate explicit step from `complete`**

`complete` already calls `release_and_archive()` internally. The standalone `archive` command exists for specs completed before the integrated flow was built, and `archive --stale` is the batch cleanup. This is two paths to the same end state.

**Severity:** Low — `archive` is still useful for edge cases (manually completed specs, abandoned specs that weren't archived). The `--stale` batch mode is genuinely valuable.

**Recommendation:** No action. The standalone `archive` may look redundant but serves real edge cases. Document that `complete` is the normal path and `archive` is the retroactive path.

---

**F-Y4: `create` requires type as positional arg**

`patina spec create feat my-feature` works for power users but is discoverable-hostile. `patina spec create my-feature --type feat` would be more conventional and allow defaulting the type (e.g., `feat` as most common).

**Severity:** Low — CLI convention preference, not a bug. Power users (the primary audience) know the types.

**Recommendation:** No action unless onboarding new users to the spec system. The current UX matches Patina's "power tool" identity.

---

### Strengths (Unanimous)

All four perspectives agreed on these as **strong design choices**:

1. **`mutate_spec` closure pattern** — single mutation point with rollback. Clean, testable, correct.
2. **Filesystem-as-truth for spec existence** — git-versioned markdown files are the most durable representation.
3. **Git tags as lifecycle event log** — immutable, cheap, already in the natural workflow.
4. **One-paused-spec rule** — simple capacity constraint that prevents WIP explosion.
5. **`--json` on every command** — makes the spec system a programmable platform, not just a CLI.
6. **`_value()` pattern** — business logic returns data, presentation is a separate concern.
7. **Exit criteria gate on `complete`** — quality feedback loop with `--force` escape hatch.
8. **`split` as first-class operation** — respects real workflow where specs don't finish cleanly.
9. **`load_spec` ID assertion** — prevents source-of-truth drift between filename and frontmatter.
10. **Actionable error messages** — every error tells you what to do next.

## Conclusions

The spec system is architecturally sound — 10 unanimous strengths vs 14 findings, most at Low severity. The top 4 findings (promoted to specs) address real gaps: type safety, observability, data truth consistency, and performance. The remaining 10 findings are improvement opportunities, not defects.

**Priority ordering for the 4 actionable specs:**
1. [[spec-next-typed]] — smallest change, highest type-safety ROI
2. [[spec-scan-efficiency]] — unblocks [[spec-query-filesystem-truth]]
3. [[spec-query-filesystem-truth]] — eliminates the dual-truth risk
4. [[spec-history]] — highest user-value but largest scope

The system's biggest meta-strength is that it's **self-auditable** — every lifecycle event is in git, every decision traces to a spec, and the `_value()` pattern means programmatic access is built in from day one.
