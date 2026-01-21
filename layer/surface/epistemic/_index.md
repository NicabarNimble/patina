---
type: index
updated: 2026-01-21
---

# Epistemic Layer Index

This directory contains Patina's epistemic belief system - atomic beliefs with evidence, support/attack relationships, and derived rules.

## Statistics

- **Beliefs**: 15
- **Rules**: 3
- **Total Confidence**: 12.86/15.0 (avg: 0.857)
- **Highest Entrenchment**: eventlog-is-truth (very-high)

## Argument Graph

```
                         CORE BELIEFS
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
          ▼                   ▼                   ▼
   [[eventlog-is-truth]]  [[measure-first]]  [[spec-first]]
        (0.92)              (0.88)            (0.85)
          │                   │                   │
          │         ┌─────────┴─────────┐         │
          │         │                   │         │
          │         ▼                   ▼         │
          │  [[dont-build-what-exists]] ◄─────────┘
          │         (0.90)
          │         │
          │         ├──────────┐
          │         │          │
          ▼         ▼          ▼
   [[smart-model-in-room]]  [[compose-over-build]]
         (0.88)                  (0.85)
```

## Derived Rules

```
BELIEFS                              RULES
────────                             ─────
measure-first ─────────┐
                       ├──► [[implement-after-measurement]]
spec-first ────────────┘

smart-model-in-room ───┐
                       ├──► [[use-adapter-for-synthesis]]
dont-build-what-exists─┘

eventlog-is-truth ─────────► [[capture-at-boundary]]
```

## Belief Inventory

| ID | Confidence | Entrenchment | Status |
|----|------------|--------------|--------|
| [[eventlog-is-truth]] | 0.92 | very-high | active |
| [[dont-build-what-exists]] | 0.90 | high | active |
| [[commit-early-commit-often]] | 0.90 | high | active |
| [[phased-development-with-measurement]] | 0.89 | high | active |
| [[sync-first]] | 0.88 | high | active |
| [[smart-model-in-room]] | 0.88 | high | active |
| [[measure-first]] | 0.88 | high | active |
| [[error-analysis-over-architecture]] | 0.88 | medium | active |
| [[session-git-integration]] | 0.87 | medium | active |
| [[compose-over-build]] | 0.85 | medium | active |
| [[spec-first]] | 0.85 | high | active |
| [[project-config-in-git]] | 0.85 | high | active |
| [[progressive-disclosure]] | 0.82 | medium | active |
| [[system-owns-format]] | 0.80 | medium | active |
| [[skills-for-structured-output]] | 0.75 | medium | active |

## Rule Inventory

| ID | Confidence | Derived From | Status |
|----|------------|--------------|--------|
| [[implement-after-measurement]] | 0.82 | measure-first, spec-first | active |
| [[use-adapter-for-synthesis]] | 0.85 | smart-model-in-room, dont-build-what-exists | active |
| [[capture-at-boundary]] | 0.88 | eventlog-is-truth | active |

## Attack Graph

| Attacker | Target | Status | Scope |
|----------|--------|--------|-------|
| [[analysis-paralysis]] | spec-first | active | "only when spec exceeds 1 week" |
| [[analysis-paralysis]] | error-analysis-over-architecture | active | "only when error analysis exceeds 2 days without findings" |
| [[cost-concerns]] | smart-model-in-room | active | "high-volume synthesis" |
| [[latency-concerns]] | smart-model-in-room | active | "real-time interactions" |
| [[latency-concerns]] | progressive-disclosure | active | "on-demand loading adds delay" |
| [[storage-overhead]] | eventlog-is-truth | scoped | "ref repos use lean storage" |
| [[measurement-overhead]] | measure-first | active | "trivial changes" |
| [[high-concurrency-needed]] | sync-first | active | "network-heavy or many parallel connections" |
| [[streaming-responses]] | sync-first | active | "long-running streaming APIs" |
| [[adapter-agnostic-required]] | skills-for-structured-output | active | "supporting Gemini CLI or OpenCode" |
| [[llm-flexibility-needed]] | system-owns-format | active | "output format varies by context" |
| [[context-switching-cost]] | commit-early-commit-often | active | "mitigated by session tracking" |
| [[secret-leakage-risk]] | project-config-in-git | active | "mitigated by splitting project config from secrets" |

## Defeated Beliefs

| Belief | Defeated By | Reason |
|--------|-------------|--------|
| [[move-fast-break-things]] | spec-first, error-analysis-over-architecture | leads to rework |
| [[not-invented-here]] | dont-build-what-exists | - |
| [[local-model-always]] | smart-model-in-room | quality gap too large |
| [[mutable-state-simpler]] | eventlog-is-truth | loses history |
| [[build-it-they-will-come]] | measure-first | - |
| [[async-by-default]] | sync-first | consider actual I/O patterns first |
| [[rqlite-architecture]] | sync-first | migrated to SQLite |
| [[json-schema-for-validation]] | skills-for-structured-output | separate schema file must stay in sync |
| [[load-everything-upfront]] | progressive-disclosure | wastes context window |
| [[llm-writes-markdown-directly]] | system-owns-format | format discovery is non-deterministic |
| [[premature-optimization]] | error-analysis-over-architecture | optimize after measuring failures |
| [[architecture-first]] | error-analysis-over-architecture | architecture comes after understanding failure modes |
| [[batch-commits]] | commit-early-commit-often | makes bisecting failures harder, obscures reasoning |
| [[commit-noise]] | commit-early-commit-often | detailed history valuable for learning |
| [[gitignore-all-config]] | project-config-in-git | leads to CI/local divergence |
| [[machine-specific-drift]] | project-config-in-git | solved by config sections |

## Personas

| Persona | Facets | Beliefs | Rules |
|---------|--------|---------|-------|
| architect | development-process, design, engineering, llm, data-architecture, rust, tooling, context-management, epistemic, methodology, measurement, git, workflow, devops, configuration, ci | 12 | 3 |

## How to Use

### Adding a Belief (Recommended: Use Skill)

Use the `epistemic-beliefs` skill or `/belief-create` command:

```bash
.claude/skills/epistemic-beliefs/scripts/create-belief.sh \
  --id "belief-id" \
  --statement "One sentence belief" \
  --persona "architect" \
  --confidence "0.80" \
  --evidence "[[source]]: description (weight: 0.X)" \
  --facets "domain1,domain2"
```

The skill will auto-trigger when discussing belief creation. After creation:
1. Edit the file to add supports/attacks relationships
2. Update this index

### Adding a Belief (Manual)

1. Create `beliefs/<belief-id>.md`
2. Fill in frontmatter (type, persona, confidence, entrenchment, status)
3. Write statement
4. Add evidence with weights
5. Link supports/attacks
6. Update this index

### Adding a Rule

1. Create `rules/<rule-id>.md`
2. Fill in frontmatter (type, persona, rule_type, confidence, derived_from)
3. Write conditions (which beliefs)
4. Write conclusion
5. Add exceptions
6. Update this index

### Revision Process (AGM)

1. **Expansion**: New belief arrives → check conflicts → add if consistent
2. **Revision**: New belief conflicts → compare entrenchment → minimize loss
3. **Contraction**: Belief defeated → archive to dust (or scope it)

## References

- [[spec-surface-layer]] - Surface layer spec
- [[spec-epistemic-markdown]] - This layer's design doc (pending)
- [[agm-framework]] - Academic grounding
