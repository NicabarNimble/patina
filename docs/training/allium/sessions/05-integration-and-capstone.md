# Session 5: Integration, Best Practices & Capstone (1.5–2 hours)

**Objective**: Wire Allium into the team's real workflow and kick off the
capstone.

## Agenda

| Time | Segment |
|------|---------|
| 0:00–0:30 | Integrating with tools, workflows, harnesses |
| 0:30–0:50 | Scaling to team/large codebases |
| 0:50–1:05 | Measuring impact |
| 1:05–1:30+ | Capstone kickoff |

## 1. Integration — how this team already does it (30 min)

This section is a guided tour of the **live integration** in patina and
patina-mct. Nothing hypothetical.

### The resume-style pipeline (patina)

From `patina/layer/allium/mother/README.md`:

```bash
# 1. Author/update the spec
$EDITOR layer/allium/mother/mother-lifecycle.allium

# 2. Validate
allium check   layer/allium/mother/mother-lifecycle.allium
allium analyse layer/allium/mother/mother-lifecycle.allium

# 3. Generate/update obligations
allium plan layer/allium/mother/mother-lifecycle.allium \
  > layer/allium/mother/mother-lifecycle.allium.plan.json
# (or refresh everything: layer/allium/mother/regenerate-artifacts.sh)

# 4. Tag Rust tests with obligation ids
```

```rust
// obligation id: rule-success.StartupSucceeded
// obligation ids: rule-success.StartupSucceeded + rule-entity-creation.StartupSucceeded.1
#[test]
fn startup_success_marks_control_plane_ready() { ... }
```

The obligation ID is the **traceability edge**: spec construct →
`plan.json` obligation → test comment → CI. Grep for `obligation` in
`patina/src/commands/mother/daemon/` to see it in the wild.

### CI enforcement

- **patina** (`.github/workflows/test.yml`): installs `allium-cli` via
  cargo, re-runs `regenerate-artifacts.sh`, then
  `git diff --exit-code -- layer/allium/mother/*.plan.json` — **plan drift
  fails the build**. You cannot silently change behavior obligations.
- **patina-mct**: `scripts/install-allium-ci.sh` pins the exact Allium
  release by version *and* SHA-256 (toolchain reproducibility), then
  tier-0 runs `allium check layer/allium`. Spec validity is a tier-0 gate,
  same class as compilation.

Discussion: what's *not* yet enforced? (`analyse` in patina-mct CI;
obligation-comment coverage isn't machine-checked — both are contribution
candidates.)

### Harness integration

- Skills run inside the agent harness; `patina context` and the Allium
  layer both feed agent sessions — the spec is loaded context, not
  documentation.
- `allium model` output is the hook for feeding specs into a broader
  control plane (Mother itself, or an external one like 8090).

### Minimal-artifact policy

`patina/layer/allium/README.md`: keep artifacts lean — **specs + plans
only**, no generated gate/report sprawl. Adopt the same policy for new
spec directories.

## 2. Scaling to team & large codebases (20 min)

- **Modular specs with explicit scope blocks.** The Mother specs split by
  concern (lifecycle / orchestration / routing / secrets), and each one's
  `-- Excludes:` names the others. That's the modularization mechanism:
  scope comments are the import boundaries.
- **A product map spec as the shared root.**
  `mct-product-map.allium` (~1900 lines) is the umbrella: principles,
  contracts, and open questions for the whole product, with feature specs
  below it. Pattern: *one map, many focused specs*.
- **Specs as backlog** (belief: `allium-as-business-backlog`): finishing an
  implementation slice ≠ finishing the spec. Un-implemented obligations
  become explicit follow-on specs, not silent debt.
- **Decision hygiene**: `open question` + `-- Decision:` keeps the map
  current without meetings. Review diffs of the spec like code.
- **Shared libraries**: extract recurring shapes (authority record,
  safe projection, terminal result) into the team patterns library rather
  than copy-pasting entity clusters between specs.

## 3. Measuring impact (15 min)

Pick a small set, measure before/after adoption, revisit quarterly:

| Signal | Proxy metric |
|--------|--------------|
| Fewer behavior bugs | Bugs traced to "spec was ambiguous/missing" in retro labels |
| Faster onboarding | Time for a new dev/agent session to make a correct behavioral change |
| Better LLM outputs | Weed findings per PR (drift trend); re-elicitation frequency |
| Traceability | % of spec obligations with tagged tests (grep-able today) |
| Toolchain health | CI failures from plan drift caught pre-merge |

Anti-metric warning: "number of specs" and "spec line count" reward sprawl —
the minimal-artifact policy exists for a reason.

## 4. Capstone project (kickoff — work spans the following 1–2 weeks)

**Brief**: [exercises/capstone.md](../exercises/capstone.md)

Team picks a real module or feature (in patina or patina-mct) and runs the
full cycle:

1. **Distill or elicit** the spec (distill if code exists, elicit if new).
2. **Implement** against it (or reconcile existing code).
3. **Weed + propagate**: alignment clean, `plan.json` generated,
   obligation-tagged tests added, CI green including drift check.
4. **Review + contribution idea**: present the spec to the team; each
   capstone must end with one written internals-level contribution proposal
   (from Session 4 Part B, refined).

Suggested scopes (small enough to finish, real enough to matter):

- A `patina` CLI subcommand family without a spec today (e.g. slate
  commands).
- One MCT slice from `layer/slate/work/` that lacks spec coverage.
- Promote an existing informal doc (a `layer/core/*.md` behavior section)
  into a checked spec.

Pairing: same junior/senior pairs as Session 2; facilitator reviews scope
before work starts (see [facilitator guide](../facilitator-guide.md)).

## Resources

- `patina/layer/allium/mother/README.md` — the pipeline this session teaches
- `patina/.github/workflows/test.yml`, `patina-mct/scripts/ci-tier0.sh`,
  `patina-mct/scripts/install-allium-ci.sh`
- `patina-mct/layer/core/spec-driven-design.md` — the governing doctrine
- [Capstone brief](../exercises/capstone.md)
