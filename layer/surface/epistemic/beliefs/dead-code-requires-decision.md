---
type: belief
id: dead-code-requires-decision
persona: architect
facets: [code-quality, workflow, human-in-loop]
confidence:
  score: 0.85
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.95
    survival: 0.50
    user_endorsement: 0.90
entrenchment: medium
status: active
extracted: 2026-01-29
revised: 2026-01-29
---

# dead-code-requires-decision

Dead code requires a human decision, not silent annotation. Surface the code's original purpose and let the user choose: implement it or delete it.

## Statement

Dead code requires a human decision, not silent annotation. Surface the code's original purpose and let the user choose: implement it or delete it. `#[allow(dead_code)]` hides the decision rather than making it.

## Evidence

- session-20260129-074742: Found `#[allow(dead_code)]` on `get_spec_milestones()`. Presented options to user: "use it for progress display" vs "remove it". User chose remove. The annotation was hiding a decision that needed to be made. (weight: 0.9)
- [[session-20260129-074742]]: User correction: "dead code shouldn't always be deleted... the human should be presented with the dead code reason for creation and what it would mean to be implemented vs deleted" (weight: 0.95)

## Verification

```verify type="sql" label="No allow(dead_code) annotations" expect="= 0"
SELECT COUNT(*) FROM code_search WHERE context LIKE '%allow(dead\_code)%' ESCAPE '\'
```

## Supports

- [[signal-over-noise]]: Surfacing dead code for decision is signal; silencing it is noise
- [[smart-model-in-room]]: LLM can explain what dead code does and present options

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/version/internal.rs`: `get_spec_milestones()` surfaced with options (progress display vs remove), user chose remove

## Revision Log

- 2026-01-29: Created (confidence: 0.85)
- 2026-01-29: Revised — not "always delete" but "surface for decision" (user correction)
