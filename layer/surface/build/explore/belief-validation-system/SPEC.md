---
type: explore
id: belief-validation-system
status: design
created: 2026-01-21
updated: 2026-02-06
sessions:
  origin: 20260206-060219
related:
  - layer/surface/build/feat/mother-delivery/d1-belief-oracle/SPEC.md
beliefs:
  - measure-the-measurement
  - practical-memory-over-epistemic-formalism
---

# explore: Computed Belief Confidence

> Belief confidence should be derived from verifiable data, not assigned by judgment.

## Problem

Belief confidence is currently a human-assigned number with no verification. The system
has 68 beliefs but no way to know which ones are well-supported vs. aspirational.

## What We Already Have

| Infrastructure | What It Provides |
|----------------|------------------|
| `belief_code_reach` table | Which code files a belief applies to |
| `belief_fts` table | Full-text search over belief statements |
| BeliefOracle | Hybrid vector + FTS5 retrieval |
| E4.5 verification queries | 25 queries connecting beliefs to DB facts |
| Session archives | 500+ files of evidence with wikilinks |
| Git history | Timestamps for all belief creation/modification |

## Verification Layers (Exploration)

| Layer | Signal | Feasibility |
|-------|--------|-------------|
| Link integrity | Do evidence wikilinks resolve to real files? | Easy — file existence check |
| Code reach | Does the belief actually reach code via `belief_code_reach`? | Easy — already computed |
| Semantic support | Does scry find supporting evidence for the belief statement? | Medium — use BeliefOracle |
| Graph support | How many other beliefs cite this one (in_degree)? | Medium — graph query |
| Temporal health | Is the belief still referenced in recent sessions? | Easy — grep + git log |
| Contradiction detection | Do any beliefs conflict? | Hard — semantic similarity threshold |

## Open Questions

1. Should confidence auto-update on scrape, or compute on demand?
2. What's the minimum evidence threshold for a belief to be considered "verified"?
3. How does this relate to eval-repair's product metric work?
4. Is the value in computed confidence, or just in detecting *broken* beliefs (link rot, unreachable code)?

## Previous Work

- Prolog neuro-symbolic system attempted this (removed — parallel architecture, wrong approach)
- E4.5 belief verification proved the pattern: connect claims to DB facts
- eval-repair spec addresses belief MRR (ranking quality, not confidence quality)

## Moved From

`deferred/belief-validation-system/` — original spec was stale (bash scripts, dead Prolog
references). Concept preserved, implementation details stripped.
