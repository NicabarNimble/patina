---
type: belief
id: core-primitives-are-not-children
persona: architect
facets: [architecture, mother, children]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-22
revised: 2026-03-22
---

# core-primitives-are-not-children

Patina's knowledge primitives (scry, scrape, assay, belief, measure, oxidize) are core Mother capabilities, not child-provided; children are pluggable strategy providers that feed INTO core.

## Statement

Patina's knowledge primitives (scry, scrape, assay, belief, measure, oxidize) are core Mother capabilities, not child-provided; children are pluggable strategy providers that feed INTO core.

## Evidence

- [[session-20260321-164003-365905000]] - User correction during pre-v1 build review: build agent incorrectly pushed core primitives behind daemon-mediated children (weight: 1.0)

## Supports

- [[mother-is-the-daemon]] — Mother IS the knowledge system; core primitives are her native capabilities
- [[children-have-agency-toys-are-capabilities]] — children have bounded agency within Mother's sandbox, but they don't replace Mother's own primitives

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- The pre-v1 build implicitly treated primitives as daemon-mediated child capabilities — this belief corrects that framing

## Applied-In

- Reframes zero-fallback cutover: the question isn't "make daemon stubs real" but "what's core vs what's pluggable strategy"
- scrape should have its own strategy children (code-scraper, github-scraper, markdown-scraper) — not every project is code-based
- DuckLake is a scrape strategy for GitHub data, not a standalone knowledge child

## Revision Log

- 2026-03-22: Created — metrics computed by `patina scrape`
