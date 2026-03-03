---
type: belief
id: probe-emits-dashboard-displays
persona: architect
facets: [architecture, measure, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-03
revised: 2026-03-03
---

# probe-emits-dashboard-displays

Probes emit findings to the event stream; dashboards read events and render for humans. Doctor is a probe (emit-first, display is convenience summary), measure is the dashboard (canonical health view). New health checks go in probes, new display goes in dashboards.

## Statement

Probes emit findings to the event stream; dashboards read events and render for humans. Doctor is a probe (emit-first, display is convenience summary), measure is the dashboard (canonical health view). New health checks go in probes, new display goes in dashboards.

## Evidence

- [[session-20260303-134008]]: [[doctor-probe-clarity]] spec: clarifies doctor as probe-first, emit before display, terminal output is summary with pointer to measure --full (weight: 0.9)

## Supports

- [[unix-philosophy]]: One tool, one job — doctor probes, measure displays. Neither does both.
- [[events-are-autobiography-not-telemetry]]: Doctor writes its autobiography (emit findings), doesn't report to a dashboard.
- [[plugin-is-agent-plus-skill]]: Doctor is a skill (single operation: probe environment, emit).

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `plugins/doctor/src/lib.rs:174` — `measure::record_measurement()` emit call (the probe action)
- `src/commands/measure/internal.rs:512` — `CaptureHealthCheckMetrics` parses doctor's emitted findings (the dashboard action)
- `src/commands/measure/internal.rs:752` — `diagnostics()` renders missing-tools warnings from doctor's emit data

## Revision Log

- 2026-03-03: Created — metrics computed by `patina scrape`
