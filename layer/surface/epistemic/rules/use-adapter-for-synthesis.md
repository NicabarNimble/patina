---
type: rule
id: use-adapter-for-synthesis
persona: architect
rule_type: synthesized
confidence: 0.85
derived_from: [smart-model-in-room, dont-build-what-exists]
status: active
extracted: 2026-01-16
---

# rule: use-adapter-for-synthesis

## Conditions

- [[smart-model-in-room]] (confidence > 0.8)
- [[dont-build-what-exists]] (confidence > 0.7)
- Task requires intelligence (synthesis, conflict resolution, pattern extraction)

## Conclusion

Use the adapter's frontier LLM (Claude/Gemini/OpenCode) for synthesis tasks. Keep deterministic operations in Mother daemon. Defer local model optimization to Phase 6+.

## Rationale

Frontier LLMs are already available through adapters. Building local model infrastructure before proving patterns is premature optimization. The quality gap between frontier and local models is significant for synthesis tasks.

## Exceptions

- [[privacy-sensitive-data]] - Use local model for sensitive synthesis
- [[offline-required]] - Local model when no network
- [[proven-pattern]] - Local model for well-understood, repetitive synthesis (Phase 6+)

## Applied-In

- Mother architecture - deterministic daemon, not local LLM
- Surface layer Phase 3 - adapter synthesis
- Uncertainty queue - routed to adapter for review

## Threshold

Only move to local model when:
- Pattern is proven (>100 successful applications)
- Quality is validated (adapter vs local comparison)
- Cost justifies infrastructure

## Revision Log

- 2026-01-16: Synthesized from Mother architecture decisions
