---
type: feat
id: epistemic-e6
status: design
created: 2026-02-02
updated: 2026-02-02
sessions:
  origin: 20260202-134539
related:
  - layer/surface/build/feat/epistemic-layer/SPEC.md
  - layer/surface/build/feat/epistemic-layer/E5/SPEC.md
---

# feat: Curation Automation (E6)

> Automate belief lifecycle: promote the proven, archive the stale, resurrect the re-validated.

**Parent:** [epistemic-layer](../SPEC.md) phase E6
**Prerequisite:** E5 (revision and reasoning provide the decision inputs for curation)

---

## Problem

As the belief corpus grows, manual curation doesn't scale. Beliefs that are heavily used and
verified across time should promote to core. Beliefs with no usage, stale evidence, and no
verification queries should archive to dust. Beliefs in dust that gain new evidence or
cross-project confirmation should resurrect.

---

## Scope

- [ ] Importance scoring based on usage + verification health + semantic centrality
- [ ] Promotion: surface → core (high entrenchment, verified across time, high use)
- [ ] Archival: surface → dust (low use, stale, no verification queries)
- [ ] Resurrection: dust → surface (re-validated by new evidence or cross-project confirmation)

---

## Exit Criteria

- [ ] Automated promotion recommendations surfaced in `belief audit`
- [ ] Automated archival recommendations for low-signal beliefs
- [ ] At least 1 belief promoted surface → core via automated flow
- [ ] At least 1 belief resurrected dust → surface via new evidence

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-02 | design | Extracted from epistemic-layer monolith during spec decomposition |
