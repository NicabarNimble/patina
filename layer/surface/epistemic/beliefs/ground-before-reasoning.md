---
type: belief
id: ground-before-reasoning
persona: architect
facets: [process, llm-discipline, epistemic]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-02
revised: 2026-02-02
---

# ground-before-reasoning

Ground reasoning in code, beliefs, and data before making inferential leaps — read-code-before-write applies to thinking, not just editing.

## Statement

Ground reasoning in code, beliefs, and data before making inferential leaps — read-code-before-write applies to thinking, not just editing.

## Evidence

- [[session-20260202-063713]]: LLM assumed "archive" meant "move to dust" without checking how the version system handles completed specs; chased a false path for multiple turns before user corrected (weight: 0.9)
- [[session-20260202-063713]]: Andrew Ng review surfaced the contested `archive-completed-work` finding, but reasoning about the fix was ungrounded — jumped to file moves instead of reading the version system (weight: 0.8)

## Supports

- [[read-code-before-write]] — extends the same principle from code editing to reasoning
- [[measure-first]] — measuring the system state before reasoning about changes

## Attacks

## Attacked-By

## Applied-In

- `archive-completed-work` belief update — the contested finding should be re-evaluated against the version system, not assumed to require file moves

## Revision Log

- 2026-02-02: Created — metrics computed by `patina scrape`
