---
type: feat
id: spec-drift-detection
status: building
created: 2026-02-02
updated: 2026-02-02
sessions:
  origin: 20260202-104403
related:
  - layer/surface/build/feat/epistemic-layer/SPEC.md
  - layer/surface/build/feat/v1-release/SPEC.md
  - layer/surface/build/refactor/spec-system/SPEC.md
beliefs:
  - stale-context-is-hostile-context
---

# feat: Spec Drift Detection

**Problem:** Stale specs poison LLM context reboots. An LLM trusts what it reads — a spec
claiming "E4.5 (exploring)" when E4.5 is complete causes the next session to plan work that's
already done and miss work that emerged during building.

**Belief:** `stale-context-is-hostile-context` — the human thinks non-linearly across many
timelines and can't maintain all specs linearly. The system must detect decay, not rely on
discipline.

**Constraint:** This is not a doc maintenance problem. It's a fundamental break in the
human/LLM symbiotic relationship. Patina exists to be the accurate context reboot layer.
Every stale spec is a bug in Patina's primary mission.

---

## North Star

> If a spec's `updated:` date is older than commits touching files it describes, the spec
> is drifting. The system detects this and surfaces it before an LLM reads stale context.

---

## Design

### What "Staleness" Means

A spec is stale when its description of reality has diverged from reality. Three measurable
signals:

| Signal | Mechanism | Strength |
|--------|-----------|----------|
| **Temporal gap** | `updated:` frontmatter date vs last git commit touching related files | High — cheap, works on every spec |
| **Status contradiction** | Spec says `status: building` but related tasks are all checked | Medium — requires parsing checkboxes |
| **Assertion failure** | Spec carries checkable claims about code/data that no longer hold | Highest — like belief verification for specs |

### What We Already Have

| Infrastructure | Location | Relevance |
|----------------|----------|-----------|
| Frontmatter `updated:` field | All 10 active specs | Human-maintained timestamp; currently not parsed by scraper |
| Frontmatter `status:` field | All specs | Parsed by scraper into `patterns` table |
| Git commit history | `commits` table in patina.db | Timestamp, files changed, message |
| Co-change data | `co_changes` table | Which files change together |
| Layer scraper | `src/commands/scrape/layer/mod.rs` | Already parses frontmatter; doesn't parse `updated:` |
| Doctor command | `src/commands/doctor.rs` | Health checks with warning/critical levels |
| Session start | `src/commands/session/internal.rs` | Surfaces beliefs; could surface stale specs |
| Belief verification | E4.5 | Proves beliefs against DB facts — same pattern for spec assertions |

### What's Missing

1. Scraper doesn't parse `updated:` field from frontmatter
2. No code compares `updated:` against git history for related files
3. Doctor doesn't check spec health
4. Session-start doesn't warn about stale specs
5. Specs carry no machine-checkable assertions about their own currency

---

## Staleness Detection: Three Layers

### Layer 1: Temporal Drift (cheap, automatic)

**Mechanism:** Compare spec's `updated:` date against the most recent commit touching files
the spec describes.

**How to find "related files":**

A spec's related files come from two sources:
1. **Explicit references** in frontmatter: `related:` field lists other specs/files
2. **Content references**: file paths mentioned in the spec body (e.g., `src/commands/scry/mod.rs`)
3. **Naming convention**: spec `feat/epistemic-layer/SPEC.md` relates to files matching
   `*epistemic*`, `*belief*` (extracted from spec `id` + `facets`/`tags`)

**Algorithm:**
```
for each spec in patterns table:
    spec_updated = parse updated: from frontmatter (or file mtime as fallback)
    related_files = extract_related_files(spec)
    latest_commit = max(commit.timestamp) WHERE commit touches any related_file
    drift_days = latest_commit.date - spec_updated.date
    if drift_days > threshold:
        mark spec as stale(drift_days, latest_commit)
```

**Thresholds:**
- 0-7 days: fresh (spec recently updated relative to code changes)
- 7-30 days: aging (code changed since spec was last touched)
- 30+ days: stale (significant drift — spec likely describes past reality)
- Active spec with 0 recent commits: dormant (no code activity, spec may be fine)

**Storage:** Add `updated` and `staleness_days` columns to `patterns` table.

### Layer 2: Status Contradiction (medium, scrape-time)

**Mechanism:** Cross-reference spec `status:` against its internal checkbox state.

**Signals:**
- Spec says `status: building` but all `- [x]` checkboxes are checked → may be complete
- Spec says `status: in_progress` but no commits reference it in 60+ days → may be dormant
- Spec says `status: complete` but isn't archived via `patina spec archive` → should archive

**Algorithm:**
```
for each spec:
    total_checkboxes = count(lines matching "- [ ]" or "- [x]")
    checked = count(lines matching "- [x]")
    if status == "building" and checked == total_checkboxes and total_checkboxes > 0:
        warn("spec appears complete but status is still building")
    if status in ["building", "in_progress"]:
        last_related_commit = ... (from Layer 1)
        if days_since(last_related_commit) > 60:
            warn("spec appears dormant — no related commits in 60 days")
```

### Layer 3: Spec Assertions (highest value, opt-in)

**Mechanism:** Specs carry machine-checkable claims about their own state, similar to belief
verification queries. When these assertions fail, the spec is concretely stale — not just
time-based suspicion.

**Format:** A new `## Assertions` section in spec markdown:

```markdown
## Assertions
<!-- Machine-checkable claims about this spec's currency -->

- [query: "SELECT COUNT(*) FROM beliefs"] > 40
  <!-- Spec references "46 beliefs" — if count drops below 40, spec text is wrong -->

- [file: "src/commands/scry/mod.rs"] contains "belief"
  <!-- Spec describes scry belief mode — if removed, spec is stale -->

- [status: "belief-verification"] == "archived"
  <!-- Spec references E4.5 as complete — if not archived, spec text is wrong -->
```

**Why this is powerful:** Temporal drift is a heuristic. Assertions are proofs. A spec that
says "46 beliefs" with an assertion `SELECT COUNT(*) FROM beliefs > 40` will fail when beliefs
are removed — catching staleness that time-based detection would miss.

**This is the belief verification pattern applied to specs.** E4.5 proved that connecting
claims to DB facts works. Spec assertions extend this to spec claims about code/data state.

**Implementation:** Reuse the verification engine infrastructure. Parse assertion blocks from
spec markdown. Run during `patina scrape` or `patina doctor`. Surface failures as warnings.

---

## Surfacing: Where Staleness Becomes Visible

### In `patina doctor`

Add a "Spec Health" section:

```
Spec Health:
  10 active specs
  ✅ 6 fresh (updated within 7 days of related commits)
  ⚠️  3 aging (7-30 days behind related commits)
  ❌ 1 stale (epistemic-layer: updated 2026-01-22, related commits 2026-02-02 = 11 days)
  ⚠️  1 status contradiction (belief-verification: all checkboxes checked but status=building)
```

Exit code: warning (2) if any aging, critical (3) if any stale.

### In `patina session start`

After surfacing previous session beliefs, show stale specs relevant to the session:

```
Stale specs touching your work:
  ⚠️  epistemic-layer (11 days behind) — last updated 2026-01-22, related commits today
  ⚠️  v1-release (4 days behind) — milestone 0.9.4 shipped but spec shows 0.9.3
```

**Filtering:** Only show specs related to files changed on the current branch since diverging
from main. Don't show all stale specs — show the ones that matter for this session.

### In `patina scrape` output

Add a staleness summary after pattern indexing:

```
Layer: 15 patterns (3 new, 12 unchanged)
  Staleness: 6 fresh, 3 aging, 1 stale
  ❌ epistemic-layer: updated 2026-01-22, latest related commit 2026-02-02
```

### In belief audit (future)

The `stale-context-is-hostile-context` belief itself should have a verification query that
checks whether spec drift detection is operational — dogfooding the system.

---

## Build Steps

### Phase 1: Temporal Drift Detection (core mechanism)

- [ ] 1. Parse `updated:` field in layer scraper (`src/commands/scrape/layer/mod.rs:189`)
  - Add regex for `updated:` alongside existing `created:` parser
  - Store in `patterns` table as new `updated` column
  - Fallback: use `git log -1 --format=%aI -- <file>` for specs missing `updated:`

- [ ] 2. Extract related files from spec content
  - Parse `related:` frontmatter field (already extracted as `references`)
  - Extract file paths from spec body (regex for `src/...`, `layer/...` paths)
  - Map spec `id` to likely related paths (e.g., `epistemic-layer` → `*epistemic*`, `*belief*`)

- [ ] 3. Compute staleness score during scrape
  - For each spec: find latest commit touching any related file
  - Compare against spec's `updated:` date
  - Store `staleness_days` and `latest_related_commit` in patterns table

- [ ] 4. Surface staleness in `patina scrape` output
  - After "Layer: N patterns" line, add staleness summary
  - List any stale specs (>30 days) by name

### Phase 2: Doctor Integration

- [ ] 5. Add "Spec Health" check to `patina doctor`
  - Query patterns table for specs with staleness data
  - Categorize: fresh/aging/stale
  - Include in doctor output with appropriate severity

- [ ] 6. Add status contradiction detection
  - Parse checkbox state from spec markdown during scrape
  - Compare against `status:` field
  - Surface contradictions in doctor output

### Phase 3: Session Integration

- [ ] 7. Surface stale specs in `patina session start`
  - After belief context, show stale specs relevant to current branch
  - Filter by: specs whose related files overlap with files changed on branch
  - Output format: spec name, days behind, what changed

### Phase 4: Spec Assertions (stretch)

- [ ] 8. Define assertion format in spec markdown
  - `## Assertions` section with machine-checkable claims
  - Support: SQL queries against patina.db, file existence/content checks, status checks

- [ ] 9. Parse assertions during scrape
  - Extract assertion blocks from spec content
  - Store in new `spec_assertions` table (spec_id, assertion_type, assertion_text, last_result)

- [ ] 10. Run assertions in `patina doctor --deep`
  - Execute each assertion against DB/filesystem
  - Surface failures as spec staleness warnings
  - Reuse verification engine infrastructure where possible

---

## Exit Criteria

- [ ] `patina scrape` parses `updated:` and computes staleness for all specs
- [ ] `patina doctor` surfaces stale specs with fresh/aging/stale categories
- [ ] `patina session start` shows stale specs relevant to current branch
- [ ] At least 3 specs demonstrate meaningful staleness detection (not false positives)
- [ ] Status contradiction detection catches at least 1 real case

### Stretch Exit Criteria

- [ ] At least 2 specs carry assertions that are machine-checkable
- [ ] `patina doctor --deep` runs spec assertions and surfaces failures

---

## What This Spec Does NOT Tackle

- **Auto-fixing stale specs** — detection only, not remediation (LLM + human fix specs)
- **Spec versioning** — git history is the version system
- **Cross-project spec drift** — mother federation scope
- **Real-time monitoring** — runs at scrape/doctor/session-start time, not continuously

---

## Design Decisions

### Why `updated:` field, not git mtime?

Git mtime of the spec file tells you when the file was last committed. But a spec might be
committed for formatting changes without updating its substantive content. The `updated:` field
is a human signal: "I reviewed and updated this spec's claims about reality." The gap between
`updated:` (last human review) and latest related commit (latest reality change) is the
meaningful measure of drift.

Fallback to git mtime when `updated:` is missing — something is better than nothing.

### Why related-file heuristics, not explicit linking?

Requiring humans to maintain explicit file lists in specs is the same discipline problem we're
solving. The system should infer relatedness from:
1. What's already in the spec (file paths, references)
2. Naming conventions (spec ID → related code paths)
3. Co-change data (files that change together)

Explicit `related:` is a bonus signal, not a requirement.

### Why not auto-update `updated:` on every commit?

That would defeat the purpose. `updated:` means "a human verified this spec reflects reality."
Auto-updating it would make staleness invisible again. The gap is the signal.

---

## References

- [[stale-context-is-hostile-context]] — Problem belief
- [[spec-carries-progress]] — Specs must track progress
- [[ground-before-reasoning]] — LLMs must read reality before reasoning
- [[practical-memory-over-epistemic-formalism]] — Decision memory, not knowledge graph
- E4.5 belief verification — Same pattern: connect claims to checkable facts
