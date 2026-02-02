---
type: belief
id: system-owns-format
persona: architect
facets: [architecture, llm, tooling]
confidence:
  score: 0.80
  signals:
    evidence: 0.85
    source_reliability: 0.80
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-16
revised: 2026-01-16
---

# system-owns-format

For deterministic output, the system should own the format while the LLM provides content - LLM synthesizes data, system handles structure and validation.

## Statement

For deterministic output, the system should own the format while the LLM provides content - LLM synthesizes data, system handles structure and validation.

## Evidence

- [[session-20260116-095954]]: E2 design principle - LLM provides args to script, script writes markdown with correct format (weight: 0.85)

## Verification

```verify type="sql" label="Validation scripts tracked in git" expect=">= 1"
SELECT COUNT(*) FROM git_tracked_files WHERE file_path LIKE '%scripts/create-belief%'
```

```verify type="sql" label="Grounding reaches belief scraper" expect=">= 1"
SELECT COUNT(*) FROM belief_code_reach WHERE belief_id = 'system-owns-format' AND file_path LIKE '%beliefs%.rs'
```

## Supports

- [[skills-for-structured-output]] - Skills implement this pattern
- [[smart-model-in-room]] - Lets LLM focus on synthesis, not formatting

## Attacks

- [[llm-writes-markdown-directly]] (status: defeated, reason: format discovery is non-deterministic, prone to errors)
- [[json-schema-for-validation]] (status: scoped, reason: "Rust structs serve same purpose without separate schema file")

## Attacked-By

- [[llm-flexibility-needed]] (status: active, confidence: 0.3, scope: "when output format varies by context")

## Applied-In

- `create-belief.sh` - LLM provides args, script writes markdown
- Session commands - LLM provides content, shell scripts own file format
- Commit messages - LLM drafts, system validates via hooks

## Revision Log

- 2026-01-16: Created (confidence: 0.80)
