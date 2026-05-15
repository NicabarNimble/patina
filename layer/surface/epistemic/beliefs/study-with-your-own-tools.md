---
type: belief
id: study-with-your-own-tools
persona: architect
facets: [methodology, dogfooding, validation, research]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-05-14
revised: 2026-05-14
---

# study-with-your-own-tools

When a project studies itself — or any externally-observable work — it should consume its own toolchain as an external consumer would, not by reaching into internal modules. The study then doubles as a real-world validation of the tools on a workload outside their home use case.

## Statement

Research projects that analyze a codebase have a natural temptation to bypass that codebase's user-facing tools and read internal structures directly. Resisting that temptation produces two compounding benefits: (1) the methodology section of the paper writes itself, because the tools used are publicly documented commands rather than internal symbols; (2) any rough edge the research project hits is a real-world bug report against the toolchain, validating that the tools work for non-internal users. The study and the dogfooding are the same activity.

## Evidence

- [[session-20260514-073518]]: Designed an external research project to analyze the patina session corpus by consuming patina's own Scry, Assay, Oxidize, Scrape, and tree-sitter tooling as command-line consumers. The research project becomes a third-party-style user of patina without ever calling its internals — and the resulting paper has a methodology section grounded in publicly-reproducible tool invocations.

## Supports

- [[observational-research-is-read-only]] — using a project's public tools enforces the read-only posture by construction; internal-API access would invite write-back paths.

## Applied-In

- Planned external research repo for patina session corpus (v0 deliberately defers tool integration to prove the extraction loop end-to-end first, then layers in `patina scrape`, `patina assay`, `patina scry`, `patina oxidize` as external consumers) — see [[session-20260514-073518]].

## Revision Log

- 2026-05-14: Created during research-project design session. Validation pending — principle applied but not yet exercised by v0 scaffold.
