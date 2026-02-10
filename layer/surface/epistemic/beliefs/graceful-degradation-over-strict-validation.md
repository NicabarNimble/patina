---
type: belief
id: graceful-degradation-over-strict-validation
persona: architect
facets: [architecture, belief-system, resilience]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# graceful-degradation-over-strict-validation

The system should always return something useful, never crash on incomplete data. Broken evidence links degrade to audit warnings, not errors. Nil sentinels over null crashes — incomplete beliefs participate in retrieval and surface their incompleteness through metrics, not through failure.

## Statement

The system should always return something useful, never crash on incomplete data. Broken evidence links degrade to audit warnings, not errors. Nil sentinels over null crashes — incomplete beliefs participate in retrieval and surface their incompleteness through metrics, not through failure.

## Evidence

- [[session-20260209-120229]]: [[session-20260209-061005]] - LOATs stream analysis: Anton's nil sentinel pattern (index 0 = valid zero-initialized value) means dereferencing a broken reference never crashes — it returns a zeroed struct. Patina already follows this: broken wikilinks degrade to evidence_verified=0, ungrounded beliefs get 'floating' warnings, scry always returns results even for poorly-formed beliefs. The system makes bad states visible rather than preventing operation. (weight: 0.9)

## Supports

- [[anti-tunneling-as-belief-challenge]]: Beliefs bubble up with minimal resistance because the system degrades gracefully on incomplete beliefs rather than rejecting them
- [[practical-memory-over-epistemic-formalism]]: Capture first, formalize later — works because the system handles informal/incomplete beliefs without breaking
- [[progressive-disclosure]]: Incomplete data surfaces as warnings at the appropriate level, not as blocking errors

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Silent degradation can mask real problems — if broken wikilinks always return nil, you might never fix them. Counter: audit warnings make bad states visible on demand; the degradation is graceful but not silent.

## Applied-In

- **`scry` retrieval**: Always returns results, even for queries that partially match. No query fails — worst case is low-relevance results
- **`belief audit` warnings**: `no-evidence`, `floating`, `unverified`, `verify-contested` — all warnings, never errors. The audit surfaces problems without blocking usage
- **`get` returns nil, not crash**: `things.get(player_idx)` returns nil thing when player is dead/gone — same pattern as Anton's nil sentinel. Patina's `patina context` returns beliefs ranked by relevance even when grounding data is incomplete
- **Evidence wikilink resolution**: Broken `[[wikilinks]]` degrade to `evidence_verified: 0` — the belief still participates in retrieval, just with lower truth metrics

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
