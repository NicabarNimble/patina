---
type: feat
id: epistemic-layer
status: in_progress
created: 2026-01-16
updated: 2026-02-02
sessions:
  origin: 20260116-054624
related:
  - layer/surface/build/feat/v1-release/SPEC.md
---

# feat: Epistemic Markdown Layer

> Patina is not a note system. It is epistemic infrastructure for LLM collaboration.

**Prototype:** `layer/surface/epistemic/` (47 beliefs, 3 rules, 47 verification queries)

---

## Phase Index

| Phase | Status | Location |
|-------|--------|----------|
| E0-E3 | complete | archived: `spec/epistemic-e0-e3` |
| E4 steps 1-7 | complete | archived: `spec/epistemic-e4-metrics` |
| E4.5 | complete | archived: `spec/belief-verification` |
| E4.6a | complete | archived: `spec/epistemic-e4.6a-grounding` |
| **E4.6a-fix** | **ready** | [E4.6a-fix/SPEC.md](E4.6a-fix/SPEC.md) — multi-hop code grounding |
| E4.6b | design | [E4.6b/SPEC.md](E4.6b/SPEC.md) — belief↔belief relationships |
| **E4.6c** | **ready** | [E4.6c/SPEC.md](E4.6c/SPEC.md) — forge semantic integration |
| E5 | design | [E5/SPEC.md](E5/SPEC.md) — revision + cross-project reasoning |
| E6 | design | [E6/SPEC.md](E6/SPEC.md) — curation automation |

**Remaining E4 cleanup** (steps 8-10): remove fake `confidence.signals` from 44 belief files,
update scraper for new schema, add belief metrics to MCP `context` tool.

---

## What's Next

**E4.6a-fix** — addresses distribution mismatch where direct belief↔code cosine never reaches
threshold. Uses semantic hop (belief→commit) + structural hop (commit→files→functions).

**Then:** E4.6c (forge embeddings), E4.6b (belief clustering), E4 steps 8-10 (schema migration).

**E4.6 does NOT tackle:** graph traversal/propagation algorithms, transitive attack chains,
cross-project belief routing via mother, or automatic belief revision — all E5 scope.

**E4.6 grounds for the future:** Mother's multi-project belief design needs to know which code a
belief is about (to verify against the right repo's DB) and which beliefs cluster together (to
detect cross-project conflicts). E4.6a provides the semantic bridge (belief→commit), E4.6a-fix
the structural bridge (commit→code via multi-hop), E4.6b typed inter-belief edges, and E4.6c
forge data in the semantic space. Mother consumes these as inputs, not reimplements them.

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
