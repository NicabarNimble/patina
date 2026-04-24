# Mother Allium

Canonical Mother behavioral specs.

## Specs

- `mother-lifecycle.allium`
- `mother-child-toy-orchestration.allium`
- `mother-source-graph-routing.allium`
- `mother-secrets-session-coordination.allium`

## Generated artifacts (kept minimal)

- `*.plan.json` from `allium plan`

We intentionally do **not** keep extra gate/report artifacts here by default.

## Workflow (resume-style)

1. Author/update `.allium` spec.
2. Validate:
   - `allium check <spec>`
   - `allium analyse <spec>`
3. Generate/update obligations:
   - `allium plan <spec> > <spec>.plan.json`
4. Add/adjust Rust tests and tag tests with obligation ids:

```rust
// obligation id: rule-success.StartupSucceeded
// obligation ids: rule-success.StartupSucceeded + rule-entity-creation.StartupSucceeded.1
```

5. Maintain a human-readable coverage matrix in docs as needed.

## Commands

Refresh all Mother plans:

```bash
layer/allium/mother/regenerate-artifacts.sh
```

Run all checks:

```bash
allium check layer/allium/mother
allium analyse layer/allium/mother/mother-lifecycle.allium
allium analyse layer/allium/mother/mother-child-toy-orchestration.allium
allium analyse layer/allium/mother/mother-source-graph-routing.allium
allium analyse layer/allium/mother/mother-secrets-session-coordination.allium
```
