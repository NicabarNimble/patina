# Session 1: Basics & Motivation (1 hour)

**Objective**: Understand why Allium exists and get to first hands-on success.

## Agenda

| Time | Segment |
|------|---------|
| 0:00–0:15 | The problems Allium solves |
| 0:15–0:25 | The Allium philosophy |
| 0:25–0:35 | Install (skills + CLI) |
| 0:35–0:55 | Live demo: distill a real Patina module |
| 0:55–1:00 | Homework briefing |

## 1. The problems it solves (15 min)

Talking points — ground each one in something the team has felt:

- **Prompt drift within/across sessions.** Every new agent session
  re-explains the same intent, slightly differently. Patina exists partly to
  fight this (durable project memory); Allium fights it at the *behavior*
  level: the spec is the durable, re-loadable statement of what the system
  must do.
- **Code as "single source of truth" mixes intent with accidents.** Show
  `patina/src/commands/mother/daemon/dispatch.rs` — which parts are
  *required behavior* and which are incidental implementation? You can't
  tell from the code alone. `layer/allium/mother/mother-lifecycle.allium`
  answers it directly: `ManualStartRejectedWhenManagedBySupervisor` is
  intent; the specific error string formatting is an accident.
- **Ambiguities hidden in prose/markdown.** A design doc can say "the daemon
  stops cleanly" and never define what happens with a stale PID file. The
  Allium version is forced to decide: see `StopWhenStalePidFile` vs
  `StopWhenNoPidFile` vs `StopTimesOut` — three cases prose usually blurs
  into one.

## 2. The Allium philosophy (10 min)

- **Behavioral specification: what, not how.** Entities, states, rules,
  invariants — never SQL schemas, HTTP handlers, or UI layouts.
- A **durable, LLM-readable, contradiction-detecting contract** between
  humans, agents, and code:
  - *Durable* — lives in git (`layer/allium/`), survives sessions.
  - *LLM-readable* — agents load the spec instead of re-eliciting intent.
  - *Contradiction-detecting* — the CLI mechanically flags structural
    problems (dead entities, rules listening for events nothing provides),
    and the weed skill hunts spec/code contradictions; Session 3 covers
    exactly where the mechanical net ends and review takes over.
- **In this team's words** (`patina-mct/layer/core/spec-driven-design.md`):

  > Allium says what MCT is. Slate says what work is ready.
  > Beliefs/evidence say why. Code executes inside that boundary.

- Allium is **not a runtime** (`patina/layer/allium/README.md`). Code and
  tests implement and verify the contract; the spec never executes.

## 3. Quick install (10 min)

Everyone installs during the session — don't defer this.

```bash
# Skills — pick the flow that matches your harness
npx skills add juxt/allium
# or: Claude plugin commands (per the Allium README)

# CLI — pick one
brew tap juxt/allium && brew install allium
cargo install --locked allium-cli        # what patina CI uses
```

Smoke test against a real spec:

```bash
allium --version
cd ~/path/to/patina
allium check layer/allium/mother/mother-lifecycle.allium
allium analyse layer/allium/mother/mother-lifecycle.allium
```

Both should return empty `diagnostics`/`findings` JSON. If someone's install
fails, pair them up — do not let this block the demo.

## 4. Live demo: `/allium` entry point → distill (20 min)

Run in an agent session on **patina**, projected for the room:

1. Invoke the `/allium` entry point and show how it routes to the right
   skill for the situation.
2. **Distill** (backward, from code): pick something small and real —
   e.g. `patina-mct/scripts/install-allium-ci.sh` (a ~40-line shell script
   with genuinely interesting behavior: pinned version, checksum
   verification, platform gating, PATH registration).
3. Watch the skill produce entities/rules. Discuss as a group:
   - What did it capture that the script only implies?
     (e.g. *"install is refused on checksum mismatch"* is an explicit rule.)
   - What did it correctly leave out? (curl flags, temp dir names.)
4. Run `allium check` + `allium analyse` on the result. Green output =
   first success.

If time allows, show the contrast: **elicit** (forward, from intent) a
two-sentence feature idea into a skeleton spec.

## Homework

1. Install skills + CLI on your own machine (if not done in session).
2. Distill **one small existing function or module you own** into a spec.
   Keep it under ~60 lines of Allium. Suggested targets: a single `patina`
   CLI subcommand, one MCT script, one small state-carrying struct.
3. Run `allium check` and `allium analyse` until both are clean.
4. Bring the spec to Session 2 — we'll trade and read each other's.

## Resources

- Main Allium README and https://juxt.github.io/allium/
- Installation guide (skills + CLI)
- `patina/layer/allium/README.md` — how this repo hosts specs
- `patina/layer/allium/mother/mother-lifecycle.allium` — the reference
  example used throughout the course
