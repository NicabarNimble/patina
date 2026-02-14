---
type: belief
id: parser-agnostic-interfaces
persona: architect
facets: [architecture, plugin-system]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# parser-agnostic-interfaces

Design plugin interfaces that are agnostic to the implementation technology behind them. The pipeline world's handle(request) → result doesn't know or care about tree-sitter vs cairo-lang-parser — this generality is what lets it serve all 9 grammars despite different parser backends.

## Statement

Design plugin interfaces that are agnostic to the implementation technology behind them. The pipeline world's handle(request) → result doesn't know or care about tree-sitter vs cairo-lang-parser — this generality is what lets it serve all 9 grammars despite different parser backends.

## Evidence

- [[session-20260214-130235]]: [[grammar-extraction]] — 8 tree-sitter grammars and 1 Cairo grammar (cairo-lang-parser, pure Rust) all fit the same pipeline plugin interface because handle() takes source code as string and returns structured JSON. Parser technology is invisible to the host. (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
