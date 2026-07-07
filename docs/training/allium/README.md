# Allium Training Curriculum

A complete training program to take the team from Allium beginners to
proficient users *and* contributors who understand how Allium works under the
hood — anchored in this codebase, not toy examples.

## Training Goals

- **Basics** — comfortable installing Allium, using the core skills, and
  writing/reading Allium specs.
- **Deep dive** — understand the language design, why it works, the internals
  (parser, CLI, skills), and how to extend or customize it.
- **Proficiency** — confidently integrate Allium into the Patina/MCT
  workflow, debug issues, and contribute improvements (skills, patterns,
  tools).

## Format

4–6 sessions (1–2 hours each) + hands-on labs + a capstone project.
Mix of lectures, live demos, pair programming, and individual exercises.
Every session ends with coding time.

| # | Session | Length | Focus |
|---|---------|--------|-------|
| 1 | [Basics & Motivation](sessions/01-basics-and-motivation.md) | 1h | Why Allium exists; install; first distill |
| 2 | [Core Skills & Workflow](sessions/02-core-skills-and-workflow.md) | 1.5h | The Allium loop: elicit, distill, propagate, weed, tend |
| 3 | [Language Deep Dive](sessions/03-language-deep-dive.md) | 1.5–2h | v3 syntax, semantics, analysis, anti-patterns |
| 4 | [Internals & Architecture](sessions/04-internals-and-architecture.md) | 2h | Skills, CLI parser, analysis passes, extensibility |
| 5 | [Integration, Best Practices & Capstone](sessions/05-integration-and-capstone.md) | 1.5–2h | CI, obligations, scaling, capstone kickoff |
| 6 | [Contributing & Advanced Topics](sessions/06-contributing-and-advanced.md) (optional) | 1.5h | OSS workflow, custom skills, large specs |

Supporting material:

- [Facilitator guide](facilitator-guide.md) — teaching tips, pairing plan,
  assessment rubric.
- [Cheat sheets](cheatsheets/) — syntax, CLI, and the Allium loop on one page
  each.
- [Exercises](exercises/) — starter specs and worked solutions, all validated
  with `allium check` / `allium analyse`.
- [Slides](slides/session-03-slides.md) — Session 3 expanded into a full
  36-slide deck with speaker notes: one running example grown from a
  one-sentence behavior to production patterns and toolchain internals.

## Why this curriculum is Patina/MCT-flavored

Allium is already load-bearing here. The course uses the real artifacts:

- **patina** — `layer/allium/` is the canonical home for behavioral specs.
  The Mother specs (`layer/allium/mother/*.allium`) and their generated
  `*.plan.json` obligations are the reference material for Sessions 2, 3,
  and 5. CI installs `allium-cli` and fails on plan drift.
- **patina-mct** — `layer/allium/mct-product-map.allium` is a full product
  map written in Allium (decisions, open questions, contracts, surfaces).
  CI pins the Allium release (`scripts/install-allium-ci.sh`) and runs
  `allium check layer/allium` in tier-0.
- **Doctrine** — `patina-mct/layer/core/spec-driven-design.md`:
  *"Allium says what MCT is. Slate says what work is ready. Beliefs/evidence
  say why. Code executes inside that boundary."*
- **Beliefs** — `allium-as-business-backlog` and
  `allium-as-agent-display-lisp` in `layer/surface/epistemic/beliefs/`
  capture how the team already thinks about Allium.

## Prerequisites

Before Session 1, each attendee should have:

1. A working Rust toolchain (`rustup`, matching `rust-toolchain.toml`).
2. A clone of `patina` and `patina-mct`.
3. Claude Code (or another agent harness) with the Allium skills available —
   see Session 1 install steps.

## Installing the toolchain

Two halves: the **skills** (agent-side) and the **CLI** (analysis-side).

```bash
# Skills (pick one)
npx skills add juxt/allium            # generic skills install
# or the Claude Code plugin flow — see Session 1

# CLI (pick one)
brew tap juxt/allium && brew install allium   # macOS
cargo install --locked allium-cli             # what patina CI does
# CI-pinned binary (linux x86_64): patina-mct/scripts/install-allium-ci.sh
```

Verify:

```bash
allium --version           # course material validated against 3.5.0
allium check /home/user/patina/layer/allium/mother/mother-lifecycle.allium
```

## Assessment

Completion = a reviewed spec + a small contribution. See the
[facilitator guide](facilitator-guide.md#assessment) for the rubric.
