---
type: explore
id: alignment-narrative-slice-execution
status: draft
created: 2026-03-14
sessions:
  origin: 20260313-155708-WKJS
exit_criteria: []
---
# explore: Alignment Narrative for Slice Execution

> Define how user vision, current code truth, and user preferences become a repeatable slice-by-slice execution contract.

## Question

How should Patina treat "alignment narrative" as a first-class planning model,
so user vision, current code truth, and user preferences reliably drive
slice-by-slice implementation?

What structure keeps agent execution narrow and verifiable without losing
system-level intent?

## Findings

1. Alignment needs an explicit spine before any slice starts.

- Vision: the user-visible outcome and success condition.
- Code truth: what the repo currently does (facts, not intent).
- Preferences: user coding preferences (style, boundaries, dependency rules).
- Constraints: hard limits (safety, performance, architecture boundaries).

2. Dependency policy must be stated as a gate, not a suggestion.

- In-tree first: prefer existing dependencies and local patterns.
- New dependency only with explicit gap evidence.
- User preference can tighten policy further (for example: "no new deps").

3. Slices should be vertical and contract-shaped.

- One user behavior or one integration boundary per slice.
- Include UI/CLI behavior, logic, data movement, and tests where applicable.
- Keep acceptance binary: pass/fail with concrete verification commands.

4. The alignment narrative should be a repeatable sentence per slice.

Template:
"Given vision V, current code truth T, and preferences P under constraints C,
this slice changes X to achieve Y, verified by Z."

5. Patina command flow already supports this model with minimal extension.

- Discover and ground truth: `patina context`, `patina scry`, `patina assay`.
- Create bounded contract: `patina spec create <type> <id>`.
- Prepare execution packet: `patina spec prompt <id>`.
- Run/review loop: implement, verify, update session notes.
- Handoff/continuation: `patina spec handoff <id>` and `patina spec packet <id> --json`.
- Scope correction when needed: `patina spec split <id>`.

6. This model matches Patina's spec governance principle.

- Specs remain authority.
- Sessions capture reasoning.
- Execution stays within bounded scope and explicit exit criteria.

## Conclusions

Patina should treat alignment as a two-level system:

1. North-star alignment narrative (map):
- vision,
- code truth,
- preferences,
- constraints.

2. Executable slice specs (route segments):
- one bounded behavior,
- explicit non-goals,
- verification commands,
- completion criteria.

Operationally: spec broadly, execute narrowly, and re-anchor each new slice to
the same alignment spine before coding starts.

This keeps work dependable under agent execution while preserving user intent.
