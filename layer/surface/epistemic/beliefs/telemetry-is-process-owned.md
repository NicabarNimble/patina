---
type: belief
id: telemetry-is-process-owned
persona: architect
facets: [architecture, telemetry, mother, children]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-04-02
---

# telemetry-is-process-owned

Each process (Mother, child, project) owns its own telemetry capture. Shared substrate enforces policy but does not emit telemetry. Mother federates access to telemetry rather than being the universal sink.

## Statement

Each process (Mother, child, project) owns its own telemetry capture. Shared substrate enforces policy but does not emit telemetry. Mother federates access to telemetry rather than being the universal sink.

## Evidence

- [[session-20260310-074810]]: Emerged during early child-architecture exploration: autonomous child model means each child captures its own metrics. HTTP proxy extraction audit confirmed shared proxy should not carry telemetry hooks. Outside agent validated: "telemetry should follow ownership boundaries". (weight: 0.9)

## Supports

- [[mother-holds-connections-pipes-transform]] — Mother orchestrates, doesn't centralize execution
- [[patina-is-domain-agnostic-knowledge-system]] — telemetry format varies by domain; process-local ownership avoids forcing a universal schema

## Attacked-By

- Fragmented metrics risk: without a thin common telemetry contract for querying across processes, each child can become an island. Mitigated by Mother's ability to query child telemetry on demand via shared observation surfaces.

## Applied-In

- [[http-proxy-extraction]] — shared proxy enforces security policy, does not emit telemetry; broker wrapper adds its own measurement
- `src/measure.rs` and `src/child/internal/mod.rs` — source/process ownership checks keep telemetry tied to emitting actor boundaries
- [[child-construction-canon]] — canon children own their own telemetry while Mother owns federation/observation access

## Revision Log

- 2026-03-10: Created — metrics computed by `patina scrape`
- 2026-04-02: Revised — retained origin context as historical, removed ducklake-specific current anchors, and re-anchored to current measurement surfaces.
