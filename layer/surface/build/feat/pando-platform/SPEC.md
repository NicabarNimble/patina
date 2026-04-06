---
type: feat
id: pando-platform
status: draft
created: 2026-04-06
parent: child-construction-canon
sessions:
  origin: 20260405-133644-511306000
beliefs:
  - "[[pandos-are-products-children-are-compute]]"
  - "[[pando-is-composed-children]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[wasi-is-foundation-not-option]]"
blocked_by:
  - sdk-wasi-trait-alignment
related:
  - layer/surface/build/feat/child-construction-canon/SPEC.md
  - children/spec-manager/
  - mother/src/builtin_children.rs
  - src/commands/spec/
  - src/main.rs
exit_criteria:

  - id: pp1-pando-manifest
    text: "`pando.toml` format defined and parsed by Mother. Declares name, description, children, composition wiring, and commands."
    checked: false

  - id: pp2-mother-pando-registry
    text: "Mother reads `pando.toml` files, builds a pando registry, and maps command namespaces to pandos. Rejects registration when a command namespace collides with an existing pando."
    checked: false

  - id: pp3-cli-command-discovery
    text: "The `patina` binary asks Mother for registered pando commands. Unknown commands route to Mother for pando dispatch. `patina --help` shows native commands; `patina <pando> --help` shows pando commands served from the manifest."
    checked: false

  - id: pp4-pando-to-child-dispatch
    text: "Mother receives a pando command, resolves which child handles the action, calls `handle(action, payload)` on that child, returns the result to the CLI."
    checked: false

  - id: pp5-folder-text-retrofit
    text: "`folder-text-to-parquet` has a `pando.toml` and is managed by Mother as a pando. No CLI commands — pipeline pando, proves basic pando lifecycle (register, list, health)."
    checked: false

  - id: pp6-slate-child-built
    text: "Slate-manager child exists as a proper WASM child using the SDK. Uses toys: `wasi:filesystem`, `wasi:keyvalue`, `wasi:logging`, `patina:git`. Handles all spec lifecycle actions (list, show, check, create, promote, complete, abandon, pause, resume, block, archive, rename, reopen, set, next, history, split, prompt, handoff)."
    checked: false

  - id: pp7-slate-pando-commands
    text: "Slate pando has a `pando.toml` declaring CLI commands. `patina slate list`, `patina slate next`, `patina slate complete <id>`, `patina slate archive <id>` all work end-to-end through the pando dispatch path."
    checked: false

  - id: pp8-git-toy-additions
    text: "`patina:git` WIT interface extended with `rm` (remove files/dirs from tree) and `for-each-ref` (query tags). Host implementations in Mother."
    checked: false

  - id: pp9-builtin-dispatch-removed
    text: "`BuiltinChild::SpecManager` and `BuiltinChildAction::SpecDispatch` removed. `src/commands/spec/internal/` dead code removed. All spec/slate commands route through the pando platform."
    checked: false

  - id: pp10-spec-compat
    text: "`patina spec` works as an alias for `patina slate` during migration. Existing scripts and skills using `patina spec` continue to function."
    checked: false

  - id: pp11-compile-proof
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass. `patina slate list`, `patina slate check child-construction-canon`, and `patina slate archive` work with real specs."
    checked: false
---
# feat: Pando Platform — Composed Children as User-Facing Products

## Problem

Patina has children (WASM compute) and toys (sandbox openings), but no way to
compose them into user-facing products. The spec system is a builtin function
dispatch inside Mother — not a child, not composable, not installable. Adding
new user-facing features means modifying the binary.

The `folder-text-to-parquet` pipeline proved children compose through events,
but it has no user-facing surface — no CLI commands, no install/uninstall
lifecycle. A third-party developer cannot build a pando that adds commands
to `patina`.

## Goal

Build the pando platform: composed groups of children that appear as one
capability to the user. Mother manages the pando registry. The binary
discovers and routes pando commands. Users install pandos, not children.

Prove it by building the slate pando — the first interactive pando that
replaces the spec-manager builtin dispatch with a proper WASM child
behind the pando platform.

## Status

Draft.

## Non-Goals

- Third-party pando publishing/installation infrastructure (future spec)
- Pando versioning and upgrade lifecycle (future spec)
- LLM-driven pando assembly from registry (ccc7, future)
- Migrating other builtin commands to pandos (one at a time, after slate proves the platform)

## Pando Manifest Format

```toml
[pando]
name = "slate"
description = "Plan, track, and archive build work with exit criteria"
version = "0.1.0"

[[children]]
name = "slate-manager"

[commands.list]
description = "List all slates"
child = "slate-manager"
action = "list"

[commands.show]
description = "Show a slate"
child = "slate-manager"
action = "show"

[commands.check]
description = "Check exit criteria"
child = "slate-manager"
action = "check"

[commands.next]
description = "Recommend next slate to work on"
child = "slate-manager"
action = "next"

[commands.create]
description = "Create a new slate"
child = "slate-manager"
action = "create"

[commands.complete]
description = "Complete an active slate"
child = "slate-manager"
action = "complete"

[commands.archive]
description = "Archive a completed slate"
child = "slate-manager"
action = "archive"

# ... remaining commands follow same pattern
```

A pipeline pando with no CLI commands:

```toml
[pando]
name = "folder-text-to-parquet"
description = "Ingest local text files into parquet with provenance"
version = "0.1.0"

[[children]]
name = "file-system-monitor"

[[children]]
name = "content-extractor"

[[children]]
name = "schema-enforcer"

[[children]]
name = "dedup-filter"

[[children]]
name = "record-writer"

[[children]]
name = "lakehouse-catalog"

[composition]
wiring = [
  "file-system-monitor.file.found -> content-extractor",
  "content-extractor.record.extracted -> schema-enforcer",
  "schema-enforcer.record.validated -> dedup-filter",
  "dedup-filter.record.ready -> record-writer",
  "record-writer.file.written -> lakehouse-catalog",
]
```

## Mother's Role

Mother is the OS. Pandos are apps.

1. **Registration** — Mother reads `pando.toml` from `~/.patina/pandos/<name>/`.
   First-party pandos ship with the binary and are seeded on first run (same
   pattern as interface templates). Third-party pandos are installed later.

2. **Command registry** — Mother maps command namespaces to pandos. `slate` →
   slate pando. If two pandos try to register the same namespace, Mother rejects
   the second one.

3. **Dispatch** — CLI sends `{ pando: "slate", command: "complete", args: {...} }`
   to Mother. Mother resolves which child handles the action, calls
   `handle(action, payload)` on that child, returns the result.

4. **Command schema** — Mother serves the pando's command definitions (names,
   descriptions, argument shapes) to the binary so it can render `--help`
   without dispatching to the child.

5. **Lifecycle** — Mother tracks pando health (are all children loaded?),
   supports `patina pando list` to show installed pandos.

## Binary's Role

The binary is the native shell. It has:

- **Fixed native commands** — `init`, `scrape`, `context`, `mother`, `pando`,
  etc. These always work, no Mother required.
- **Catch-all routing** — any command the binary doesn't recognize is forwarded
  to Mother as a pando command lookup. If Mother isn't running: "start Mother
  first."
- **Help rendering** — `patina slate --help` asks Mother for the slate pando's
  command schema and renders it.

## Solution Phases

### Phase A — pando.toml parser and Mother registry

- Define `pando.toml` schema (serde deserialization)
- Mother reads pandos from `~/.patina/pandos/`
- Mother builds command registry with collision detection
- `patina pando list` shows registered pandos

### Phase B — CLI routing

- Binary catch-all for unknown commands
- Binary queries Mother for command schema (for `--help`)
- Binary sends pando commands to Mother, prints response
- Error handling when Mother isn't running

### Phase C — retrofit folder-text-to-parquet

- Create `pando.toml` for folder-text-to-parquet
- Mother registers it as a pando (no commands, pipeline only)
- Proves basic pando lifecycle

### Phase D — git toy additions

- Add `rm` to `patina:git` WIT + host implementation
- Add `for-each-ref` to `patina:git` WIT + host implementation

### Phase E — slate child

- Port spec logic from `src/commands/spec/internal/` to
  `children/slate-manager/src/lib.rs`
- Child uses SDK with toys: `wasi:filesystem`, `wasi:keyvalue`,
  `wasi:logging`, `patina:git`
- All 20 spec actions handled via `handle(action, payload)`
- Tests against real spec files on disk

### Phase F — slate pando

- Create `pando.toml` for slate pando
- Mother registers slate commands
- `patina slate list`, `patina slate complete`, etc. work end-to-end
- `patina spec` aliased to `patina slate` for migration

### Phase G — cleanup

- Remove `BuiltinChild::SpecManager` and `BuiltinChildAction::SpecDispatch`
- Remove `src/commands/spec/internal/` (dead code)
- Remove stale patterns DB dependency

## Resolved Decisions

- Pandos own CLI surfaces, not children. Children are internal compute.
- Command namespaces are per-pando, enforced by Mother. No collisions allowed.
- Mother must be running for pando commands. Native binary commands work without
  Mother (same as apps needing an OS).
- `patina spec` stays as alias for `patina slate` — no breaking change for
  existing scripts and skills.
- The slate pando is a single-child pando today. It may split later if natural
  seams emerge, but we don't force decomposition before the build proves it.

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib
patina pando list
patina slate list
patina slate check child-construction-canon
patina slate next
patina slate complete <id>
patina slate archive <id>
```

## Build Readiness

Phase A is ready. No blockers. Existing children and toys cover the needs.
`patina:git` needs two additions (Phase D) but those don't block Phase A-C.
