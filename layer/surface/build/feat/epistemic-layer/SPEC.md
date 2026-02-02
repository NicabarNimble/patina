---
type: feat
id: epistemic-layer
status: complete
created: 2026-01-16
updated: 2026-02-02
sessions:
  origin: 20260116-054624
related:
  - layer/surface/build/feat/v1-release/SPEC.md
---

# feat: Epistemic Markdown Layer

> Patina is not a note system. It is epistemic infrastructure for LLM collaboration.

**Prototype:** `layer/surface/epistemic/` (47 beliefs, 3 rules, 25 verification queries)

---

## Phase Index

| Phase | Status | Location |
|-------|--------|----------|
| E0-E3 | complete | archived: `spec/epistemic-e0-e3` |
| E4 steps 1-7 | complete | archived: `spec/epistemic-e4-metrics` |
| E4 steps 8-10 | complete | schema cleanup, scraper validation, MCP belief metrics |
| E4.5 | complete | archived: `spec/belief-verification` |
| E4.6a | complete | archived: `spec/epistemic-e4.6a-grounding` |
| E4.6a-fix | complete | [E4.6a-fix/SPEC.md](E4.6a-fix/SPEC.md) — multi-hop code grounding |
| E4.6b | deprioritized | [E4.6b/SPEC.md](E4.6b/SPEC.md) — eval showed no LLM behavior change |
| E4.6c | complete | [E4.6c/SPEC.md](E4.6c/SPEC.md) — forge semantic integration |
| E5 | deferred | [E5/SPEC.md](E5/SPEC.md) — blocked on delivery layer (mother scope) |
| E6 | deferred | [E6/SPEC.md](E6/SPEC.md) — blocked on delivery layer (mother scope) |

---

## What's Next

**The epistemic layer is complete as designed.** All buildable phases (E0-E4.6c) are done.

**Remaining work is delivery-layer scope (mother):**
- E4.6b (belief relationships): deprioritized — A/B eval (evaluation-next.md) showed grounding
  annotations don't change LLM behavior (delta -0.05). The belief data is correct and valuable,
  but the retrieval/delivery gap is a mother-scope concern.
- E5 (revision + cross-project): deferred — requires mother's multi-project routing.
- E6 (curation automation): deferred — requires intent→principle matching from mother.

**What this layer provides to mother:**
- 47 beliefs with computed use/truth metrics (E4 steps 1-7)
- 25 verification queries with pass/fail tracking (E4.5)
- Multi-hop code grounding: belief→commit→file (E4.6a-fix, 100% precision, 86% recall)
- Forge semantic integration: issues/PRs in vector space (E4.6c, 81 events embedded)
- MCP context tool with belief metrics (E4 step 10)

---

## See Also

- [design.md](design.md) — AGM framework, schema definitions, confidence model, argument graph
- [evaluation.md](evaluation.md) — Andrew Ng methodology (avg +2.2 delta, 9/10 max score)
- [[spec-surface-layer]] — Parent spec
- [[spec/mothership-graph]] — Cross-project relationships

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-16 | design | Initial spec (session 20260116-054624) |
| 2026-01-22 | in_progress | E0-E3 complete |
| 2026-02-01 | in_progress | E4 steps 1-7 + E4.5 complete |
| 2026-02-02 | in_progress | E4.6a complete, decomposed 1509-line monolith into focused specs |
| 2026-02-02 | in_progress | E4.6a-fix complete — 100% precision, 86% recall |
| 2026-02-02 | in_progress | A/B eval: grounding annotations delta -0.05, delivery gap identified |
| 2026-02-02 | complete | E4 steps 8-10, E4.6c complete — spec finished as designed |
