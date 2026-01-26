---
type: explore
id: clawdbot-patterns
status: design
created: 2026-01-25
updated: 2026-01-25
sessions:
  origin: 20260125-172601
  work: []
related:
  - ./research.md
  - layer/surface/epistemic/beliefs/phased-development-with-measurement.md
  - layer/surface/build/deferred/spec-skills-universal.md
---

# explore: Clawdbot Pattern Validation

> Don't build what you can't measure. (Andrew Ng)

**Problem:** Deep dive into Clawdbot identified patterns that look useful. But "read it in another repo" isn't evidence. Need to test before adopting as beliefs.

**Thesis:** For each observed pattern: define hypothesis, design test, establish baseline, apply, measure, decide.

---

## Exit Criteria

- [x] Patterns documented with hypotheses
- [x] Test approach defined for each pattern
- [x] Triaged by current use case
- [ ] SKILL.md frontmatter - proceed via spec-skills-universal
- [ ] Workflow runbooks - tested on complex procedure
- [ ] Learnings in skills - tracked in sessions
- [ ] Multi-agent safety - tested when use case exists

---

## Patterns Under Evaluation

### 1. Multi-Agent Safety Protocols

**Observed in:** `AGENTS.md`

```markdown
- Do NOT create/apply/drop git stash entries
- Do NOT switch branches unless explicitly requested
- When user says "commit", scope to YOUR changes only
```

**Hypothesis:** Explicit rules prevent git conflicts when multiple LLM sessions touch same repo.

**Test Design:**

| Step | Action |
|------|--------|
| 1 | Run 2 Claude sessions on Patina simultaneously |
| 2 | Session A: refactor module, Session B: add tests to same module |
| 3 | Measure: conflicts, stash collisions, commit scope errors |

**Baseline:** No explicit rules

**Status:** Deferred - no multi-agent use case today

---

### 2. Workflow Runbooks

**Observed in:** `.agent/workflows/update_clawdbot.md`

Step-by-step procedure with decision points, bash snippets, troubleshooting sections.

**Hypothesis:** Documented step-by-step procedures reduce errors on complex tasks vs ad-hoc execution.

**Test Design:**

| Step | Action |
|------|--------|
| 1 | Pick repeatable complex task (e.g., "add language extractor") |
| 2 | Execute without runbook, record steps and errors |
| 3 | Create runbook from experience |
| 4 | Execute again with runbook, compare error rate and time |

**Baseline:** Current approach (specs define what, not step-by-step how)

**Question:** How different from specs? Specs = requirements, runbooks = execution steps.

**Status:** Pending - test on next complex procedure

---

### 3. SKILL.md Frontmatter with Requirements

**Observed in:** `skills/*/SKILL.md`

```yaml
---
name: github
description: "Interact with GitHub..."
metadata: {"clawdbot":{"requires":{"bins":["gh"]}}}
---
```

**Hypothesis:** Machine-readable requirements enable `patina doctor --skills` validation.

**Test Design:**

| Step | Action |
|------|--------|
| 1 | Add frontmatter to existing Patina skills |
| 2 | Implement skill validation in `patina doctor` |
| 3 | Measure: skills failing due to missing deps (before/after) |

**Status:** Proceed - already planned in `spec-skills-universal.md`, aligns with agentskills.io standard

**Decision:** No new testing needed, external standard validates approach.

---

### 4. Learnings Sections in Skills

**Observed in:** `skills/coding-agent/SKILL.md`

```markdown
## Learnings (Jan 2026)
- PTY is essential: Coding agents need pseudo-terminal
- Git repo required: Codex won't run outside git directory
```

**Hypothesis:** Skill-level learnings prevent rediscovering same issues.

**Test Design:**

| Step | Action |
|------|--------|
| 1 | Track issue rediscovery in sessions (baseline) |
| 2 | Add learnings section to skills after discoveries |
| 3 | Measure: reduction in rediscovery rate |

**Question:** Overlap with beliefs? Beliefs = project-level, learnings = skill-specific.

**Status:** Pending - needs session history analysis

---

## Summary

| Pattern | Use Case Today? | External Validation? | Decision |
|---------|-----------------|---------------------|----------|
| Multi-agent safety | No | Clawdbot experience | Defer |
| Workflow runbooks | Maybe | None | Test opportunistically |
| SKILL.md frontmatter | Yes | agentskills.io spec | Proceed |
| Learnings in skills | Maybe | None | Test via sessions |

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-25 | design | Created from session clawdbot comparison |
