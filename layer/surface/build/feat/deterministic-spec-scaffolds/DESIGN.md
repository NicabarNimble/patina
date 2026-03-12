# Design: Deterministic Spec Scaffolds For Agents

## Why This Design

Patina's interface surfaces will keep diverging. Claude, OpenCode,
Gemini, and future runtimes may each have custom skills and tool usage
patterns. That makes it even more important that the core spec system be
deterministic and strong on its own.

The lesson from this session is simple:

- strong agents can still drift if the scaffold is weak
- strong specs should not depend on one interface's prompt habits
- the core spec workflow should carry more architectural discipline by
  default

## Build Target

After this build:

- `patina spec create` produces stronger feat/refactor scaffolds by
  default
- `DESIGN.md` scaffolds contain clearer implementation contracts
- `patina spec promote` or readiness checks can detect ambiguous specs
- agents get a better deterministic handoff surface regardless of
  interface runtime

## Resolved Decisions

- the core spec system should stay runtime-agnostic
- interface-specific skills should stay thin and mostly advisory
- deterministic scaffolding matters more than interface-specific prompt
  cleverness
- spec quality should be improved primarily through core tool structure,
  not per-interface customization
- improve the current spec workflow conservatively rather than replacing
  it with a new product shape
- direct code targets should start as prose conventions plus lint rather
  than a heavy structured schema change
- readiness lint should fail by default with an override path
- agent handoff should begin as an option on `patina spec show`

## Commits

1. `feat(spec): strengthen default spec and design scaffolds`
   - Update `src/commands/spec/internal/create.rs` templates to encode a
     stronger default flow.

2. `feat(spec): add readiness lint for ambiguous architecture`
   - Extend spec promotion/check logic so ready-stage specs are harder to
     leave underspecified.

3. `feat(spec): support direct code targets and agent handoff views`
   - Add structured support for code targets and a compact build-ready
     rendering mode for agents.

## Direct Code Targets

- `src/commands/spec/internal/create.rs` — body/design templates and
  spec creation defaults
- `src/commands/spec/mod.rs` — CLI surface for new lint/handoff options
- `src/commands/spec/internal/queries.rs` — handoff/readiness rendering
- `src/commands/spec/internal/mutations.rs` — promote/ready validation
- `layer/surface/build/feat/knowledge-child-platform/SPEC.md` — example
  of stronger flow to learn from
- `layer/surface/build/refactor/mother-doctrine-cleanup/SPEC.md` — test
  case for the improved structure

## Verification Plan

- create new feat and refactor specs and confirm the scaffold is
  materially stronger by default
- confirm readiness checks catch intentionally ambiguous test specs
- confirm handoff view shows resolved decisions, code targets, exit
  criteria, and open questions cleanly
- confirm the workflow remains interface-agnostic and does not depend on
  Claude/OpenCode/Gemini-specific prompt behavior

## Build Readiness

This design is intentionally focused on the deterministic core. It does
not attempt to unify interface UX. It should be possible to implement
this entirely inside the existing spec tooling while keeping custom
interface skills thin and the current workflow recognizable.

## Open Questions

- Should code targets become frontmatter-backed structured data, or stay
  in the body/design docs with lint support?
- Should readiness lint warn by default or fail promotion unless forced?
- Should handoff rendering be a new command or an option on
  `patina spec show`?
