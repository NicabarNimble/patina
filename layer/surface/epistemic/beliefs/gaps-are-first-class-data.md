---
type: belief
id: gaps-are-first-class-data
persona: architect
facets: [data, schema, observability, methodology]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-05-14
revised: 2026-05-14
---

# gaps-are-first-class-data

In an observability or research dataset, missing data is data. Gaps belong in their own rows with a type, a source, and a detecting rule — not in silent NULLs that interpretation has to reconstruct downstream.

## Statement

When extracting facts from a heterogeneous corpus (session artifacts, logs, mixed-era metadata), the schema must record what is absent as explicitly as what is present. A `gaps` (or `observations`) table with `(thing_id, gap_type, detected_by, observed_at)` turns "60% of sessions have no narrative body" into a query against real rows, not an editorial claim that has to be re-derived from NULL counts. This makes the dataset honest under peer review and lets pipeline improvements (a new gap detector) become additive rather than retroactive.

## Evidence

- [[session-20260514-073518]]: Audited `src/commands/scrape/layer/sessions.rs` and found multiple frontmatter fields silently dropped (`llm`, `status`, `participants`, `git.starting_commit`, `git.start_tag`, `git.end_tag`). Without explicit gap tracking, downstream analytics cannot distinguish "field never existed in source" from "field existed but wasn't extracted" — both look like NULL.

## Supports

- [[observational-research-is-read-only]] — gaps must be recorded rather than backfilled into the source.
- [[verify-claims-against-reality]] — claims about the dataset's coverage become queryable instead of asserted.

## Applied-In

- Planned research DB schema for patina session corpus — includes a dedicated gap table with `gap_type` taxonomy (missing-llm, missing-end-tag, empty-context, unreferenced-commits, etc.) per [[session-20260514-073518]].

## Revision Log

- 2026-05-14: Created during research-project design session. Validation pending — principle applied but not yet exercised by v0 scaffold.
