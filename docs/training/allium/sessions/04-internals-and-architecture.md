# Session 4: Internals & Architecture (2 hours)

**Objective**: Understand *how* Allium works so the team can debug, extend,
and contribute — not just consume.

## Agenda

| Time | Segment |
|------|---------|
| 0:00–0:20 | Component map: skills, CLI, editor integrations |
| 0:20–0:45 | How skills work |
| 0:45–1:15 | Parser & validation pipeline |
| 1:15–1:35 | Extensibility |
| 1:35–2:00 | Exercise: trace a spec through the CLI + propose an extension |

## 1. Component map (20 min)

Allium is three cooperating layers with the **spec file as the only shared
contract** between them:

```text
┌─────────────────────────────────────────────────────────┐
│ Skills / agents (Markdown instructions an LLM follows)  │
│   /allium /elicit /distill /propagate /weed /tend       │
│   → read & write .allium files, invoke the CLI          │
├─────────────────────────────────────────────────────────┤
│ CLI (Rust: allium-cli)                                  │
│   check | analyse | parse | plan | model                │
│   → deterministic parsing + structural/semantic analysis│
├─────────────────────────────────────────────────────────┤
│ Editor integrations                                     │
│   LSP, Tree-sitter grammar, diagnostics, autofix        │
│   → same parser core, surfaced at edit time             │
└─────────────────────────────────────────────────────────┘
```

Design consequence worth dwelling on: the LLM parts (skills) are
**probabilistic**, the CLI is **deterministic**. The loop works because
every probabilistic step is followed by a deterministic gate — an agent can
hallucinate a rule, but `allium check`/`analyse` will not hallucinate a
pass. This is the same decide/effect separation MCT uses (kernel decides,
adapters perform), which makes it a familiar shape for this team.

## 2. How skills work (25 min)

Open the installed skill files and read them as artifacts:

- **Markdown-based instructions LLMs follow.** A skill is prompt
  engineering under version control: role framing, procedure, output
  format, guardrails. There is no skill "runtime" — the harness loads the
  Markdown into context.
- **Routing** — the `/allium` entry point classifies the situation (new
  intent? existing code? drift? change request?) and hands off to
  elicit/distill/weed/tend. Compare with how Patina routes agent guidance
  via `patina context`.
- **Prompt-engineering patterns to spot while reading:**
  - Forced interrogation (elicit *must* surface `open question`s rather
    than invent answers).
  - Output validation loops ("run `allium check`; if diagnostics, fix and
    re-run") — the deterministic gate embedded in the instructions.
  - Anti-injection framing (spec content is data, not instructions).
- **Autonomous agents** — because skills are files, they compose into
  larger flows (e.g. a CI agent that runs weed on every PR). This is the
  hook for Session 5's harness discussion.

**Live reading**: pull up the distill skill and annotate its structure as a
group — where's the procedure? where are the guardrails? what would you
change?

## 3. Parser & validation (30 min)

Reference: parser code and docs in the **allium-tools** repo.

- **Recursive descent** — one parse function per construct
  (entity/value/rule/contract/surface/config/given), hand-written rather
  than grammar-generated, so diagnostics can be precise and recovery
  deliberate.
- **AST** — see it directly:

  ```bash
  allium parse layer/allium/mother/mother-lifecycle.allium | jq '.' | less
  ```

  Walk one rule from source text to its AST node: trigger, guard
  expressions, effect list, source spans. Note `source_span` byte offsets —
  the same spans appear in `plan.json` obligations and LSP diagnostics
  (one parser, three consumers).

- **Validation passes** (roughly in order):
  1. Parse → syntax diagnostics.
  2. Name/type resolution — every field reference resolves against a
     declared entity/value; enum literals must belong to the field's enum.
  3. Structural checks (`check`) — construct-level well-formedness, unused
     declarations, trigger reachability.
  4. Process analysis (`analyse`) — the documented passes are data flow
     tracing, edge reachability, deadlock detection, conflict detection,
     and invariant verification (`allium help analyse`). Recall Session 3's
     calibration: several semantic defect classes still pass silently in
     3.5.0 — reading this layer's source in allium-tools to see what each
     pass actually covers is exactly the kind of contribution
     reconnaissance this session is for.
- Output is **JSON everywhere** — `diagnostics` (structural) vs `findings`
  (semantic). Machine-readable by design: CI, editors, and agents consume
  the same output. `patina-mct/scripts/ci-tier0.sh` is a one-line consumer:
  `allium check layer/allium`.

## 4. Extensibility (20 min)

Three tiers, cheapest first:

1. **New skills or patterns** — Markdown only, no Rust. A team-specific
   pattern (e.g. "MCT authority-record pattern" from Session 3) can ship as
   a skill fragment or a patterns-library entry. Lowest bar to contribute.
2. **Custom analysis rules** — Rust, in the analysis layer. Model: a pass
   consumes the AST/state graph and emits findings JSON. Candidate ideas
   from this codebase:
   - "every `entity` with a state enum must have a terminal state";
   - "every `surface` exposing a field marked sensitive must have a
     matching contract invariant";
   - "warn on rules whose `ensures` references config not declared in
     `config {}`".
3. **Harness integration** — feed `allium model` / `plan` output into a
   control plane. Local example: Patina's Mother could consume plan
   obligations as readiness evidence; an external example is feeding specs
   into a control plane like 8090. The JSON output contract is the
   integration surface — no CLI changes needed.

## 5. Exercise: trace + propose (25 min)

**Part A — trace** (pairs): take your Session 2 toy-grant spec and follow
it through the pipeline, capturing each stage:

```bash
cd docs/training/allium/exercises/02-toy-grant-lifecycle/solution
allium parse   toy-grant-lifecycle.allium > /tmp/ast.json
allium check   toy-grant-lifecycle.allium
allium analyse toy-grant-lifecycle.allium
allium plan    toy-grant-lifecycle.allium | jq -r '.obligations[].id'
```

For one rule, write down: its AST shape, which analyse findings *could*
fire on it, and which obligations `plan` derived from it.

**Part B — propose** (individual, written): one paragraph proposing a small
extension — a new diagnostic, analysis rule, or pattern. Must include: the
defect it catches, a spec snippet that triggers it, and the finding JSON
you'd expect. Best proposals become Session 6 / capstone contribution
candidates.

## Resources

- allium-tools repo: docs, AGENTS.md, parser code, contributing guide
- `allium help <command>` for each pipeline stage
- [CLI cheat sheet](../cheatsheets/cli.md)
