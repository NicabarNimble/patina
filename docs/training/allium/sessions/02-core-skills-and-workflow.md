# Session 2: Core Skills & Workflow (1.5 hours)

**Objective**: Master daily usage — the Allium loop as the default way to
ship behavior.

## Agenda

| Time | Segment |
|------|---------|
| 0:00–0:10 | Homework review: trade and read specs from Session 1 |
| 0:10–0:35 | The Allium loop, skill by skill |
| 0:35–0:55 | Hands-on 1: elicit + distill |
| 0:55–1:10 | Hands-on 2: break it, weed it |
| 1:10–1:25 | Exercise briefing: end-to-end feature |
| 1:25–1:30 | Patterns for common domains |

## 1. The Allium loop (25 min)

```text
        ┌──────────────── gather context ────────────────┐
        │  /elicit   forward: intent → spec              │
        │  /distill  backward: code  → spec              │
        └────────────────────┬───────────────────────────┘
                             ▼
        ┌──────────────── take action ───────────────────┐
        │  /propagate  spec → tests, implementation      │
        └────────────────────┬───────────────────────────┘
                             ▼
        ┌──────────────── verify ────────────────────────┐
        │  /weed          spec ↔ code alignment          │
        │  allium check   structural diagnostics         │
        │  allium analyse reachability, conflicts, flow  │
        └────────────────────┬───────────────────────────┘
                             ▼
        ┌──────────────── iterate ───────────────────────┐
        │  /tend  evolve the spec as behavior changes    │
        └─────────────────────────────────────────────────┘
```

Skill-by-skill talking points:

- **`/elicit`** — forward from intent. Input: a feature idea, a
  conversation, an issue. Output: a spec that forces the ambiguities into
  the open (every `open question "..."` in
  `patina-mct/layer/allium/mct-product-map.allium` is elicitation residue —
  show a couple, and show how decisions get recorded as `-- Decision:`
  comments next to them).
- **`/distill`** — backward from code. Input: existing module. Output: the
  behavioral contract the code already implements, minus the accidents.
- **`/propagate`** — spec to artifacts. Generates test obligations and
  drives implementation. In this repo the concrete form is
  `allium plan <spec> > <spec>.plan.json` plus obligation-tagged Rust tests
  (Session 5 covers this pipeline in depth).
- **`/weed`** — alignment check. Finds where code and spec have drifted
  apart, in both directions.
- **`/tend`** — controlled evolution. Behavior changes go to the spec
  *first*, then propagate outward.

Key habit to teach: **the loop is entered at different points** — new
feature → elicit; legacy code → distill; bug report → weed; changed
requirement → tend. It's not a fixed pipeline you always run end to end.

## 2. Hands-on 1: elicit + distill (20 min, pairs)

Pair juniors with seniors. Each pair does both directions:

- **Elicit a new feature spec.** Prompt: *"Patina should support snoozing a
  belief: a snoozed belief is excluded from `patina context` output until
  its snooze expires or is cancelled."* Elicit until the spec answers: who
  can snooze? what states exist? what happens at expiry? can you snooze a
  retired belief?
- **Distill from legacy code.** Use the homework module of the junior
  partner, or `patina/src/commands/slate.rs` as a fallback.

Both specs must pass `allium check` before the segment ends.

## 3. Hands-on 2: break it, weed it (15 min)

Take the elicited spec and deliberately damage it four ways, running
`allium check` after each:

1. Delete a `provides:` line from a surface → the CLI reports
   `allium.rule.unreachableTrigger` for every rule listening to it.
2. Add an entity nothing references → `allium.entity.unused` (warning).
3. Typo a field name in a `requires:` (e.g. `belief.stae`) → **the CLI
   stays silent**. Let the room sit with that.
4. Add a second rule with the same `when`/`requires` but a contradictory
   `ensures` → also silent (validated against CLI 3.5.0).

Then run `/weed` against a stub implementation and watch it catch what the
structural check missed — the typo and the contradiction.

The point, stated explicitly: **each net catches what the others miss.**
The CLI is a fast deterministic gate for structure; weed and review carry
the semantics. Green check ≠ correct spec — this is why the loop has three
verify layers, not one.

## 4. Exercise: end-to-end feature (brief in session, finish as homework)

Build one small feature using **only** the Allium-driven flow — no writing
code before the spec is green.

**Exercise**: [Toy grant lifecycle](../exercises/02-toy-grant-lifecycle/) —
request → active → expired/revoked, with a deny-by-default contract. This is
the same domain as MCT's real `ToyGrant`, scaled down for training. Starter
spec with TODOs and a validated solution are provided.

Alternative for variety: password reset or order processing, same rules.

Deliverable for Session 3: spec (check + analyse clean), plan JSON, and at
least one obligation-tagged test.

## 5. Recommended patterns for common domains (5 min, pointer only)

Point at the patterns library and the local exemplars:

- **Auth / authority decisions** — `MctPeerBinding` +
  `MctPeerAdmissionDecision` in `mct-product-map.allium`: separate the
  *evidence* (presentation), the *authority record* (binding), and the
  *decision* (admission) into distinct entities.
- **State machines** — `MotherDaemon.lifecycle` in
  `mother-lifecycle.allium`: one enum field, one rule per transition,
  timeout as a rule on `deadline <= now`.
- **Notifications / observations** — `MctObservation`: append-only facts
  with a closed `kind` set; logs/metrics as projections, never the truth.

## Resources

- `patina/layer/allium/mother/README.md` — the team's resume-style workflow
- [Cheat sheet: the Allium loop](../cheatsheets/allium-loop.md)
- [Exercise 02: toy grant lifecycle](../exercises/02-toy-grant-lifecycle/)
