---
type: belief
id: boundary-string-internal-enum
persona: architect
facets: [architecture, rust, plugins, wasm]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-24
revised: 2026-02-24
---

# boundary-string-internal-enum

At serialization boundaries (YAML, JSON, WIT, MCP), keep string representations; internally, parse to typed enums immediately at the boundary edge

## Statement

At serialization boundaries (YAML, JSON, WIT, MCP), keep string representations; internally, parse to typed enums immediately at the boundary edge

## Evidence

- [[session-20260224-195035]]: Design review of spec-create type system — three-way analysis (build agent, Gjengset-style review, plugin-architecture review) converged on boundary=string, internal=enum as the standard adapter pattern for WASM/WIT (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-02-24: Created — metrics computed by `patina scrape`
