---
type: explore
id: anti-slop
status: design
created: 2026-01-23
updated: 2026-01-23
sessions:
  origin: 20260123-050814
  work: []
related:
  - layer/surface/build/spec-epistemic-layer.md
  - layer/surface/epistemic/beliefs/
---

# explore: Signal Over Noise

> Patina: Local-first quality layer that surfaces signal over noise. Captures understanding alongside implementation. Completes git, doesn't compete with it.

**Problem:** Open source faces increasing noise across all contribution surfaces - not just code PRs, but issues, discussions, and docs. AI amplifies this by generating content that's syntactically correct but context-free. Git tracks *what* changed but not *why* or *under what understanding*.

**Thesis:** Spec captures understanding. Git captures implementation. Integrating these creates a fuller picture - and a potential trust signal for quality contributions.

---

## Exit Criteria

- [x] Problem space documented (noise types across surfaces)
- [x] Patina's existing capabilities mapped to signal/noise filtering
- [x] Core mechanism identified (linkage as signal)
- [x] Honest limitations documented
- [x] Existing code audited (what's real vs vision)
- [x] Linkage discipline documented (conventions that work today)
- [x] Build roadmap defined (what code is needed)
- [ ] Demonstrated on Patina repo (this spec → this session → commits)
- [ ] Prototype spec→code coverage table
- [ ] Prototype linkage scoring in scry

---

## The Core Insight

**The signal IS the linkage.**

A quality contribution can be traced:

```
Spec (why this exists)
    ↓ links to
Session (work record)
    ↓ links to
Commit (implementation)
    ↓ links to
Diff/Code (grounded change)
```

If these connections exist and are coherent, that's signal. If they don't, that's noise.

**No new tools needed.** Just discipline in linking existing artifacts:
- Specs link to sessions (`sessions: origin: YYYYMMDD`)
- Sessions link to commits (activity logs, tags)
- Commits reference specs (`implements explore/anti-slop`)
- Code changes are grounded in diffs

The question becomes: **Can you trace this code change back to a spec that explains why?**

---

## The Integration Model

Git solved **what changed**. Patina solves **why, under what understanding, aligned with what beliefs**.

```
┌─────────────────────────────────────┐
│  Frontend: Claude Code, editors     │  ← Where work happens
└─────────────────────────────────────┘
                ↓
┌─────────────────────────────────────┐
│  Patina: Local quality layer        │  ← Understanding capture
│  - Specs (problem/solution)         │
│  - Beliefs (alignment)              │
│  - Sessions (provenance)            │
└─────────────────────────────────────┘
                ↓
┌─────────────────────────────────────┐
│  Git: Version control               │  ← Implementation capture
└─────────────────────────────────────┘
                ↓
┌─────────────────────────────────────┐
│  Backend: GitHub/GitLab/Gitea       │  ← Distribution
└─────────────────────────────────────┘
```

**Key properties:**
- **Branch/fork compatible** - Patina is just files in a repo. No protocol changes.
- **Local-first** - No server dependency, works offline, your knowledge stays yours.
- **Backend agnostic** - Works with any git-compatible backend.
- **Mac-centric (for now)** - Worth the tradeoff. Ship excellent on one platform, expand later.

---

## The Trust Layer Thesis

If Patina proves itself as a quality signal on its own repo, "made with Patina" could become a trust marker:

1. **Dogfood** - Patina uses Patina, builds track record
2. **Empirical evidence** - Patina-assisted contributions have better outcomes (fewer reverts, faster reviews, less churn)
3. **Reputation transfer** - "Made with Patina" becomes a signal maintainers can weight
4. **Adoption flywheel** - More repos use Patina → more data → stronger signal

Not cryptographic proof. **Empirical track record.**

### What "Proving Itself" Looks Like

Measurable outcomes for Patina's own repo:
- PR quality (review cycles needed)
- Revert rate (contributions that stick)
- Time to merge (faster with context?)
- Duplicate issue rate (scry catching repeats)

Build the case study on ourselves first.

---

## Noise Across Surfaces

| Surface | Noise Forms | Signal Characteristics |
|---------|-------------|----------------------|
| **Code PRs** | Generic changes, context-free | Aligns with patterns, references beliefs |
| **PR descriptions** | Vague "improved X", AI boilerplate | Explains why, references project context |
| **Issues** | Generic bug reports, AI feature requests | Specific, relates to existing work |
| **Discussions** | Drive-by opinions, repeated questions | Builds on captured knowledge |
| **Docs PRs** | Surface-level rewording | Addresses actual gaps, matches voice |

---

## Two Perspectives

### 1. Patina as a Public Repo
How does Patina protect itself from noise?

### 2. Patina as a Tool
How does Patina help other repos filter signal from noise?

---

## How Patina Helps (Existing Capabilities)

| Capability | Signal/Noise Value |
|------------|-------------------|
| **Scry** | Duplicate detection - "similar to #142, related to belief X" |
| **Beliefs** | Alignment filter - "conflicts with `sync-first`" |
| **Sessions** | "Already explored" detector - "covered in session 20250815" |
| **Context** | Triage assist - surface relevant patterns for review |
| **Issues index** | Semantic search across existing issues |

### Scry as Duplicate/Relevance Detector

New issue comes in. Run:
```bash
patina scry "user can't login after password reset"
```
→ "Similar to #142, #89. Related belief: `session-management`"

Surfaces: is this noise (duplicate) or signal (new information)?

### Beliefs as Alignment Filter

Issue requests "add async everywhere". Check against beliefs:
```
⚠️ May conflict with:
- sync-first (confidence: 0.88)
See layer/surface/epistemic/beliefs/sync-first.md
```

Not blocking - surfacing context. Maybe the request is valid and belief should revise. Or maybe it's noise.

### Sessions as "Already Explored" Signal

Proposal for approach X. Scry reveals:
```
Session 20250815-103422: "Explored X, rejected due to Y"
Belief: `not-x` (confidence: 0.75)
```

Saves time - we already thought about this.

---

## Signal Detection Questions

| Question | Patina Capability |
|----------|------------------|
| Have we seen this before? | Scry for similar content |
| Does this align with our direction? | Check against beliefs |
| Did we already decide against this? | Search sessions/defeated beliefs |
| Who has context on this? | Session provenance |
| Is this generic or specific? | Pattern match against project vocabulary |

---

## Linkage: What EXISTS Today

### The Linkage Graph (Current State)

```
Spec ──────────────────────────────────────────────────────────────
  │
  │ sessions.origin: YYYYMMDD             [MANUAL - frontmatter]
  ▼
Session ───────────────────────────────────────────────────────────
  │
  │ git tags: session-YYYYMMDD-start/end  [AUTO - session scripts]
  │ activity logs mention commits          [MANUAL - markdown]
  ▼
Commit ────────────────────────────────────────────────────────────
  │
  │ find_session_for_commit()             [AUTO - timestamp-based]
  │ session_id in eventlog JSON           [AUTO - stored on scrape]
  │ "Implements:" in message              [MANUAL - not parsed]
  ▼
Code ──────────────────────────────────────────────────────────────
  │
  │ (nothing)                             [GAP - no link exists]
  ▼
```

### Link Summary

| From | To | Method | Auto/Manual |
|------|-----|--------|-------------|
| Spec | Session | `sessions: origin:` frontmatter | Manual |
| Session | Commits | Git tags bracket the session | Auto |
| Commit | Session | `find_session_for_commit()` | Auto |
| Commit | Eventlog | `session_id` in JSON | Auto |
| Commit | Spec | `Implements:` in message | Manual (not parsed) |
| Belief | Session | `## Evidence` wikilinks | Manual |
| Code | Spec | **NOTHING** | **GAP** |

### Verified in Codebase

| Capability | Status | Code Location |
|------------|--------|---------------|
| **Commit→Session linking** | ✅ EXISTS | `src/commands/scrape/git/mod.rs:115` `find_session_for_commit()` |
| **Session_id in eventlog** | ✅ EXISTS | `src/commands/scrape/git/mod.rs:361` stored in JSON |
| **Sessions indexed** | ✅ EXISTS | `src/commands/scrape/sessions/mod.rs` tables: sessions, observations, goals |
| **Beliefs with confidence** | ✅ EXISTS | `src/commands/scry/internal/enrichment.rs:46-86` |
| **Semantic search all types** | ✅ EXISTS | Code, patterns, commits, beliefs queryable |
| **Co-change analysis** | ✅ EXISTS | Temporal relationships in scrape |

### How Commit→Session Works (Real Code)

```rust
// src/commands/scrape/git/mod.rs:115-144
fn find_session_for_commit(commit_time: &str, sessions: &[SessionBounds]) -> Option<String> {
    // Links commits to sessions BY TIMESTAMP
    // If commit_time falls within session start/end tags, returns session_id
}
```

This is **temporal linking** - commits made during a session are associated with it. The `session_id` is stored in the eventlog JSON for each commit.

### What's Missing (Gaps)

| Gap | Current State |
|-----|---------------|
| **Spec→code mapping** | No table, no analysis |
| **Commit message parsing** | "Implements:" not parsed, only timestamps |
| **Linkage scores in scry** | Scry returns similarity, not linkage |
| **Belief alignment scoring** | No automatic check |

### Discipline Layer (No Code Needed)

These work TODAY through convention:

1. **Spec frontmatter** - `sessions: origin: YYYYMMDD` (manual, works)
2. **Session activity logs** - Commits listed in markdown (manual, works)
3. **Commit message convention** - `Implements: explore/anti-slop` (manual, not parsed)
4. **Scry for discovery** - Find related specs/sessions semantically (works)

### The Quality Question

For any contribution, ask:

> Can I trace this change back to a spec that explains why it exists?

- **Yes, full chain** → Signal (understanding demonstrated)
- **Partial chain** → Review needed (some context, gaps remain)
- **No chain** → Noise (context-free, generic)

---

## Linkage Measurement: TO BUILD

> **Status: VISION** - This section describes features that don't exist yet.

### Target Linkage Graph

```
          ┌──────────────────────────────────────────────────────┐
          │                                                      │
          ▼                                                      │
Spec ◄───────────────────────────────────────────────────────┐   │
  │                                                          │   │
  │ sessions.origin                                          │   │
  ▼                                                          │   │
Session ◄────────────────────────────────────────────────┐   │   │
  │                                                      │   │   │
  │ git tags                                             │   │   │
  ▼                                                      │   │   │
Commit ──────────────────────────────────────────────────┼───┘   │
  │         find_session_for_commit() [EXISTS] ──────────┘       │
  │         parse "Implements:" [TO BUILD] ──────────────────────┘
  │
  ▼
Code ────────────────────────────────────────────────────────────
          spec_coverage table [TO BUILD] links back to Spec
```

The same semantic system that indexes beliefs COULD measure linkage quality. This would require new code.

### The Pattern (Aspirational)

| What We Measure | Current | Target |
|-----------------|---------|--------|
| **Beliefs** | ✅ Indexed, scored, queryable | Done |
| **Linkage** | ❌ Not computed | Indexed, scored, queryable |

### Linkage Signals (To Implement)

| Signal | Description | Implementation Needed |
|--------|-------------|----------------------|
| `spec_coverage` | Does code have a spec? | Parse specs for paths, build coverage table |
| `session_provenance` | Developed in session? | ✅ EXISTS via `find_session_for_commit` |
| `commit_context` | Commit refs spec? | Parse "Implements:" from commit messages |
| `belief_alignment` | Aligns with beliefs? | Semantic similarity between diff and beliefs |

### What Would Need to Be Built

**Estimated: 500-1000 lines of Rust**

1. **Spec→code mapping table** (~200 lines)
   - Parse specs for file/function references
   - Store in SQLite: `spec_coverage(spec_id, path_pattern)`
   - Query: "Which spec covers this file?"

2. **Commit message parser** (~100 lines)
   - Extract `Implements:` and `Session:` fields
   - Store in commits table or eventlog

3. **Linkage score computation** (~200 lines)
   - Aggregate signals into score
   - Store per-file or per-function

4. **Scry enrichment** (~100 lines)
   - Add linkage info to `enrich_results()`
   - Display in scry output

### Envisioned Output

```bash
# FUTURE - doesn't exist yet
patina scry "src/commands/scrape"
→ [1] Score: 0.91 | code | src/commands/scrape/mod.rs
      Linkage: 0.92 (spec: spec-pipeline, session: 20260115)
```

### What This Would Enable

- **Codebase health dashboard** - Overall linkage score
- **PR review signal** - Flag low-linkage contributions
- **Tech debt identification** - Code without spec coverage
- **Contribution quality metrics** - Track over time

### Build Priority

| Component | Value | Effort | Priority |
|-----------|-------|--------|----------|
| Session provenance | High | ✅ Done | - |
| Spec coverage | High | Medium | P1 |
| Commit parsing | Medium | Low | P2 |
| Belief alignment | Medium | Medium | P3 |
| Scry integration | High | Low | P1 (after data) |

---

## Asymmetric Friction Model

Noise economics require: generate quickly, submit to many projects, hope some stick.

**Anything requiring project-specific engagement breaks this model** - not cryptographic security, just doesn't scale for noisy actors.

Goal: Friction **low for signal** (Patina makes it easy to learn project context) and **high for noise** (requires engagement noisy actors won't do).

---

## What Can't Be Faked (Easily)

| Signal | Fakeable? | Notes |
|--------|-----------|-------|
| Process followed | Yes | Tools don't know who uses them |
| Intent stated | Yes | Words are cheap |
| Project-specific knowledge | Harder | Requires actual engagement |
| Outcomes over time | Hardest | Requires track record |

**Limitation:** Bad actor using Patina correctly produces same signals as good actor. Process verification necessary but not sufficient.

---

## Non-Goals (For Now)

- **External detection tools** - Bots, CI integrations, GitHub Apps
- **ZK proofs of understanding** - Cryptographic verification
- **On-chain reputation** - Outcome tracking, slashable stake
- **Proof of personhood** - Anti-sybil mechanisms
- **Git protocol changes** - New metadata formats in git itself

**Clarification:** We DO need to build ~500-1000 lines of Rust to compute linkage scores. But this extends existing Patina infrastructure (scrape, scry), not external systems.

See [[design.md]] for extended exploration of deferred ideas.

---

## Honest Limitations

This filters **lazy noise**, not **determined bad actors**.

Someone willing to engage with Patina could still submit garbage with plausible context. But that's a smaller threat surface.

**Goal isn't perfect filtering. It's raising signal-to-noise ratio.**

---

## Open Questions

1. **Adoption** - How do contributors learn Patina before it's widespread?
2. **False positives** - What if genuine contributors don't use Patina?
3. **Gaming** - Once patterns known, can noise generators fake engagement?
4. **Measurement** - How do we know this improves signal/noise ratio?
5. **Barrier height** - Is "use Patina" too high for casual contributors?
6. **Cross-project** - Can noise patterns from one repo inform another?

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-23 | design | Initial exploration from session discussion |
| 2026-01-23 | design | Expanded from "anti-slop" to "signal over noise" across all surfaces |
| 2026-01-23 | design | Added trust layer thesis and integration model |
| 2026-01-23 | design | Reframed: linkage as signal, not new tools |
| 2026-01-23 | design | Added linkage measurement via semantic system |
| 2026-01-23 | design | Reality check: audited code, separated EXISTS vs TO BUILD |
| 2026-01-23 | design | Added linkage graph diagrams (current state + target state) |

---

## See Also

- [[design.md]] - Extended thinking on git blame for intent, ZK ideas
- [[spec-epistemic-layer.md]] - Beliefs/patterns foundation
- [[session-20260123-050814]] - Origin discussion
