---
type: belief
id: llm-readable-code
persona: architect
facets: [architecture, llm, documentation]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# llm-readable-code

Code should be self-documenting for AI readers — data contracts, doc comments, and structured metadata enable LLMs to understand what a command does, reads, and writes without tracing through implementation

## Statement

Code should be self-documenting for AI readers — data contracts, doc comments, and structured metadata enable LLMs to understand what a command does, reads, and writes without tracing through implementation

## Evidence

- [[session-20260205-084522]]: CLI reorganization discussion — "Most commands can stay top-level. It's how we organize the CODE that matters. An LLM needs to see the command and understand." (weight: 0.9)
- [[system-introspection]] SPEC: DataContract type with reads/writes/write_path enables `patina introspect` (weight: 0.8)

## Supports

- [[spec-first]] — specs are LLM-readable documentation
- [[read-code-before-write]] — LLM must be able to understand code it reads

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Maintenance burden — contracts drift from implementation if not verified
- Over-documentation — too much metadata obscures actual code

## Applied-In

- [[cli-reorganization]] SPEC: `DATA_CONTRACT` constant with `CommandGroup`, reads, writes, related commands
- [[system-introspection]] SPEC: DataContract schema with Source/Sink enums and WritePath taxonomy
- Group `mod.rs` documentation pattern: Each command group has module-level docs explaining purpose and data flow

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
