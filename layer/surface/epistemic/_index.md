---
type: index
updated: 2026-01-16
---

# Epistemic Layer Index

This directory contains Patina's epistemic belief system - atomic beliefs with evidence, support/attack relationships, and derived rules.

## Statistics

- **Beliefs**: 6
- **Rules**: 3
- **Total Confidence**: 5.31/6.0 (avg: 0.885)
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
          │         └──────────┐
          │                    │
          ▼                    ▼
   [[smart-model-in-room]] ◄───┘
         (0.88)
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
| [[sync-first]] | 0.88 | high | active |
| [[spec-first]] | 0.85 | high | active |
| [[dont-build-what-exists]] | 0.90 | high | active |
| [[smart-model-in-room]] | 0.88 | high | active |
| [[eventlog-is-truth]] | 0.92 | very-high | active |
| [[measure-first]] | 0.88 | high | active |

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
| [[cost-concerns]] | smart-model-in-room | active | "high-volume synthesis" |
| [[latency-concerns]] | smart-model-in-room | active | "real-time interactions" |
| [[storage-overhead]] | eventlog-is-truth | scoped | "ref repos use lean storage" |
| [[measurement-overhead]] | measure-first | active | "trivial changes" |
| [[high-concurrency-needed]] | sync-first | active | "network-heavy or many parallel connections" |
| [[streaming-responses]] | sync-first | active | "long-running streaming APIs" |

## Defeated Beliefs

| Belief | Defeated By | Reason |
|--------|-------------|--------|
| [[move-fast-break-things]] | spec-first | leads to rework |
| [[not-invented-here]] | dont-build-what-exists | - |
| [[local-model-always]] | smart-model-in-room | quality gap too large |
| [[mutable-state-simpler]] | eventlog-is-truth | loses history |
| [[build-it-they-will-come]] | measure-first | - |
| [[async-by-default]] | sync-first | consider actual I/O patterns first |
| [[rqlite-architecture]] | sync-first | migrated to SQLite |

## Personas

| Persona | Facets | Beliefs | Rules |
|---------|--------|---------|-------|
| architect | development-process, design, engineering, llm, data-architecture, rust | 6 | 3 |

## How to Use

### Adding a Belief

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
