---
type: feat
id: mother-design
status: draft
created: 2026-02-16
sessions:
  origin: 20260216-113530
related:
- layer/core/patina-identity.md
- layer/surface/build/feat/cross-project-beliefs/SPEC.md
beliefs:
- beliefs-are-the-product
- mother-is-the-daemon
- mother-owns-ref-repo-indexing
---

# feat: Mother Design — Knowledge Model & Evolution Roadmap

> Design principles and vocabulary for Mother's knowledge system.
> Parking lot for ideas that outlive any single implementation SPEC.
> Draft — may be built, split, or abandoned.

## Knowledge Model

Two kinds of epistemic knowledge, distinct in scope and grounding:

**Beliefs** are project-scoped assertions with project-scoped evidence. "This
codebase prefers Result types for error handling" — grounded in commits, code
patterns, and session history within one project. Beliefs live in
`layer/surface/epistemic/beliefs/` (git-tracked, per-project).

**Values** are cross-project principles held by a persona. "I prefer explicit
error handling across all projects" — grounded across projects (which projects
apply this? what evidence from multiple codebases?). Values live in
`~/.patina/layer/surface/beliefs/` (machine-local, per-user).

Values need grounding just as much as beliefs — but cross-project grounding,
not single-project evidence. That grounding infrastructure doesn't exist yet.
[[cross-project-beliefs]] makes values discoverable; this SPEC considers how
to make them verifiable.

## Design Principles

### Projects Are Islands

Projects are self-contained islands of knowledge. Mother is the overlay that
sees across islands and brokers introductions. The boundary rule:

- Projects never write into Mother
- Mother never writes into projects
- Cross-project links live in Mother's graph, not in either project

If a user adopts a belief from project-B into project-A, the belief is created
natively in project-A's layer. Mother records the provenance edge. Neither
project knows about the other.

Per [[mother-owns-ref-repo-indexing]]: "projects are the door, mother is the
house."

### Mother Pulls, Never Pushes

Data flows from projects to Mother via `mother sync`, not from `scrape` into
graph.db. `patina scrape` stays project-pure — it doesn't know Mother exists.
Mother reaches down to read from project islands.

This was a correction from the original SPEC design which had scrape writing
to graph.db — that violated the island boundary.

### Persona Is Not User

Persona is architecturally distinct from user. A user could have multiple
personas with different values (e.g., "rust-architect" vs "quick-prototyper").
For now user = persona (1:1), but the name slot matters. Persona separation
and multi-persona support are future capabilities.

## Evolution Roadmap

These are ideas, not commitments. Each would need its own SPEC if pursued.

### Legacy Persona Deprecation

The persona system (`src/commands/persona/mod.rs`) was born Dec 2025. It has
5 subcommands, 3 integration points in scry, and 2 data stores (JSONL +
markdown). It's tech debt — the concept is valid (user-layer values) but the
module is a parallel universe disconnected from the belief system.

Deprecation path:
1. `persona note` should write `~/.patina/layer/surface/beliefs/` directly,
   replacing JSONL entirely
2. `persona query` and `persona materialize` are superseded by `mother search`
   and `mother sync` once [[cross-project-beliefs]] ships
3. Eventually: remove persona commands, fold remaining functionality into
   mother subcommands

### Value Grounding

Values currently have no grounding infrastructure — just markdown files on
disk. Future: cross-project grounding that answers "which projects apply this
value?" and "what evidence from multiple codebases supports it?"

This would require Mother to correlate value statements against project
beliefs across registered projects — a graph analysis problem.

### Belief Adoption Workflow

When `mother search` surfaces a belief from project-B, the user should be
able to adopt it into project-A with provenance tracking:

1. Create a new belief in project-A's layer
2. Record `sourced-from` reference in the belief
3. Add an `adopted-from` edge in Mother's graph

### Directory Naming

`~/.patina/layer/surface/beliefs/` stores values, not beliefs — a Phase 1
naming artifact. A future change should rename to `values/` for clarity.
Mother currently distinguishes by source field, not directory name.

### Multi-User / Multi-Persona

- Multiple personas per user with different value sets
- Per-user values at project level (team projects where each contributor
  has different preferences)
- Values-to-rules system (values that become enforceable project rules)

### Ref Repo Belief Extraction

Reference repositories may contain extractable beliefs. Mother could index
these alongside project beliefs — `[ref:beads] belief` in search results.

## Non-Goals (for this SPEC)

This is a design document, not an implementation contract. It has no exit
criteria and authorizes no code changes. Individual items graduate to their
own SPECs when ready to build.

## Evidence

| Claim | Source |
|-------|--------|
| Persona system has 5 subcommands | `src/commands/persona/mod.rs` (note, materialize, query, list, status) |
| Persona born Dec 2025 | `git log --follow src/commands/persona/` |
| Persona writes JSONL only, not markdown | `src/commands/persona/mod.rs:76-87` |
| 5 persona values exist as markdown | `~/.patina/layer/surface/beliefs/` |
| Original SPEC had scrape writing to graph.db | [[session-20260216-113530]] review finding |
| Mother v2 archived | `git tag spec/mother-v2` |
