---
type: belief
id: value-is-boundary-not-domain
persona: architect
facets: [rust, type-safety, serde, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-01
revised: 2026-03-01
---

# value-is-boundary-not-domain

serde_json::Value in a non-boundary struct field is deferred type debt that compounds via copy-paste — if you're writing serde_json::json\!({...}) and you know the field names, you already have the struct definition, just write it down.

## Statement

serde_json::Value in a non-boundary struct field is deferred type debt that compounds via copy-paste — if you're writing serde_json::json\!({...}) and you know the field names, you already have the struct definition, just write it down.

## Evidence

- [[session-20260301-191035]]: measure/internal.rs used Value as domain state for SourceSummary.latest_metrics, causing 22+ .get().as_*().unwrap_or() chains that silently hide typos and missing data. 5 construction sites erased already-typed SQL results into Value, then 22 consumption sites re-extracted them through fallible string-keyed access. (weight: 0.95)

## Supports

- [[parse-at-boundary-type-the-interior]] — this belief is a specific instance: Value is the boundary type, typed structs are the interior
- [[silent-default-hides-missing-data]] — Value chains require `.unwrap_or()` which silently defaults; typed structs use `Option<T>` to make absence explicit

## Attacks

<!-- None known -->

## Attacked-By

- Prototyping speed: Value lets you iterate on JSON shapes without recompiling structs — mitigated by the fact that struct changes are fast in Rust (compiler catches all consumers) while Value typos are silent at runtime
- Heterogeneous data: when a single field must hold genuinely different shapes, Value seems natural — mitigated by Rust enums, which are exactly the typed equivalent of "one of several shapes"

## Applied-In

- [[type-measure-domain]] spec — replaces `SourceSummary.latest_metrics: serde_json::Value` with `VerbMetrics` enum carrying 7 typed struct variants

## Revision Log

- 2026-03-01: Created — metrics computed by `patina scrape`
