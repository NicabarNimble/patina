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
blocked_by: []
related:
  - layer/surface/build/feat/child-construction-canon/SPEC.md
  - children/spec-manager/
  - mother/src/builtin_children.rs
  - src/commands/spec/
  - src/main.rs
exit_criteria:

  - id: pp1-pando-manifest
    text: "`pando.toml` format defined and parsed by Mother. Declares name, description, children, composition wiring, and commands."
    checked: true

  - id: pp2-mother-pando-registry
    text: "Mother reads `pando.toml` files, builds a pando registry, and maps command namespaces to pandos. Rejects registration when a command namespace collides with an existing pando or a native binary command."
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
    text: "Slate-manager child exists as a proper WASM child using the SDK. Uses toys: `wasi:filesystem`, `patina:keyvalue`, `patina:logging`, `patina:git`. Handles all spec lifecycle actions (list, show, check, create, promote, complete, abandon, pause, resume, block, archive, rename, reopen, set, next, history, split, prompt, handoff)."
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
args = [
  { name = "status", type = "string", required = false, description = "Filter by status" },
  { name = "json", type = "flag", description = "Output as JSON" },
]

[commands.show]
description = "Show a slate"
child = "slate-manager"
action = "show"
args = [
  { name = "id", type = "string", required = true, positional = true, description = "Slate ID" },
  { name = "json", type = "flag", description = "Output as JSON" },
]

[commands.check]
description = "Check exit criteria"
child = "slate-manager"
action = "check"
args = [
  { name = "id", type = "string", required = true, positional = true, description = "Slate ID" },
]

[commands.next]
description = "Recommend next slate to work on"
child = "slate-manager"
action = "next"

[commands.create]
description = "Create a new slate"
child = "slate-manager"
action = "create"
args = [
  { name = "id", type = "string", required = true, positional = true, description = "Slate ID" },
  { name = "type", type = "string", required = true, description = "Slate type (feat, fix, refactor, explore)" },
]

[commands.complete]
description = "Complete an active slate"
child = "slate-manager"
action = "complete"
args = [
  { name = "id", type = "string", required = true, positional = true, description = "Slate ID" },
]

[commands.archive]
description = "Archive a completed slate"
child = "slate-manager"
action = "archive"
args = [
  { name = "id", type = "string", required = true, positional = true, description = "Slate ID" },
]

# ... remaining commands follow same pattern
```

Binding-oriented extension (migration target):

```toml
[[children]]
name = "slate-manager"

[children.bindings.filesystem.spec_root]
guest_path = "/specs"
host_path = "./layer/surface/build"
mode = "read-write"

[children.bindings.keyvalue.state]
namespace = "slate"

[children.bindings.git.repo]
scope = "project"

[children.bindings.http.github]
domains = ["api.github.com"]
```

During migration, `needs.toys` remains the intent surface while bindings become
the enforcement surface. The steady state is one-to-one resource bindings with
explicit scopes and no umbrella capability labels.

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
   slate pando. Three-tier collision rejection:
   - **Native collision** — Mother knows the binary's native command list. A pando
     cannot register a namespace that matches a native command (`init`, `mother`,
     `scrape`, `context`, `pando`, `belief`, etc.). Error: `"namespace '<name>'
     is a native command"`.
   - **Pando collision** — if two pandos try to register the same namespace,
     Mother rejects the second one. Error: `"namespace '<name>' already
     registered by pando '<other>'"`.
   - **Alias collision** — migration aliases (`spec` → `slate`) are registered
     in the same namespace. A pando cannot claim `spec` while `slate` holds it
     as an alias. Error: `"namespace '<name>' is an alias for pando '<other>'"`.
   The binary sends its native command list to Mother at startup so the
   collision check is always current.

3. **Dispatch** — CLI sends `{ pando: "slate", command: "complete", args: {...} }`
   to Mother. Mother resolves which child handles the action, calls
   `handle(action, payload)` on that child, returns the result.

4. **Command schema** — Mother serves the pando's command definitions to the
   binary so it can render `--help` without dispatching to the child. Schema
   includes: command name, description, and typed argument list. Each arg has
   `name`, `type` (string, flag, int), `required`, `positional`, and
   `description`. This is parsed from `pando.toml` at registration — the child
   is never called for help rendering.

5. **Lifecycle** — Mother tracks pando health (are all children loaded?),
   supports `patina pando list` to show installed pandos.

## Binary's Role

The binary is the native shell. It has:

- **Fixed native commands** — `init`, `scrape`, `context`, `mother`, `pando`,
  etc. These always work, no Mother required. The native command list is
  hardcoded in the binary and cannot be overridden by pandos.
- **Catch-all routing** — any command the binary doesn't recognize is forwarded
  to Mother as a pando command lookup. If Mother isn't running: "start Mother
  first."
- **Help rendering** — `patina slate --help` asks Mother for the slate pando's
  command schema and renders it.

### Native vs Pando Boundary

Commands that exist today as native binary commands (`spec`, `belief`, etc.)
become Mother-dependent when migrated to pandos. This is an intentional trade:
the pando version is composable, extensible, and WASM-sandboxed, but requires
Mother running. The migration contract:

1. Native command stays in the binary as an alias that forwards to the pando.
2. If Mother is down, the alias prints: `"<command> requires Mother — run
   'patina mother start'"`. No silent fallback to stale native code.
3. Migration is one command at a time (`spec` → `slate` first). Each migration
   is its own spec with a compat alias exit criterion.
4. Commands that must work without Mother (init, mother start, help) never
   become pandos.

## Solution Phases

### Phase 0 — binding alignment guardrail

- Keep `needs.toys` for compatibility.
- Normalize runtime capability naming to one-to-one mappings
  (`filesystem`→`host_filesystem`, `keyvalue`→`host_keyvalue`,
  `sql`→`host_sql`) with aliases only as migration shims.
- Add tests proving child manifests with filesystem/keyvalue/sql scopes map to
  explicit runtime capabilities.

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
- Child uses SDK with toys: `wasi:filesystem`, `patina:keyvalue`,
  `patina:logging`, `patina:git`
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
- Command namespaces are per-pando, enforced by Mother. Three-tier collision
  rejection: native commands, pando-vs-pando, and alias-vs-pando.
- Mother must be running for pando commands. Native binary commands work without
  Mother (same as apps needing an OS). Migrated commands that become pandos
  print a clear "requires Mother" error when Mother is down — no silent
  fallback to stale native code.
- `patina spec` stays as alias for `patina slate` — no breaking change for
  existing scripts and skills.
- The slate pando is a single-child pando today. It may split later if natural
  seams emerge, but we don't force decomposition before the build proves it.

## Registry Contract

### Binary ↔ Mother Protocol

The binary sends a `PandoRegistryInit` message to Mother at startup:

```json
{
  "protocol_version": 1,
  "native_commands": ["init", "scrape", "context", "mother", "pando", "belief", ...],
  "binary_version": "0.10.0"
}
```

Mother responds with `PandoRegistryState`:

```json
{
  "protocol_version": 1,
  "pandos": [
    {
      "name": "slate",
      "status": "loaded",
      "commands": ["list", "show", "check", "next", ...],
      "aliases": ["spec"]
    }
  ]
}
```

Version rules:
- `protocol_version` is an integer, incremented on breaking changes.
- Binary and Mother must agree on protocol version. If they disagree, Mother
  returns an error with both versions and the binary prints:
  `"Mother protocol v{m} incompatible with binary v{b} — upgrade patina"`.
- Adding new optional fields to payloads is non-breaking (same version).
- Removing fields, changing types, or changing semantics is breaking (bump version).

### Arg Schema Types

Canonical arg types for `pando.toml` command definitions:

| Type | Rust maps to | CLI example |
|---|---|---|
| `string` | `String` | `--status active`, positional `<id>` |
| `flag` | `bool` | `--json` (present = true, absent = false) |
| `int` | `i64` | `--limit 10` |
| `strings` | `Vec<String>` | `--tag foo --tag bar` (repeatable) |

Rules:
- `required: true` args that are missing produce: `"missing required argument '<name>'"`.
- `positional: true` args are consumed in declaration order. At most one positional
  arg per command (keeps parsing unambiguous).
- Unknown args produce: `"unknown argument '<name>' for <pando> <command>"`.
- Type coercion: none. `--limit foo` for an `int` arg produces:
  `"argument '<name>' expects int, got 'foo'"`.
- No default values in v1. Omitted optional args are absent, not defaulted.
  Children handle absence in their action logic.

### Resolution Order

Command resolution follows strict precedence, highest to lowest:

1. **Native command** — hardcoded in binary, always wins.
2. **Alias** — migration alias registered by a pando (e.g. `spec` → `slate`).
   Resolves to the owning pando and dispatches normally.
3. **Pando namespace** — registered pando command (e.g. `slate list`).
4. **Unknown** — not found anywhere. Error: `"unknown command '<name>'"`.

When Mother is down:
- Native commands work normally.
- Alias and pando commands print: `"'<command>' requires Mother — run 'patina mother start'"`.
- `patina spec --help` when Mother is down: same error. No cached help fallback.

### First-Party Pando Seeding

First-party pandos ship embedded in the binary and are seeded to
`~/.patina/pandos/<name>/` on first run or when missing.

Conflict policy:
- **Missing** — seed the pando directory and `pando.toml`. Normal case.
- **Exists, same version** — skip. No-op.
- **Exists, older version** — overwrite. First-party pandos are always replaced
  by the binary's version. The binary owns these. Log: `"updated pando '<name>'
  from v{old} to v{new}"`.
- **Exists, newer version** — this means a newer binary seeded it previously,
  then the user downgraded. Still overwrite — binary version is authoritative
  for first-party pandos. Warn: `"downgraded pando '<name>' from v{old} to v{new}"`.
- **Exists, user-modified** — no detection in v1. First-party pandos are not
  user-editable. If users want custom behavior, they create a separate pando
  with a different name.

Overwrite scope: seeding only touches `pando.toml` and child WASM binaries
inside `~/.patina/pandos/<name>/`. It never touches:
- `~/.patina/pandos/<name>/state/` — runtime state (keyvalue data, caches)
- `~/.patina/pandos/<name>/data/` — user data managed by the pando's children
- Any path outside the pando's own directory

Seeding creates `pando.toml` and `children/` only. If `state/` or `data/`
exist from a prior run, they are preserved across seed/overwrite/downgrade.
Test: seed v2, create state, downgrade to v1, assert state survives.

### Pando Lifecycle States

Every pando in the registry has a status:

| State | Meaning |
|---|---|
| `registered` | `pando.toml` parsed, namespace claimed. Children not yet loaded. |
| `loaded` | All children in the pando are instantiated and ready. |
| `degraded` | At least one child loaded, but one or more children unavailable. |
| `error` | Manifest or registry failure. Pando is not usable. |

The boundary rule: **`degraded` means partial child availability;
`error` means the pando itself is invalid.**

- `degraded` — the `pando.toml` is valid, the namespace is claimed, but not
  all children could be instantiated. Commands routing to a missing child fail
  individually; commands routing to a healthy child still work. Mother retries
  failed children on a backoff schedule.
- `error` — the `pando.toml` could not be parsed, the namespace collides, or
  required fields are missing. The pando is not registered in the command
  namespace. Nothing works. Mother does not retry — the manifest must be fixed.

State transitions:
- `registered` → `loaded` when Mother instantiates all children.
- `registered` → `degraded` when at least one child loads but others fail.
- `registered` → `error` on collision, parse failure, or zero children loadable.
- `loaded` → `degraded` if a child crashes at runtime.
- `degraded` → `loaded` on successful child restart.
- `error` is terminal for that run. Fix the manifest and restart Mother.

Commandless pandos (pipelines like `folder-text-to-parquet`) follow the same
states. They are `loaded` when all children are instantiated and composition
wiring is active. `patina pando list` shows them with their state. They have
no commands column — just name, status, and child count.

## Binding-Oriented Direction

Pandos are moving toward a binding-oriented resource model (Fastly/Cloudflare style)
where child declarations name required resources and Mother provides scoped bindings
at instantiate-time.

- `needs.toys` remains the intent surface during migration.
- Runtime enforcement shifts to explicit bindings (filesystem paths, sql connection
  names, http domains, event streams) rather than umbrella capability labels.
- Compatibility aliases remain during migration; new work should prefer explicit
  one-to-one resource naming.

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
