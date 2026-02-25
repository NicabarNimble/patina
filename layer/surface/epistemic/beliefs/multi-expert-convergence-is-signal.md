---
type: belief
id: multi-expert-convergence-is-signal
persona: architect
facets: [methodology, review, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-25
revised: 2026-02-25
---

# multi-expert-convergence-is-signal

When 3+ independent expert perspectives flag the same issue, it is strong signal of genuine architectural concern — single-expert findings tend toward preference, convergent findings tend toward design debt.

## Statement

When 3+ independent expert perspectives flag the same issue, it is strong signal of genuine architectural concern — single-expert findings tend toward preference, convergent findings tend toward design debt.

## Evidence

- [[session-20260225-104204]]: Spec system audit (2026-02-25): DB/filesystem dual-truth flagged by Gjengset (type drift), Ng (freshness), Sutton (two representations) independently — promoted to [[spec-query-filesystem-truth]]. Untyped next_spec_value flagged by Gjengset (type safety) and Sutton (inconsistency) — promoted to [[spec-next-typed]]. Single-expert findings (F-G1 untagged enum, F-Y4 positional arg) stayed at Low severity. (weight: 0.9)

## Supports

- [[specs-ship-features-audits-ship-quality]] — convergent review is the mechanism that makes audits produce quality
- [[stale-context-is-hostile-context]] — convergent findings surface stale patterns that single perspectives miss

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Convergence could be groupthink if the expert perspectives share the same training data or biases — mitigated by choosing experts with genuinely different value systems (type safety vs measurement vs simplicity vs platform DX)

## Applied-In

- [[spec-system-audit-2026-02]]: 4 findings flagged by 3+ experts became top-priority actionable specs; 10 single-expert findings stayed as Low-severity notes in the explore spec
- [[session-20260214-061751]]: "Three convergent analyses are evidence, not opinion" — internal audit + 2 outside agents independently prioritized the same top-3

## Revision Log

- 2026-02-25: Created — metrics computed by `patina scrape`
