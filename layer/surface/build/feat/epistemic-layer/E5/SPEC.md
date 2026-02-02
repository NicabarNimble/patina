---
type: feat
id: epistemic-e5
status: design
created: 2026-02-02
updated: 2026-02-02
sessions:
  origin: 20260202-134539
related:
  - layer/surface/build/feat/epistemic-layer/SPEC.md
  - layer/surface/build/feat/epistemic-layer/E4.6b/SPEC.md
---

# feat: Revision and Cross-Project Reasoning (E5)

> When beliefs conflict — within a project or across projects — the system needs a resolution path.

**Parent:** [epistemic-layer](../SPEC.md) phase E5
**Prerequisite:** E4.6 (semantic relationships provide the detection layer E5 reasons over)

---

## Problem

E4.6 detects relationships and conflicts. E5 acts on them. When beliefs conflict —
within a project or across projects — the system needs a resolution path. Today, conflicts are
invisible. E5 makes them visible and resolvable.

---

## Scope

### 5a. Within-Project Revision

- [ ] Conflict detection on new belief (trigger: new belief's embedding is close to existing
  belief with opposing supports/attacks)
- [ ] Entrenchment-based ordering — when conflict detected, surface which belief is more costly
  to remove (higher use metrics, more dependents, longer survival)
- [ ] Adapter LLM proposes resolution (keep both with scopes, weaken one, split into cases,
  replace one)
- [ ] User approval flow — revision logged to eventlog as `surface.belief.revise`

### 5b. Transitive Reasoning

- [ ] Attack chain detection — if A attacks B and B supports C, surface the indirect threat to C
- [ ] Support cluster identification — beliefs that form reinforcing clusters (mutual support)
  are collectively stronger than isolated beliefs
- [ ] Weakness propagation — if a high-entrenchment belief's verification starts failing, surface
  all beliefs that depend on it via support edges

### 5c. Cross-Project Belief Routing (requires mother federation)

- [ ] Beliefs scoped to a project vs. beliefs held by the persona across projects
- [ ] Ref repo beliefs — structural claims about third-party codebases, verified against that
  repo's `patina.db`, held in the persona layer (not in the ref repo)
- [ ] Mother mediation — when project A's belief contradicts project B's belief, mother surfaces
  the conflict with context from both projects' verification results
- [ ] Universal beliefs — claims that apply to all projects (layer/core candidates), promoted
  when verified across multiple project DBs

---

## Exit Criteria

- [ ] Conflicting belief triggers revision flow with LLM proposal
- [ ] User can approve/reject revision
- [ ] Revision logged in eventlog
- [ ] Transitive attack/support chains surfaced in audit
- [ ] At least 1 cross-project belief verified against a ref repo DB

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-02 | design | Extracted from epistemic-layer monolith during spec decomposition |
