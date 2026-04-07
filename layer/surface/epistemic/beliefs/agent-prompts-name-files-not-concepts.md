---
type: belief
id: agent-prompts-name-files-not-concepts
persona: architect
facets: [agent-orchestration, development-process, code-quality]
confidence:
  score: 0.92
entrenchment: medium
status: active
extracted: 2026-04-07
revised: 2026-04-07
---

# agent-prompts-name-files-not-concepts

Build agent prompts must name specific files and line numbers, not abstract concepts. Agents that receive concepts invent assumptions; agents that receive file paths read code.

## Statement

When writing prompts for build agents, always include concrete file paths with line numbers and "read these files first" lists. Agents given abstract instructions ("read the schema version") will assume the mechanism exists. Agents given concrete targets ("read `src/eventlog.rs:159` for the `set_last_processed` pattern") will discover whether it exists before building on it.

## Evidence

- [[session-20260407-063612]]: Federation build agent assumed `schema_version` key existed in `scrape_meta` table — no code writes that key. Agent built `read_project_schema_major()` against a nonexistent data contract. Required two fix commits ([[commit-22407370]], [[commit-d7ffae23]]) to add the write path and handle migration. (weight: 0.95)
- [[session-20260407-063612]]: Pando-platform build agent prompt included explicit "read these files first" list with `mother/src/state.rs` line references, `mother/src/builtin_children.rs`, `src/main.rs` routing code. Agent followed the pattern and produced correct code without assumption gaps. (weight: 0.90)
- [[session-20260407-063612]]: Federation build agent prompt included `mother/src/registry.rs:112` for `observe_handle` telemetry pattern. Agent replicated the exact event schema (`event_type`, `source_id`, labels). Concrete reference → correct code. (weight: 0.90)

## Supports

- [[read-code-before-write]] — the same principle, applied to agent prompt authoring
- [[spec-driven-design]] — specs name code targets; prompts must do the same
- [[audit-prompt-build-cycle]] — audit agents catch gaps, but prompts that name files prevent gaps in the first place
- [[ground-before-reasoning]] — ground agent in actual code state, not conceptual model

## Attacks

- [[trust-the-agent]] (status: scoped, reason: agents are capable but operate on the information given; vague prompts produce plausible-but-wrong code)

## Attacked-By

- [[prompt-maintenance-cost]] (status: active, confidence: 0.3, scope: "line numbers drift as code evolves; use grep patterns when exact lines are unstable")

## Applied-In

- Pando-platform build agent prompt: 6 files named with line numbers in "read before writing" section
- Federation build agent prompt: 8 files named with line numbers, DuckDB crate API caution section
- Fix pattern: when agent builds on nonexistent contract, the fix is two commits (add the contract, handle the migration case)

## Revision Log

- 2026-04-07: Created from schema_version gap discovery in session-20260407-063612-748374000 (confidence: 0.92)
