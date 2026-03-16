# Spec Build Prompt Template

Session Goal: <one-line explicit goal>

Read FIRST (anchoring values):
- layer/core/dependable-rust.md - Small public interface, hide implementation
- layer/core/unix-philosophy.md - Each module does one thing
- layer/core/spec-driven-design.md - SPECs decide. Read the spec landscape
- layer/core/safety-boundaries.md - Project-scoped, user consent

Read the spec and design:
- <path/to/SPEC.md>
- <path/to/DESIGN.md>

Read code BEFORE writing anything:
- <path/to/file.rs> - <why this file matters>
- <path/to/file.rs> - <why this file matters>
- <path/to/file.rs> - <why this file matters>

CONTEXT - WHERE WE LEFT OFF:

<2-6 paragraphs of concrete state: what shipped, what failed, what remains>

THIS SESSION'S GOAL:

<clear objective in one paragraph>

Layered framing (recommended):

1. Runtime plane:
- <what exists>
- <what must change>
- <what boundary must be preserved>

2. Control plane:
- <domain model/control flows>
- <authority and policy constraints>
- <migration and integrity concerns>

3. UX/Operator plane:
- <commands/APIs expected>
- <headless/automation behavior>
- <error and remediation expectations>

KEY DESIGN DECISION TO LOCK FIRST:

<single irreversible architecture decision>

Execution Rules:
- Treat SPEC + DESIGN as binding.
- Do not reopen resolved decisions unless code contradiction appears.
- Read code before edits.
- Keep one commit worth of purpose per unit of change (if committing later).

STOP CONDITIONS:
- Do NOT <scope escape 1>
- Do NOT <scope escape 2>
- Do NOT <scope escape 3>
- If blocked, ask exactly one targeted question with recommended default.

DELIVERABLES:
1. <deliverable>
2. <deliverable>
3. <deliverable>
4. Open questions that block implementation

Verification Required:
- <test command>
- <test command>
- <test command>

Definition of Done:
- Exit criteria in SPEC are genuinely satisfied (not prose-only).
- Verification commands pass.
- Any spec metadata updates reflect actual implementation state.

Session Workflow Reminder:
- Run `/session-update` periodically.
- Run `/session-note` for important insights.
- Run `/session-end` at completion.
