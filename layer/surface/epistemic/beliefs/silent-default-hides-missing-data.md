---
type: belief
id: silent-default-hides-missing-data
persona: architect
facets: [rust, error-handling, scoring, data-integrity]
entrenchment: high
status: active
endorsed: true
extracted: 2026-03-01
revised: 2026-03-01
---

# silent-default-hides-missing-data

`.unwrap_or(0.0)` makes "uncomputed" indistinguishable from "actually zero." When a score, metric, or count can legitimately be absent, use `Option<T>` to preserve the distinction. Silent defaults corrupt rankings, hide data loss, and make debugging impossible.

## Statement

`.unwrap_or(0.0)` makes "uncomputed" indistinguishable from "actually zero." When a score, metric, or count can legitimately be absent, use `Option<T>` to preserve the distinction. Silent defaults corrupt rankings, hide data loss, and make debugging impossible.

## Evidence

- [[session-20260301-165723]]: Belief `grounding_score` in `mother/graph.rs` defaults NULL to 0.0 — beliefs with missing grounding are silently treated as having zero semantic grounding, indistinguishable from beliefs that were computed and scored zero. (weight: 0.95)
- [[session-20260301-165723]]: `embeddings/database.rs` `has_embeddings()` uses `.unwrap_or(0)` on a COUNT query — returns false (no embeddings) when the query itself fails (DB unreachable), a truthfulness failure. (weight: 0.9)
- [[session-20260301-165723]]: 193 occurrences of `.unwrap_or(0.0)`, `.unwrap_or(0)`, `.unwrap_or("")` found in non-test code, concentrated in scoring and metric paths. (weight: 0.85)

## Supports

- [[question-mark-on-option-is-silent-swallower]] — same class of silent failure; `?` on Option and `.unwrap_or()` both erase the distinction between "absent" and "default value"

## Attacks

<!-- None known -->

## Attacked-By

- Convenience: `Option<f64>` requires match/map at every use site — mitigated by designing consumers to handle `None` explicitly, which is the whole point

## Applied-In

<!-- Not yet applied — requires spec work -->

## Revision Log

- 2026-03-01: Created from structural audit findings
