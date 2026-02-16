---
id: spec-driven-design
layer: core
status: active
created: 2026-02-08
tags: [governance, specs, agentic, process, core-principle]
references: [dependable-rust, unix-philosophy, specs-push-discoveries-outbound]
---

# Spec-Driven Design

**Purpose:** SPECs are the single source of truth for all non-trivial action. Code traces to SPECs, SPECs trace to sessions and beliefs. The AI agent executes within SPEC scope — never outside it.

---

## Core Principle

Every non-trivial change must be authorized by a SPEC. The SPEC is a contract: it defines what gets built, why, and what "done" looks like. Sessions capture the discussion that produces SPECs. Beliefs capture the principles that inform them. Code implements them. Nothing else authorizes action.

**Sessions discuss. SPECs decide. Code executes.**

## The Problem This Solves

Without spec-driven governance, agentic development falls into a loop: rich sessions full of learning and iteration, but little action. Sessions have no contract and no exit criteria — they just end. The AI makes judgment calls that feel reasonable in the moment but diverge from intent. Knowledge accumulates but nothing ships.

SPECs break the loop by converting discussion into contracts with clear scope, exit criteria, and accountability.

## The Pipeline

```
session (discuss) → SPEC (contract) → action (execute) → session (review)
```

Each stage has a different relationship with the AI agent:

- **Session**: Participatory. Discuss, suggest, explore, disagree.
- **SPEC**: Subordinate. Execute what the SPEC says. Surface gaps. Never fill them unilaterally.
- **Action**: Bounded. Every line of code traces to a SPEC. If the SPEC doesn't authorize it, it doesn't happen.

## Rules

### 1. SPEC Is the Authority

The SPEC defines the scope of work. When the AI encounters an edge case the SPEC doesn't address, the correct action is to stop and ask — not to make a judgment call.

```
SPEC says: "Delete dead code"
AI finds:  dead functions with tests

Bad:  AI decides to preserve them with #[cfg(test)] (unilateral)
Good: AI asks "these functions are dead but have tests — delete both?"
```

### 2. SPEC Amendments Are First-Class

When a SPEC is wrong or incomplete, the fix is a new linked spec — not a silent divergence. This preserves decision provenance.

- **fix**: the original SPEC was wrong (use existing `fix` type)
- **refactor/feat**: the SPEC needs extension (use existing types)
- Link via `related` field in frontmatter to maintain the chain

The amendment chain gives you something most AI workflows lack: you can trace any piece of code back through code → commit → SPEC → amendment → original SPEC → session → beliefs.

### 3. Sessions Are Not Authorization

Sessions capture the messy process: discussion, false starts, questions, decisions-in-progress. They are raw material for SPECs, not substitutes for them.

A session can identify that work needs to happen. Only a SPEC can authorize it.

### 4. The Threshold

Not everything needs a full SPEC. The line:

**Needs a SPEC:**
- Changes to architecture or module boundaries
- New features or capabilities
- Refactors that touch multiple files
- Anything with exit criteria worth defining
- Any work where the AI might make judgment calls about scope

**Lives in commits/sessions:**
- Typo fixes, formatting
- Single-line bug fixes with obvious correctness
- Documentation updates to existing content

When in doubt, the answer is SPEC. The cost of an unnecessary SPEC is low (a small document). The cost of unscoped work is high (divergence, rework, lost intent).

### 5. Decision Provenance

Every piece of code should be traceable:

```
code → commit → SPEC → (optional amendment → original SPEC) → session → beliefs
```

This chain is what makes Patina's knowledge layer trustworthy. Without it, accumulated knowledge is just text. With it, every decision has context, rationale, and history.

### 6. Push Discoveries Outbound

Discoveries made during one spec's execution that affect other specs must be pushed to the destination spec before the originating spec can close. See [[specs-push-discoveries-outbound]].

Without this rule, archiving the originating spec severs the knowledge chain. The discovery lives in session logs (archived), beliefs (only if searched for), and commit messages (buried). None of these paths naturally surface when opening the destination spec.

```
Working on SPEC A → discover something that affects SPEC B

Bad:  Note it in session log, archive SPEC A, SPEC B never knows
Good: Push discovery to SPEC B (beliefs, related links, discovery notes)
      THEN archive SPEC A
```

This applies at all three layers:
- **Process**: check for outbound discoveries before closing a spec
- **Tooling**: `patina spec discover <target> "note"` (future)
- **Structural**: `discoveries` field in frontmatter, warned at close time (future)

### 7. Ground Every Assertion

Every testable claim in a SPEC must carry its evidence inline. An ungrounded assertion is a hypothesis masquerading as a contract — and review becomes the testing mechanism. See [[ground-assertions-or-pay-review-tax]].

Three forms of grounding:

**Verification commands** — run the command during spec creation, document timing and expected output:
```
Bad:  "Run `rg 'foo'` — should return zero"
Good: "Run `rg 'foo' src/ tests/` post-commit — targets only
       actionable locations. Should return zero."
```

**Invariants** — every "doesn't change" claim needs a one-line justification:
```
Bad:  "build.rs files — do not change"
Good: "build.rs files — paths are crate-relative; cargo runs build.rs
       from crate root, so internal paths survive the directory move"
```

**Prerequisites** — state execution context before the command, not after a reviewer discovers it:
```
Bad:  "cargo package -p my-crate"
Good: "cargo package -p my-crate (post-commit; validates manifest
       and include/exclude globs, no registry credentials needed)"
```

The cost of grounding is one sentence per assertion. The cost of NOT grounding is 2-3 review cycles per assertion — and the cycles compound because each fix can introduce new unstated assumptions.

## Relationship to Other Patterns

**[[dependable-rust]]**: SPECs are the external interface for work. Like a module's public API, the SPEC is small, stable, and authoritative. Implementation details (how the AI gets there) are internal. The contract (what gets built) is the SPEC.

**[[unix-philosophy]]**: One SPEC, one job. A SPEC that tries to authorize everything authorizes nothing. Focused SPECs with clear exit criteria are composable — they can block each other, relate to each other, and build on each other.

**[[adapter-pattern]]**: The SPEC system is adapter-agnostic. Whether Claude, Gemini, or a human reads the SPEC, the contract is the same. The SPEC doesn't encode how to build — it encodes what to build and when it's done.

## Existing Infrastructure

The spec system already supports this governance:

- **Types**: `feat`, `fix`, `refactor`, `explore` — map to kinds of authorized work
- **Status**: `draft` → `ready` → `active` → `complete` / `abandoned`
- **Relationships**: `blocked_by`, `blocks`, `related` — dependency chains
- **Beliefs**: linked beliefs provide the "why" behind the contract
- **Sessions**: linked sessions provide the discussion history
- **Milestones**: phased delivery with version targets

What this pattern adds is the governance rule: these aren't just organizational tools, they're the authority system.

## Common Mistakes

**1. AI fills gaps instead of surfacing them**
```
SPEC says: "merge match arms"
AI finds:  unused imports after merge

Bad:  AI silently cleans up imports (reasonable but unauthorized)
Good: AI cleans up imports AND notes it as a consequence of the spec'd change
Best: SPEC anticipated this — "merge match arms, clean up dead code"
```

**2. Sessions substitute for SPECs**
```
Bad:  "We discussed adding caching in the session, so I'll add it"
Good: "We discussed caching — should I draft a SPEC for it?"
```

**3. SPECs are too broad**
```
Bad:  SPEC: "Improve the retrieval system"
Good: SPEC: "Move factual oracles from scry to assay" (Phase 1)
      SPEC: "Build knowledge domain from beliefs + patterns + commits" (Phase 2)
```

**4. Amendments bypass the chain**
```
Bad:  Edit the original SPEC to change scope mid-work
Good: Create a fix/patch SPEC that links to the original, preserving history
```

## References

- [Dependable Rust](./dependable-rust.md) - Black-box module pattern (SPECs as external interface)
- [Unix Philosophy](./unix-philosophy.md) - One tool, one job (one SPEC, one scope)
- [Adapter Pattern](./adapter-pattern.md) - Agent-agnostic contracts
- [Session Capture](./session-capture.md) - The discussion layer that feeds SPECs
