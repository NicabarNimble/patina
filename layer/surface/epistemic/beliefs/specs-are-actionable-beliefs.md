---
type: belief
id: specs-are-actionable-beliefs
persona: architect
facets: [spec-system, beliefs, governance, workflow]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-15
revised: 2026-02-15
---

# specs-are-actionable-beliefs

A spec is an actionable belief — an assertion that something should be built, backed by evidence, with exit criteria that define 'done'. If a spec isn't actionable, it's not a spec — it's an idea, and ideas belong somewhere lighter. The spec pipeline gets clogged when it doubles as an idea factory. Specs should be beliefs you're ready to act on.

## Statement

A spec is an actionable belief — an assertion that something should be built, backed by evidence, with exit criteria that define 'done'. If a spec isn't actionable, it's not a spec — it's an idea, and ideas belong somewhere lighter. The spec pipeline gets clogged when it doubles as an idea factory. Specs should be beliefs you're ready to act on.

## Evidence

- [[session-20260215-075638]]: Spec staleness mirrors belief staleness — both accumulate entries that go stale. The explore/feat/refactor taxonomy tries to separate ideas from work but ideas still clog the ready queue. The knowledge-protocol explore itself demonstrated: an explore that resolves to 'no' is valuable, but the overhead of a full spec for every idea is not.

## Supports

- [[beliefs-are-the-product]] — if beliefs are the product, specs are the actionable subset
- [[spec-driven-design]] — specs authorize action, but only if they're actually actionable

## Attacks

- The current explore spec type — explores are idea investigation, not actionable work. Valuable but overweight for "I wonder if..." questions.

## Attacked-By

- [[spec-driven-design]] (status: active, scope: "explores and design specs serve a real purpose for de-risking" — tension between lightweight ideas and structured investigation)

## Applied-In

- knowledge-protocol explore: proved explores CAN work (Outcome C is a valid result) but the overhead of a full SPEC.md for a question that resolved in one code-reading session is high
- Spec staleness: ready queue accumulated specs that weren't truly actionable

## Revision Log

- 2026-02-15: Created — metrics computed by `patina scrape`
