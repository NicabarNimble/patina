---
type: belief
id: signal-over-noise
persona: architect
facets: [quality, contribution-evaluation, anti-slop]
confidence:
  score: 0.82
entrenchment: medium
status: active
extracted: 2026-01-23
revised: 2026-01-23
---

# signal-over-noise

Noise is generic. Signal engages with project-specific knowledge. Evaluate contributions by whether they demonstrate engagement with captured patterns, beliefs, and sessions.

## Statement

Noise is generic. Signal engages with project-specific knowledge. Evaluate contributions by whether they demonstrate engagement with captured patterns, beliefs, and sessions.

## Evidence

- session-20260123-050814: Exploration of slop/signal problem in open source

## Supports

- [[read-code-before-write]] - Reading existing code is engaging with project-specific knowledge
- [[spec-first]] - Specs capture project-specific understanding before implementation
- [[measure-first]] - Measurement grounds decisions in project-specific evidence

## Attacks

- [[accept-all-contributions]] (status: defeated, reason: "volume without quality degrades projects")
- [[generic-is-good-enough]] (status: defeated, reason: "context-free contributions miss project constraints")

## Attacked-By

- [[inclusive-contribution-bar]] (status: active, confidence: 0.4, scope: "requiring Patina engagement may exclude legitimate new contributors")

## Applied-In

- [[explore/anti-slop/SPEC.md]] - Core thesis of signal/noise exploration
- Issue triage - Scry to check if new issue duplicates or relates to existing work
- PR review - Surface relevant beliefs/patterns for context

## Revision Log

- 2026-01-23: Created (confidence: 0.82)
