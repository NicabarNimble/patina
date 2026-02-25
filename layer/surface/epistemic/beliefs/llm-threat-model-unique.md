---
type: belief
id: llm-threat-model-unique
persona: architect
facets: [security, llm, threat-modeling]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-22
revised: 2026-02-22
---

# llm-threat-model-unique

LLM secret leakage threat model differs from traditional infosec - protect against LLM running 'cat' and 'env' commands to read secrets, not just disk encryption or network attacks

## Statement

LLM secret leakage threat model differs from traditional infosec - protect against LLM running 'cat' and 'env' commands to read secrets, not just disk encryption or network attacks

## Evidence

- [[session-20260222-054702]]: [[session-20260222-054702]] - User clarified goal: prevent LLM from seeing secrets via shell commands. PATINA_IDENTITY env var fails this (visible to printenv). Encrypted file passes (cat shows gibberish) (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-02-22: Created — metrics computed by `patina scrape`
