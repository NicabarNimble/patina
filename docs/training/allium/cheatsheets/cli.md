# Allium CLI Cheat Sheet

All commands emit JSON (`diagnostics` = structural, `findings` = semantic).
Course material validated against `allium 3.5.0`.

## Install

```bash
brew tap juxt/allium && brew install allium   # macOS
cargo install --locked allium-cli             # what patina CI does
patina-mct/scripts/install-allium-ci.sh       # pinned + SHA-verified (linux CI)
allium --version
```

## The five commands

```bash
allium check   <spec|dir>    # validate structure; empty diagnostics = pass
allium analyse <spec>        # reachability, deadlocks, conflicts, data flow
allium parse   <spec>        # AST as JSON
allium plan    <spec>        # derive test obligations
allium model   <spec>        # extract domain model for downstream tools
allium help <command>        # per-command help
```

## Daily one-liners

```bash
# validate everything in the canonical layer
allium check layer/allium

# full analysis of one spec, findings only
allium analyse layer/allium/mother/mother-lifecycle.allium | jq '.findings'

# regenerate a plan (never hand-edit plan.json)
allium plan spec.allium > spec.allium.plan.json

# refresh all Mother plans (patina)
layer/allium/mother/regenerate-artifacts.sh

# list obligation ids for test tagging
allium plan spec.allium | jq -r '.obligations[].id'

# inspect one rule's AST
allium parse spec.allium | jq '.. | objects | select(.name? == "StopRunningDaemon")'
```

## Obligation tagging (resume-style, per layer/allium/mother/README.md)

```rust
// obligation id: rule-success.StartupSucceeded
// obligation ids: rule-success.StartupSucceeded + rule-entity-creation.StartupSucceeded.1
#[test]
fn startup_success_marks_control_plane_ready() { /* ... */ }
```

## CI gates in this org

```bash
# patina (.github/workflows/test.yml)
cargo install --locked allium-cli
layer/allium/mother/regenerate-artifacts.sh
git diff --exit-code -- layer/allium/mother/*.plan.json   # plan drift fails CI

# patina-mct (scripts/ci-tier0.sh)
allium check layer/allium                                  # tier-0 gate
```

## Reading output

| Field | Meaning |
|-------|---------|
| `diagnostics: []` | structurally valid |
| `findings: []` | no process-level issues |
| `severity` | `error` / `warning` / `info` — any diagnostic makes check exit 1 |
| `source_span {start,end}` | byte offsets into the spec — same spans in parse/plan/LSP |
| `obligations[].id` | stable ID: `<category>.<construct>[.n]` — goes in the test comment |
| `obligations[].category` | e.g. `entity_fields`, `rule-success`, `rule-failure`, `temporal`, `surface-provides` |

Diagnostics you'll actually meet:

| Code | Meaning |
|------|---------|
| `allium.rule.unreachableTrigger` | rule listens for an event no surface provides / no rule emits |
| `allium.entity.unused` | entity declared, never referenced |
| `allium.field.unused` | field declared, never referenced |

## Know the net's holes (validated on 3.5.0)

A **typo'd field name** in `requires`/`ensures` and **two rules with
identical guards and contradictory `ensures`** currently pass both check
and analyse silently. Green CLI ≠ correct spec — that's what `/weed` and
review are for. (And a good first upstream contribution.)
