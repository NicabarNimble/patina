---
type: belief
id: patina-tools-are-patina-interface
persona: architect
facets: [tooling, workflow, agentic]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-03
revised: 2026-03-03
---

# patina-tools-are-patina-interface

Patina's own tools (scry, assay, context) are the canonical way to navigate the codebase during development — raw file reads are a fallback, not the default

## Statement

Patina's own tools (scry, assay, context) are the canonical way to navigate the codebase during development — raw file reads are a fallback, not the default

## Evidence

- [[session-20260303-111006]]: `assay functions src/paths.rs` found the missing `pipeline_dir()` in one call; incremental file reads across 700-line files missed it entirely and led to copying a hardcoded anti-pattern that the user had to correct (weight: 0.9)
- [[session-20260303-111006]]: `assay search "pipeline_dir plugin directory path"` surfaced the `pipeline_dir` function in `setup/grammars.rs`, the `plugin` module in `paths.rs`, and the commit that added plugin paths — all in one query, zero file reads (weight: 0.8)

## Supports

- [[dependable-rust]]: The tools ARE the public interface to the codebase — using them is using the module's API, not reaching into internals
- [[unix-philosophy]]: Use the right tool for the job — `assay` for structural queries, `scry` for semantic search, not `cat` for both

## Attacks

- The assumption that reading source files directly is always faster or more accurate than querying structured indexes

## Attacked-By

- Tools may be stale if scrape hasn't run recently — raw reads are always current
- Novel code not yet indexed won't appear in assay/scry results

## Applied-In

- `src/paths.rs` — `pipeline_dir()` gap discovered via `assay functions` after raw reads failed to surface it across 3 files with hardcoded paths
- `src/commands/scrape/code/extract_v2.rs` — lazy plugin loading plumbing would have been planned faster with `assay callers` to trace the full call chain before editing signatures

## Revision Log

- 2026-03-03: Created — metrics computed by `patina scrape`
