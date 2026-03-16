---
type: belief
id: raw-lake-is-capture-contract-first
persona: architect
facets: [architecture, lake, data-architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-08
revised: 2026-03-08
---

# raw-lake-is-capture-contract-first

Raw lake's primary concern is the capture contract: what gets written, where, with what guarantees (append-only, idempotent, provenance-tracked). Query path (DuckDB on Parquet) is a verification tool, not the spec's main purpose. Don't let 'minimal query path' become the design center.

## Statement

Raw lake's primary concern is the capture contract: what gets written, where, with what guarantees (append-only, idempotent, provenance-tracked). Query path (DuckDB on Parquet) is a verification tool, not the spec's main purpose. Don't let 'minimal query path' become the design center.

## Evidence

- [[session-20260308-134326]]: [[session-20260308-134326]] - Outside agent review identified that v1 litmus test step 6 (query) was pulling spec focus away from the capture contract. Corrected: spec should primarily define what a connector writes, where it writes it, how sync/cursor/provenance are tracked, and what guarantees hold. (weight: 0.9)

## Supports

- [[events-are-autobiography-not-telemetry]] — raw lake is the project's record of what it observed from external sources. Capture fidelity matters more than query convenience.
- [[mother-owns-destination-format]] — Mother handles format; the capture contract defines what the connector provides, not how it's stored.

## Attacks

- Query-first design — the approach where lake design starts from "what queries should work?" This leads to curated-layer concerns leaking into raw-zone design. Raw zone should be append-only capture; query optimization belongs in the curated zone.

## Attacked-By

- Usability concern: if raw lake is hard to query, users won't adopt it. Counter: DuckDB reads Parquet natively with zero setup. The query path exists, it's just not the design center.

## Applied-In

- [[spec-raw-lake-ingestion]] — SPEC.md explicitly labels query path as "verification, not primary concern." Exit criteria focus on capture: lake registration, Parquet write, dedup, cursor tracking, provenance.

## Revision Log

- 2026-03-08: Created — metrics computed by `patina scrape`
