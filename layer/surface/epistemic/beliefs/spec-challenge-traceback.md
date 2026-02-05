---
type: belief
id: spec-challenge-traceback
persona: architect
facets: [process, architecture, spec]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-04
revised: 2026-02-04
---

# spec-challenge-traceback

When implementation reveals a spec assumption was wrong, stop coding and trace back to the spec before continuing — the spec gap may indicate a deeper design oversight.

## Statement

When implementation reveals a spec assumption was wrong, stop coding and trace back to the spec before continuing — the spec gap may indicate a deeper design oversight.

## Evidence

- [[session-20260204-110139]]: Removing `dimension` from ScryOptions broke eval ablation testing. D0 spec said remove it, but Andrew Ng principle says never throw away measurement capability. Pausing to reconsider prevented a silent regression in eval diagnostics. (weight: 0.9)
- [[d0-unified-search/SPEC.md]]: Spec said "remove --dimension flag" without distinguishing CLI flag removal from internal measurement capability. Implementation exposed the gap. (weight: 0.85)

## Supports

- [[bridges-become-permanent]] — both beliefs address moments where stopping to think prevents compounding debt

## Attacks

- [[read-code-before-write]] — complementary, not conflicting: read code catches implementation gaps, this catches spec gaps

## Attacked-By

- "Just fix it and move on" — pragmatic speed vs spec discipline trade-off. Valid when the gap is trivial, dangerous when the gap indicates a pattern.

## Applied-In

- D0 `dimension` field: spec said remove, implementation revealed eval needed it. Kept as internal-only field with doc comment. The 30-second pause prevented breaking ablation testing.

## Revision Log

- 2026-02-04: Created — metrics computed by `patina scrape`
