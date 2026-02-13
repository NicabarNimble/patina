---
type: feat
id: plugin-pipeline-world
status: design
created: 2026-02-13
sessions:
  origin: 20260213-120746
blocked_by:
- plugin-task-world
related:
- layer/surface/build/feat/plugin-ecosystem/SPEC.md
beliefs:
- separate-worlds-for-isolation
- patina-is-knowledge-protocol
---

# feat: Pipeline World (`patina:pipeline`)

> Host-invoked pure-compute plugins. Grammar parsers, chunkers, tokenizers.
> The plugin is a pure function — all side effects stay in the host.

## Problem

Grammar parsing (tree-sitter), chunking, and custom scrapers are compiled
into the binary. Adding a new language or scraper means recompiling patina.
A pipeline world lets community plugins extend the knowledge pipeline
without touching core code.

## Parent Design

Build order item #4 from [[plugin-ecosystem]] SPEC.md. Pipeline world WIT
and `handle(json)` dispatch pattern are defined there (lines 559-588).
Subsumes the archived `plugin-oracle-scraper` and `plugin-grammars` specs.

## Scope

### WIT (from ecosystem spec, locked)

```wit
world pipeline {
    import patina:host/log@0.1.0;

    export init: func();
    export name: func() -> string;
    export handle: func(input: string) -> result<string, string>;
}
```

`log` is the **only** import. No query, no layer, no HTTP, no toys.
Compile-time isolation per [[separate-worlds-for-isolation]].

### Key Architecture Decisions

- Single `handle(json)` dispatch — avoids WIT growing-exports problem
- Versioned envelope: `{"op": "parse", "version": 1, "payload": {...}}`
- Host invokes pipeline plugins during scrape/index operations
- Oracle stays host-side (resolved in ecosystem spec) — not this world
- `PipelineEngine` in `src/plugin/internal/pipeline.rs`
- Guest crate: `patina-pipeline-api` with typed `PipelineOp` enum

### Conformance Test

`echo-pipeline` — handles `{"op":"echo"}`, returns payload unchanged.
Proves: envelope parsing, manifest `[provides]` gating, host refuses
unknown ops.

### Dependencies

- Task world (build order #3) proves the third engine pattern
- HTTP interface (build order #2) not needed — pipeline has no HTTP

## Exit Criteria

- [ ] `wit/pipeline/pipeline.wit` with log-only import
- [ ] `PipelineEngine` in `src/plugin/internal/`
- [ ] Host-side integration: scrape pipeline can invoke pipeline plugins
- [ ] Versioned envelope schema validated at boundary
- [ ] Guest API crate with `PipelineOp` typed enum
- [ ] Conformance test: `echo-pipeline`
- [ ] Pre-push checks pass

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Extracted from [[plugin-ecosystem]] build order item #4. Blocked by task world. |
