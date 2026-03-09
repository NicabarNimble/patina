---
type: belief
id: contracts-before-consumers
persona: architect
facets: [architecture, planning, sequencing]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-08
revised: 2026-03-08
---

# contracts-before-consumers

Within every implementation seam, shared contracts must land before consumer rewiring — a build agent that starts with the obvious consumer hotspot will create duplicate partial models.

## Statement

Within every implementation seam, shared contracts must land before consumer rewiring — a build agent that starts with the obvious consumer hotspot will create duplicate partial models.

## Evidence

- [[session-20260308-210134]] - Outside audit: Seam 2 consumers (scrape, search, enrichment, oxidize) cannot be rewired before the schema model stabilizes — otherwise duplicate partial parsers proliferate. Seam 1 broker routing cannot proceed before transport/spawn runtime surface supports pipe/ingest + child-type sandboxing. (weight: 0.9)
- [[session-20260308-210134]] - Seam 3 lakehouse binary is dead code until Seam 1's runtime contract exists — building it first wastes effort and creates false completion signal. (weight: 0.8)

## Supports

- [[implementation-is-three-seams]] — the intra-seam ordering rule for this belief
- [[spec-driven-design]] — SPECs decide scope; contracts are the spec-level analog
- [[dependable-rust]] — small public interface = stable contract before consumers depend on it

## Attacks

<!-- None identified -->

## Attacked-By

<!-- Could be challenged if a consumer rewrite is small enough to absorb contract changes -->

## Applied-In

- [[spec-pipe-architecture]] §Implementation Gap Analysis — intra-seam ordering documented for all three seams
- [[spec-pipe-architecture]] §Build Agent Traps — five failure modes traced to premature consumer work

## Revision Log

- 2026-03-08: Created — metrics computed by `patina scrape`
