---
type: feat
id: surface-layer
status: design
created: 2026-01-08
updated: 2026-01-22
session-origin: 20260108-124107
sessions: [20260108-200725, 20260109-063849, 20260115-053944, 20260115-121358, 20260122-061519]
related:
  - eval/temporal-error-analysis.md
  - layer/surface/build/feat/belief-validation-system/SPEC.md
---

# feat: Surface Layer

**Problem:** Accumulated project wisdom is locked in eventlog/embeddings (local, not portable). When starting a new project, past learnings aren't visible. Other projects can't query your knowledge.

**Solution:** `patina surface` command that distills knowledge into atomic markdown files with wikilinks. Two functions: **Capture** (extract from scry/assay) and **Curate** (score importance, manage lifecycle).

---

## Quick Reference

| Command | Purpose | Phase |
|---------|---------|-------|
| `patina surface capture` | Generate nodes from scry/assay | M1-M2 |
| `patina surface status` | Show importance rankings | M3 |
| `patina surface connections` | Show session→commit links | M2 |

---

## North Star

> When I start a new project, my accumulated wisdom should be visible and usable from day 1.

Not queryable. Not "run scry and hope." **Visible. In files I can read.**

---

## Success Metrics

Before building, we need measurable success criteria.

### Capture Quality

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Capture Precision | >70% useful nodes | Human review sample of 50 nodes |
| Capture Recall | >50% of decisions | Compare to manual session audit |
| Connection Precision | >80% correct | Validate session→commit sample |

### Connection Scoring

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Calibration | ±10% | P(correct\|conf=0.8) ≈ 0.8 |
| AUC-ROC | >0.75 | ROC curve of connection scores |

### User Value

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Query Hit Rate | >30% | % of scry results with surface nodes |
| New Project Bootstrap | <1 hour | Time to first useful surface query |

---

## Milestones

| # | Name | Exit Criteria | Blocking Metric |
|---|------|---------------|-----------------|
| M1 | Capture Foundation | Generate component nodes from assay | Yes/No |
| M2 | Connection Scoring | Session→commit links with confidence | Connection Precision >80% |
| M3 | Curate Foundation | `surface status` shows rankings | Importance correlation >0.6 |
| M4 | LLM Synthesis | Adapter generates decision/pattern nodes | Synthesis Precision >70% |
| M5 | Automated Curation | Auto-detect stale, suggest archive | Archive Recall >70% |

---

## Baseline Measurements (Do First)

Before implementing, establish ground truth:

1. **Manual Session Audit** - Review 10 sessions, list decisions/patterns/concepts
2. **Existing Surface Inventory** - Count/categorize current `layer/surface/` content
3. **Scry Coverage** - How often does scry return session content vs code?

Output: `eval/surface-ground-truth.json`

---

## Status

- **Phase:** Design complete, needs baseline measurement
- **Blocking:** None
- **Risk:** Low (uses existing oxidize/scry infrastructure)
- **Next:** Create ground truth, then implement M1

---

See [[design.md]] for architecture, L2 eventlog design, and implementation details.
