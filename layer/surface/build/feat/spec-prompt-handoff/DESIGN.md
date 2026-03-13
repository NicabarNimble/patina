# Design: Spec Prompt and Handoff Packets

## Why This Design

Patina needs to bridge two realities:

- specs are durable architecture contracts,
- agents need highly actionable, context-rich briefings.

This design keeps specs canonical and introduces packets as deterministic
projections. That enables consistent execution prompts and reliable
handoff without reintroducing prompt-only truth.

## Build Target

Ship three capabilities:

- `patina spec prompt <id>` → human-readable execution packet
- `patina spec handoff <id>` → human-readable continuation packet
- `patina spec packet <id> --json` → machine-readable payload

with template-guided section structure derived from proven operator
prompt style.

## Resolved Decisions

- Packet generation is read-only over SPEC/DESIGN data.
- Prompt packet and handoff packet are distinct first-class outputs.
- JSON output schema is part of contract surface.
- Initial packet template is runtime-agnostic and section-oriented.
- Session workflow reminders are included as guidance only.

## Commits

1. `feat(spec): define prompt/handoff packet data model`
   - Add internal structs + schema helpers.

2. `feat(spec): add spec prompt command`
   - Render deterministic execution packet.

3. `feat(spec): add spec handoff command`
   - Render continuation packet.

4. `feat(spec): add packet json projection`
   - Add JSON output with stable shape.

5. `docs(spec): add reusable prompt template scaffold`
   - Commit template derived from proven operator style.

6. `test(spec): add packet determinism and usability tests`
   - Snapshot and schema tests + basic zero-context run fixture.

## Direct Code Targets

- `src/commands/spec/mod.rs`
  - Add command wiring for `prompt`, `handoff`, `packet`.
- `src/commands/spec/internal`
  - Add packet renderers and schema mapping.
- `src/spec.rs`
  - Extend parsed model helpers used by packet generation.
- `layer/surface/build/feat/spec-prompt-handoff/PROMPT_TEMPLATE.md`
  - Baseline template artifact.
- `src/commands/spec/internal/tests.rs` (or equivalent)
  - Determinism, schema, and usability tests.

## Verification Plan

1. Run command help checks for new surfaces.
2. Snapshot text output for a representative spec fixture.
3. Validate JSON packet parse and required fields.
4. Confirm repeated renders are byte-stable.
5. Run a zero-context dry-run by feeding generated packet to an agent
   and checking it follows constraints and verification steps.

## Build Readiness

- [ ] Output schema documented in code comments/help text.
- [ ] Template committed and referenced by command output/help.
- [ ] Commands avoid mutating spec state.
- [ ] Tests cover stable output across reorder noise in spec files.

## Open Questions

- Should `spec packet` be a separate command or `spec prompt --json`?
  - Recommended default: separate `packet` command for clarity.

- Should handoff packets be saved to disk by default?
  - Recommended default: print to stdout; add optional `--out` later.

- Should packets include git diff/status context automatically?
  - Recommended default: no; keep command deterministic on spec/design
    sources only, and add explicit optional flags later.
