---
type: belief
id: skills-for-structured-output
persona: architect
facets: [tooling, architecture, epistemic]
confidence:
  score: 0.75
  signals:
    evidence: 0.80
    source_reliability: 0.75
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-16
revised: 2026-01-16
---

# skills-for-structured-output

Skills with validation scripts provide deterministic structured output by having the system own the format while the LLM provides content.

## Statement

Skills with validation scripts provide deterministic structured output by having the system own the format while the LLM provides content.

## Evidence

- [[session-20260116-095954]]: Explored MCP vs Skills for E2 belief creation, chose skills for progressive disclosure and deterministic scripts (weight: 0.85)

## Supports

- [[dont-build-what-exists]] - Skills leverage Claude Code's built-in system
- [[smart-model-in-room]] - LLM synthesizes content, system handles format

## Attacks

- [[json-schema-for-validation]] (status: defeated, reason: separate schema file must stay in sync with code)

## Attacked-By

- [[adapter-agnostic-required]] (status: active, confidence: 0.4, scope: "when supporting Gemini CLI or OpenCode")

## Applied-In

- `.claude/skills/epistemic-beliefs/` - belief creation skill
- `create-belief.sh` - validation script for beliefs

## Revision Log

- 2026-01-16: Created (confidence: 0.75)
