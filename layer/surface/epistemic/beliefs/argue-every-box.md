---
type: belief
id: argue-every-box
persona: architect
facets: [architecture, decision-making]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# argue-every-box

Before adding any component to a system, be able to argue both FOR and AGAINST it — if you can only argue for it, you don't understand it well enough to make the decision

## Statement

Before adding any component to a system, be able to argue both FOR and AGAINST it — if you can only argue for it, you don't understand it well enough to make the decision

## Evidence

- [[session-20260205-084522]]: Jerry Nixon NDC Copenhagen 2025 talk: "Once you understand it enough to understand why you DON'T want it, then you finally have enough why you would actually want it" (weight: 0.9)
- External: Jerry Nixon "Modern Architecture 101 for New Engineers & Forgetful Experts" - NDC Copenhagen 2025 (weight: 0.8)

## Supports

- [[simplicity-is-architecture]] — arguing against forces you to identify what's essential
- [[measure-first]] — can't argue against without understanding trade-offs

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Analysis paralysis — arguing both sides endlessly without deciding

## Applied-In

- [[system-introspection]] SPEC: Created "Argue-Every-Box Test" table evaluating components (Data Contracts, Introspect Command, Experiment Infrastructure, Contract Verification, OTEL Tracing)
- OTEL decision: Argued FOR (debug production) and AGAINST (no prod users, complexity) → deferred
- Cache decision: Argued FOR (transformative performance) and AGAINST (no users) → deferred

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
