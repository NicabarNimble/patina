---
type: belief
id: observational-research-is-read-only
persona: architect
facets: [research, methodology, observability, provenance]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-05-14
revised: 2026-05-14
---

# observational-research-is-read-only

Observational research projects that study a source codebase must never write back to it. The source's own mutability — captured in git history — is observable signal, not contamination to be defended against. The research project is a sibling consumer, never a co-author.

## Statement

When building a derived dataset, analytical projection, or research artifact on top of a working codebase, the research project must be a strictly read-only consumer of the source. Backfilling missing fields, rewriting drifted links, or normalizing historical artifacts destroys the timeline that makes the data valuable in the first place. The source repo's git history is the audit log of all real changes; the research project pins commits and observes.

## Evidence

- [[session-20260514-073518]]: Designed a separate research repo to ingest the 1075-file `layer/sessions/` corpus into its own DuckDB without modifying source artifacts. Patina's normal operational mutability of session files (`session-update.sh`, `session-end.sh`, archival flag flips) becomes a data dimension, not a defect — observable via `git log -- layer/sessions/<file>`.

## Supports

- [[git-is-the-knowledge-substrate]] — git already provides the content-addressed timeline; the research project pins commits rather than building a parallel snapshot layer.
- [[gaps-are-first-class-data]] — read-only posture forces missing data to be recorded as observed-absent rather than silently backfilled.

## Applied-In

- Planned external research repo for patina session corpus analysis (v0 scope captured in [[session-20260514-073518]]).

## Revision Log

- 2026-05-14: Created during research-project design session. Validation pending — principle applied but not yet exercised by v0 scaffold.
