---
type: rule
id: capture-at-boundary
persona: architect
rule_type: synthesized
confidence: 0.88
derived_from: [eventlog-is-truth]
status: active
extracted: 2026-01-16
---

# rule: capture-at-boundary

## Conditions

- [[eventlog-is-truth]] (confidence > 0.8)
- Data crosses a non-deterministic boundary

## Conclusion

When data crosses a non-deterministic boundary (LLM synthesis, user curation, external API), capture the output in an eventlog. The output becomes a new source of truth that cannot be re-derived.

## Rationale

Helland's principle: non-deterministic operations produce outputs that cannot be regenerated identically. An LLM might produce different output; user edits are original knowledge. These must be captured, not derived.

## Boundaries Requiring Capture

| Boundary | Why Non-Deterministic | Eventlog |
|----------|----------------------|----------|
| LLM synthesis | Token sampling, model version | L2 surface.synthesize.* |
| User curation | Human judgment | L2 surface.curate.* |
| External API | Rate limits, availability | L1 forge.* |

## Boundaries NOT Requiring Capture

| Boundary | Why Deterministic | Strategy |
|----------|------------------|----------|
| Git data | Rebuildable from .git | Derived |
| Code parsing | Deterministic AST | Derived |
| Embedding | Same model = same vectors | Derived |

## Applied-In

- L2 eventlog design - captures surface decisions
- Forge sync - captures API responses in eventlog
- Ref repo storage - git derived, forge captured

## Evidence

- [[session-20260115-121358]] - L2 eventlog insight
- [[helland-paper]] - Academic grounding

## Revision Log

- 2026-01-16: Synthesized from L2 eventlog design session
