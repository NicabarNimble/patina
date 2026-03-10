---
type: belief
id: telemetry-is-process-owned
persona: architect
facets: [architecture, telemetry, mother, children]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-03-10
---

# telemetry-is-process-owned

Each process (Mother, child, project) owns its own telemetry capture. Shared substrate enforces policy but does not emit telemetry. Mother federates access to telemetry rather than being the universal sink.

## Statement

Each process (Mother, child, project) owns its own telemetry capture. Shared substrate enforces policy but does not emit telemetry. Mother federates access to telemetry rather than being the universal sink.

## Evidence

- [[session-20260310-074810]]: [[session-20260310-074810]] - Emerged from DuckLake spec design: autonomous child model means each child captures its own metrics. HTTP proxy extraction audit confirmed shared proxy should not carry telemetry hooks. Outside agent validated: 'telemetry should follow ownership boundaries' (weight: 0.9)

## Supports

- [[mother-holds-connections-pipes-transform]] — Mother orchestrates, doesn't centralize execution
- [[patina-is-domain-agnostic-knowledge-system]] — telemetry format varies by domain; process-local ownership avoids forcing a universal schema

## Attacked-By

- Fragmented metrics risk: without a thin common telemetry contract for querying across processes, each child can become an island. Mitigated by Mother's ability to query child telemetry on demand (e.g., DuckLake's `_sync_cursors` table).

## Applied-In

- [[http-proxy-extraction]] — shared proxy enforces security policy, does not emit telemetry; broker wrapper adds its own measurement
- [[ducklake]] — DuckLake child tracks ingest metrics, cursor state, and errors inside its own DuckLake catalog; Mother queries on demand via `patina mother status`

## Revision Log

- 2026-03-10: Created — metrics computed by `patina scrape`
