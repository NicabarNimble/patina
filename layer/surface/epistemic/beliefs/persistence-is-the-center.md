---
type: belief
id: persistence-is-the-center
persona: architect
facets: [architecture, domain-model, subsystem-design]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-09
revised: 2026-03-09
---

# persistence-is-the-center

Persistence is the center of a subsystem, not acquisition. Build model-out, not feature-in. The domain model is what everything depends on — provider-specific acquisition varies, but the connection record is the stable contract the rest of the system consumes.

## Statement

Persistence is the center of a subsystem, not acquisition. Build model-out, not feature-in. The domain model is what everything depends on — provider-specific acquisition varies, but the connection record is the stable contract the rest of the system consumes.

## Evidence

- [[session-20260309-131917]] - patina-connect design session: external audit agent identified that acquisition varies per provider while persistence (ConnectionRecord) is what broker, CLI, and status all depend on (weight: 0.9)

## Supports

- [[dependable-rust]] — Small, stable public interface. The domain model IS the public interface of a subsystem.
- [[mother-is-connection-and-continuity]] — Mother depends on stable connection contracts, not provider-specific acquisition details.

## Attacks

## Attacked-By

## Applied-In

- [[spec-patina-connect]] DESIGN.md — 9-commit build plan orders model (commit 2) before store (3), before provider (5), before broker integration (6). The domain model is the foundation everything else builds on.

## Revision Log

- 2026-03-09: Created — metrics computed by `patina scrape`
