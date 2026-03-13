---
type: feat
id: spec-prompt-handoff
status: draft
created: 2026-03-12
sessions:
  origin: 20260312-140904
related:
- src/commands/spec/mod.rs
- src/commands/spec/internal
- src/spec.rs
- layer/surface/build/refactor/cli-first-spec-workflow/SPEC.md
 - layer/surface/build/feat/spec-prompt-handoff/PROMPT_TEMPLATE.md
exit_criteria:
- id: spec-prompt-command-exists
  text: "`patina spec prompt <id>` generates a build-ready execution prompt packet from SPEC + DESIGN + key repo context"
  checked: false
- id: prompt-packet-has-stable-sections
  text: "Generated prompt packets use stable sections (goal, read-first, direct targets, execution order, constraints, verification, done definition) and avoid runtime-specific drift"
  checked: false
- id: handoff-command-exists
  text: "`patina spec handoff <id>` creates a durable handoff packet with progress, resolved decisions, open items, and next-agent instructions"
  checked: false
- id: json-packet-projection-available
  text: "`patina spec packet <id> --json` outputs machine-readable prompt/handoff packet payload for agent orchestration"
  checked: false
- id: template-derived-from-proven-style
  text: "A reusable prompt template derived from the proven operator style exists and is used as the scaffold baseline"
  checked: false
- id: cli-first-spec-workflow-remains-canonical
  text: "Prompt/handoff generation remains a thin projection over canonical spec files and does not reintroduce divergent spec semantics"
  checked: false
- id: session-workflow-integration-defined
  text: "Prompt and handoff packets integrate with session-update/session-note/session-end expectations without duplicating session storage logic"
  checked: false
- id: verification-covers-determinism-and-usability
  text: "Tests cover packet determinism, parseability, and practical usability for zero-context builder agents"
  checked: false
---
# feat: Spec Prompt and Handoff Packets

> Add first-class prompt packet generation from specs and evolve toward durable multi-agent handoff packets.

## Problem

Patina specs are strong architecture contracts, but the execution briefing
for agents is still hand-authored and inconsistent.

Current pain:

- teams repeatedly write custom prompts to translate spec intent into
  agent-executable steps,
- quality varies by author and model,
- handoff between agents loses context or reopens resolved decisions,
- the spec system has no first-class packet shape for prompt generation.

This creates friction exactly where the spec system should be strongest:
deterministic execution and reliable handoff.

## Goal

Make prompt packets and handoff packets first-class outputs of the spec
system.

The resulting workflow should let a zero-context builder agent begin work
correctly by reading one generated packet rather than relying on ad-hoc
session memory.

## Status

Today:

- spec files (`SPEC.md` / `DESIGN.md`) are canonical,
- high-quality prompt style exists in operator practice,
- there is no built-in command that projects that style from spec data,
- handoffs are manual and fragile.

## Non-Goals

- Do not replace SPEC.md / DESIGN.md as canonical architecture artifacts.
- Do not move spec semantics into prompt templates.
- Do not build model-specific prompt dialects as core behavior.
- Do not duplicate session persistence in spec packet files.
- Do not tie packet generation to one interface runtime.

## Target Shape

- `patina spec prompt <id>` outputs a deterministic execution prompt packet.
- `patina spec handoff <id>` outputs a durable handoff packet for the
  next agent/operator.
- `patina spec packet <id> --json` provides machine-readable packet data.
- packet generation is thin and derived from spec truth, not parallel truth.
- a reusable prompt template is shipped from the proven operator style.

## Solution

### 1. Add prompt packet command

- Implement `patina spec prompt <id>` with predictable section ordering.
- Pull from SPEC metadata, exit criteria, design decisions, direct targets,
  verification plan, and constraints.

### 2. Add handoff packet command

- Implement `patina spec handoff <id>` for next-agent continuation.
- Include status snapshot, completed work markers, unresolved risks, and
  action-oriented next steps.

### 3. Add JSON packet projection

- Implement `patina spec packet <id> --json` for orchestrators.
- Keep schema stable and documented for downstream tooling.

### 4. Ship template baseline from proven style

- Add a prompt template scaffold derived from the operator style used in
  successful sessions.
- Keep template declarative and runtime-agnostic.

### 5. Integrate with session lifecycle expectations

- Packet sections should remind operators to use session update/note/end
  workflow, but not own session data model itself.

### 6. Preserve CLI-first canon

- Generated packets must reference spec files as source of truth.
- Packet generation must not mutate or reinterpret spec lifecycle state.

## Implementation Order

1. Define packet schemas (human and JSON).
2. Implement prompt packet generation command.
3. Implement handoff packet generation command.
4. Implement JSON projection command/flag.
5. Add reusable prompt template and docs.
6. Add determinism/usability tests.
7. Validate with zero-context builder-agent dry runs.

## Resolved Decisions

- Specs remain canonical; packets are projections.
- Prompt and handoff packets are separate outputs with different intent.
- JSON packet output is required for future orchestration tooling.
- The initial template is based on proven operator style, then iterated.
- Runtime-specific language stays thin and optional.

## Verification

- Unit tests: deterministic section ordering and stable packet rendering.
- Snapshot tests: known spec fixtures render identical packet outputs.
- JSON schema tests: parseability and required fields.
- Usability tests: zero-context agent can execute from packet only.
- Regression tests: packet generation does not alter spec metadata/state.

## Exit Criteria

Use frontmatter exit_criteria as source of truth.

## Build Readiness

- [ ] Packet schema defined for text + JSON.
- [ ] Template file committed and referenced.
- [ ] Command help/docs updated for new subcommands.
- [ ] Tests cover deterministic output and usability.
