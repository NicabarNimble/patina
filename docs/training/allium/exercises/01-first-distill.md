# Exercise 01 — First Distill (Session 1 homework)

Distill one small existing function or module **you own** into an Allium
spec.

## Constraints

- Target ≤ ~60 lines of Allium.
- Must pass `allium check` and `allium analyse` (empty diagnostics and
  findings).
- Must contain: one entity with a state enum, at least three rules, and a
  scope block with at least one honest `-- Excludes:` line.

## Suggested targets

- A single `patina` CLI subcommand (e.g. one slate command).
- `patina-mct/scripts/install-allium-ci.sh` — small, but has real behavior:
  version pinning, checksum refusal, platform gating.
- One small state-carrying struct you've touched recently.

## Procedure

1. Run `/distill` on the target in an agent session (or write by hand —
   both are legitimate).
2. Read the output critically: delete anything that is an implementation
   accident (flag names, temp paths, log formatting).
3. Check what the distill *missed*: is there an error path the code handles
   that the spec doesn't mention? Add it as a rule or an
   `open question "..."`.
4. Validate:

   ```bash
   allium check   my-spec.allium
   allium analyse my-spec.allium
   ```

5. Bring it to Session 2 — pairs trade specs and try to find one behavior
   the spec gets wrong or omits.

## Self-check

- Could someone reimplement the module *from the spec alone* and preserve
  the behavior that matters?
- Could the implementation change storage/CLI framework without touching
  the spec? If not, you leaked implementation.
