---
type: belief
id: context-files-are-rules-not-docs
persona: architect
facets: [context-engineering, llm-workflow, documentation]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# context-files-are-rules-not-docs

Context files (CLAUDE.md, AGENTS.md) should contain behavioral rules and pointers, not inline documentation — LLMs follow instructions but unnecessary ones make tasks harder and increase cost.

## Statement

Context files (CLAUDE.md, AGENTS.md) should contain behavioral rules and pointers, not inline documentation — LLMs follow instructions but unnecessary ones make tasks harder and increase cost.

## Evidence

- [[session-20260223-152707]]: ETH Zurich eval-AGENTS.md paper (Feb 2026) — context files reduce task success rates while increasing cost 20%+; LLM-generated files hurt -3%, developer-written help +4%; recommendation is minimal requirements only (weight: 0.9)
- [[session-20260223-152707]]: Anthropic Skills Guide — progressive disclosure principle: frontmatter always loaded, SKILL.md on trigger, references on demand. Don't inline what can be discovered. (weight: 0.7)
- [[commit-50e7af6c]]: Applied to Patina CLAUDE.md — trimmed 155 → 44 lines, cut inline docs (architecture, project structure, key commands), kept behavioral rules and pointers (weight: 0.9)

## Supports

- [[unix-philosophy]] — minimal, focused context aligns with "one tool, one job"
- [[spec-is-a-directory]] — same principle at spec level: overview in SPEC.md, details in supporting docs loaded on demand

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- ETH paper caveat: when repos have no other documentation, context files DO help (+2.7%). Projects with unique conventions not in training data (like Patina) may need more context than generic repos.

## Applied-In

- `CLAUDE.md` ([[commit-50e7af6c]]): 134 lines deleted, kept rules + pointers only

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
