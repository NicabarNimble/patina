---
type: belief
id: mother-owns-destination-format
persona: architect
facets: [architecture, lake, pipe-architecture, connectors]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-08
revised: 2026-03-08
---

# mother-owns-destination-format

Mother owns destination routing and delegates storage to bounded children. Connectors own domain record shape; lakehouse children own storage format (Parquet, future formats). Mother governs — she routes records to the right child, tracks cursor truth, and enforces persona scoping. She never executes data-plane concerns like file writing or format serialization.

## Statement

Mother owns destination routing and delegates storage to bounded children. Connectors own domain record shape; lakehouse children own storage format (Parquet, future formats). Mother governs — she routes records to the right child, tracks cursor truth, and enforces persona scoping. She never executes data-plane concerns like file writing or format serialization.

## Evidence

- [[session-20260308-134326]]: Outside agent review corrected initial design where connector wrote Parquet directly. Second correction: Mother should not write Parquet either — lakehouse child owns storage. Mother governs routing and truth; children execute bounded roles. (weight: 0.9)

## Supports

- [[connectors-own-tables-schemas-are-contracts]] — connectors own domain contracts; Mother invokes capabilities. This belief extends: connectors own domain records, Mother owns destination format.
- [[mother-holds-connections-pipes-transform]] — Mother routes and invokes; children transform. This belief clarifies: routing includes format materialization at the destination.
- [[pipe-protocol-is-transport-agnostic]] — the protocol carries records regardless of destination. Mother handles the last mile (Parquet, SQLite, future).

## Attacks

- Connector-writes-directly — the earlier design where connectors write Parquet themselves. Coupling source capture to storage format makes connectors non-reusable.

## Attacked-By

- Performance concern: Mother as intermediary adds serialization overhead vs connector writing directly. Counter: the overhead is bounded (one Parquet write per batch), and reusability outweighs per-batch cost.

## Applied-In

- [[spec-raw-lake-ingestion]] — Mother routes connector records to lakehouse child via pipe/ingest. Lakehouse child writes Parquet. Mother tracks cursor truth. Connector unchanged from project-scope behavior.

## Revision Log

- 2026-03-08: Created — metrics computed by `patina scrape`
