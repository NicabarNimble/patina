# Session 6 (Optional): Contributing & Advanced Topics (1.5 hours)

**Objective**: Turn proficient users into upstream contributors and handle
the sharp edges of large-scale use.

## Agenda

| Time | Segment |
|------|---------|
| 0:00–0:25 | Open-source contribution workflow |
| 0:25–0:50 | Custom skills & agents |
| 0:50–1:10 | Performance & edge cases with large specs |
| 1:10–1:30 | Future directions + capstone contribution review |

## 1. Open-source workflow (25 min)

- Repos: **allium** (language, skills, docs) and **allium-tools** (CLI,
  parser, LSP, Tree-sitter). Read each repo's `AGENTS.md` and contributing
  guide before opening anything.
- Contribution ladder, lowest friction first:
  1. **Docs/patterns** — fix a pattern, document a workflow you use here
     (the resume-style obligation pipeline is genuinely shareable).
  2. **Skills** — improve a prompt guardrail you've seen fail.
  3. **Diagnostics** — a new check/analyse rule (Session 4 Part B
     proposals).
  4. **Parser/LSP** — grammar and tooling changes.
- Issue → discussion → PR etiquette: lead with the spec snippet that
  motivates the change and the finding JSON you expect (same format as the
  Session 4 exercise — that write-up *is* the issue body).
- Local norms still apply when contributing from this codebase: pin
  versions in CI (`install-allium-ci.sh` pattern), no artifact sprawl.

## 2. Custom skills & agents (25 min)

- Anatomy of a team-local skill: procedure + guardrails + deterministic
  gate (`allium check` in the loop). Store team skills next to the code
  they serve.
- Live build (group): a `/weed-ci` variant — an agent instruction file
  that, on a PR, loads changed `.allium` files + touched Rust modules,
  runs weed, and posts findings. Wire it to the obligation-comment
  convention so it can flag *untagged* obligations.
- Composition: skills calling CLI calling skills — where to put the
  deterministic gates so autonomous runs stay honest.
- Patina angle: Mother as the harness that schedules such agents
  (children/toys), with observations as the audit trail — the same
  decide/effect split Allium's own architecture uses.

## 3. Performance & edge cases with large specs (20 min)

Use `mct-product-map.allium` (~1900 lines) as the live subject:

- **Analysis cost** grows with the rule-interaction graph, not line count —
  many small independent contracts are cheap; one entity with 30
  interacting rules is not. Time `allium analyse` on the product map vs a
  Mother spec and compare.
- **When to split**: a spec that needs its own scope block *inside* itself
  is two specs. Split along `-- Excludes:` seams (the Mother specs are the
  worked example).
- **Enum explosion**: closed `kind`/`reason` sets (see `MctObservation.kind`)
  are the honest way to model taxonomies, but review their diffs carefully —
  every added variant is a behavioral promise.
- **Plan JSON churn**: large specs make large `plan.json` diffs; keep one
  plan per spec (never hand-edit) and let `regenerate-artifacts.sh` own the
  refresh so drift detection stays trustworthy.
- **Skill context limits**: agents can't load a 1900-line spec plus code;
  this is another force pushing toward modular specs + a map.

## 4. Future directions (10 min, discussion)

- **Formal verification ties** — Allium deliberately stops short of full
  formal methods (Session 3); the open question is exporting to
  model-checkable form (state graph → TLA+/Alloy) for the few contracts
  that warrant it, without changing the authoring experience.
- **Deeper harness integration** — specs as first-class runtime evidence
  (Mother readiness gates consuming plan obligations; control planes like
  8090 consuming `allium model`).
- **Spec-aware review** — LSP/CI surfacing *behavioral* diffs ("this PR
  removes an invariant") instead of textual ones.

## 5. Capstone contribution review (20 min)

Each pair presents their capstone contribution proposal (5 min each):
defect → motivating snippet → expected finding/change. The team votes on
one or two to actually submit upstream; facilitator assigns shepherds.

## Resources

- allium / allium-tools contributing guides and `AGENTS.md`
- Session 4 exercise write-ups (the proposal seeds)
- [Facilitator guide — assessment](../facilitator-guide.md#assessment)
