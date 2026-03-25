# Plugin Vocabulary Inventory (Grouped)

Audit timestamp: 2026-03-25

Match pattern used:

- `\bplugin(s)?\b`
- `plugin.toml`
- `plugins/`
- `Plugin[A-Z]`

High-level counts (refreshed 2026-03-25):

- `src/`: 282
- `resources/`: 27
- top-level active docs: 7
- `layer/surface/`: 658
- `layer/sessions/`: 1109

## 1) Runtime compatibility surface (code)

Primary files:

- `src/main.rs`
- `src/paths.rs`
- `src/child/scaffold.rs`
- `src/child/internal/{command.rs,task.rs,knowledge_child.rs,host_support.rs,pipeline.rs}`

Notes:

- This bucket mixes real compatibility shims with comment/message wording drift.
- `src/main.rs` still exposes `Plugin` command naming to users.

## 2) Grammar pipeline naming

Primary files:

- `src/commands/setup/grammars.rs`
- `src/commands/scrape/code/extract.rs`
- `src/commands/bench/grammar.rs`
- `resources/grammar-defaults.toml`
- `resources/scripts/grammar-compare.sh`

Notes:

- A lot of references are grammar-specific and may need a dedicated naming policy
  (for example whether to keep "plugin" as a grammar lane term or fully migrate to "child").

## 3) Workspace and tooling path references

Primary files:

- `Cargo.toml`
- `resources/git/pre-push-checks.sh`
- `resources/scripts/check-crate-names.sh`
- `resources/scripts/check-single-sdk-surface.sh`

Notes:

- These references can be either expected (current folder layout) or stale assumptions.
- Scope lock for this inventory bucket: normalize references only; no physical `plugins/` -> `children/` move in this spec.
- Current tree includes child/runtime crates under both `plugins/` and `children/`; consolidation is separate-spec work.

## 4) Templates and scaffolding

Primary paths:

- `resources/templates/plugin/*`
- callsites in `src/child/scaffold.rs`

Notes:

- Template path naming and generated wording should be decided together.

## 5) Active docs (user-facing)

Primary files:

- `README.md`
- `sdk/patina-sdk/README.md`
- `OPENCODE.md`
- `children/README.md`
- `plugins/README.md`

Notes:

- Some references are intentional compatibility notes and should not be removed blindly.

## 6) Living doctrine/spec docs (`layer/surface`)

Large cluster includes:

- refactor specs/design docs
- epistemic beliefs
- audit reports

Notes:

- Needs triage into "canonical doctrine" vs "historical record".

## 7) Session archives (`layer/sessions`)

Notes:

- Historical evidence; default should be no rewrites.
- If needed, add forward-looking notes rather than mutating old records.
