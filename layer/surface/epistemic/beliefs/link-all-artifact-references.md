---
type: belief
id: link-all-artifact-references
persona: architect
facets: [knowledge-graph, session-workflow, traceability]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-02
revised: 2026-02-02
---

# link-all-artifact-references

Session files must use wikilinks for all artifact references — unlinked mentions are invisible to the knowledge graph

## Statement

Session files must use wikilinks for all artifact references — unlinked mentions are invisible to the knowledge graph

## Evidence

- [[session-20260202-155143]]: Audit of 4 recent session files found specs, beliefs, and commits referenced as plain text with no wikilinks — invisible to the knowledge graph (weight: 0.95)
- `src/commands/scrape/beliefs/mod.rs`: `verify_evidence_section()` only counts `[[wikilinks]]` as verified evidence; plain text references produce evidence_verified = 0 (weight: 0.90)
- [[session-20260202-151214]]: References "evaluation-next.md", "system-owns-format", "parse_belief_file" as plain text — none traceable by scraper (weight: 0.85)

## Supports

- [[eventlog-is-truth]] — if the eventlog is truth, references to it must be machine-readable
- [[signal-over-noise]] — wikilinks are signal, plain text mentions are noise

## Attacks

- [[move-fast-break-things]] (status: defeated, reason: adding `[[` takes 2 characters and makes the reference permanent)

## Attacked-By

- [[time-pressure]] (status: active, confidence: 0.3, scope: "LLM-generated prose defaults to plain text; requires deliberate convention")

## Applied-In

- Session [[session-20260202-155143]]: retroactively added wikilinks to activity log — `[[epistemic-layer]]`, `[[epistemic-e4.6c]]`, `[[eventlog-is-infrastructure]]`, `[[commit-09e2abbf]]`, file paths to modified source files
- Belief evidence sections: all beliefs use `[[session-ID]]` wikilinks, verified by scraper

## Revision Log

- 2026-02-02: Created — metrics computed by `patina scrape`
