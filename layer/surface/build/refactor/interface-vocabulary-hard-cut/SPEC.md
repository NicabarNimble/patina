---
type: refactor
id: interface-vocabulary-hard-cut
status: completed
created: 2026-04-04
sessions:
  origin: 20260403-084319-594949000
related:
  - src/commands/interface.rs
  - src/commands/ai/
  - src/commands/launch/
  - src/interface/
  - src/session/
  - src/workspace/
  - src/project/
  - src/main.rs
  - mother/src/state.rs
  - AGENTS.md
beliefs:
  - "[[vocabulary-drift-compounds]]"
  - "[[core-principles-contain-blast-radius]]"
exit_criteria:
  - id: ivh1-canonical-vocabulary
    text: "All active Rust identifiers and type/field names across interface/session/launch/workspace/project paths use interface* naming (`interface_name`, `interfaces`, `Interface*`)."
    checked: true

  - id: ivh2-cli-hard-cut
    text: "CLI surfaces are interface-only: `patina interface` is canonical and active clap routing has no legacy adapter command/flag aliases."
    checked: true

  - id: ivh3-config-migration
    text: "Config schema is interface-only (`interface`, `interfaces`). One-time migration upgrades old keys. If both legacy and new keys exist with conflicting values, migration fails loudly with an actionable error (user must resolve manually). Runtime serde aliases for legacy keys are removed after migration tests pass."
    checked: true

  - id: ivh4-session-and-api-terms
    text: "Session payloads, result fields, and in-memory structs use interface naming only (`interface_name`, `interface_kind`, etc.)."
    checked: true

  - id: ivh5-template-language
    text: "Interface template generation code and generated artifacts use interface terminology only in active resources/wrappers/comments."
    checked: true

  - id: ivh6-command-module-cutover
    text: "Command implementation is interface-first: `src/commands/interface/` is canonical and `src/commands/adapter.rs` has been moved to `src/commands/interface/manage.rs`."
    checked: true

  - id: ivh7-schema-cutover
    text: "Mother/session storage uses interface naming, including migration/backfill from `adapter_name` to `interface_name` where persisted."
    checked: true

  - id: ivh8-zero-active-mentions
    text: "Repository scan confirms zero occurrences of the legacy adapter term (AI interface vocabulary) in active code. Scan targets: `src/`, `mother/`, `resources/`, `AGENTS.md` with globs `*.rs *.md *.toml`. Excluded: (a) historical trees `layer/sessions/`, `layer/surface/build/`, `layer/core/`; (b) architectural adapter pattern uses per `layer/core/adapter-pattern.md` — WASM component adapters in `src/child/`, strategy-vs-adapter distinction in `src/retrieval/oracle.rs`, Mother backend adapters in `src/commands/mother/adapters.rs`. Final scan: 12 matches, all architectural."
    checked: true

  - id: ivh9-proof-tests
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass; interface launch/session/setup flows pass with interface-only naming."
    checked: true
---

# refactor: Interface Vocabulary Hard Cut

## Problem

The runtime now treats `interface` as the canonical concept, but active code still
contains legacy vocabulary across CLI, payloads, templates, config aliases, and
session storage fields. This creates drift between architecture and implementation.

## Goal

Enforce a hard cut to interface-only vocabulary in all active code paths and
runtime-facing resources.

## Where Legacy Vocabulary Still Lives

### 1) CLI / command surface

- `src/commands/adapter.rs` remains an active command module.
- `src/main.rs` still wires legacy command/flag aliases.
- `src/commands/launch/mod.rs` and `src/commands/launch/internal.rs` still use
  legacy option/state names.

### 2) AI/session payloads and fields

- `src/commands/ai/internal.rs` and `src/commands/session/internal.rs` still
  emit/use legacy keys in result payloads.
- `src/interface/internal/checkin.rs` and `src/session/internal/live.rs` still
  persist and filter by `adapter_name`.

### 3) Interface/template runtime code

- `src/interface/runtime/templates.rs` still contains legacy wording in
  wrappers/comments and flag wiring.

### 4) Workspace/project config

- `src/workspace/internal.rs` and `src/project/internal.rs` still allow legacy
  config keys through serde aliasing and legacy variable names.

### 5) Runtime storage schema

- Mother/session persistence currently uses legacy field naming
  (`mother_sessions.adapter_name` and related paths).

## Scope

In scope:
- Interface-only naming across active Rust code, CLI help, payloads, and
  generated interface resources.
- Command/module cutover to `interface` naming.
- Config migration and schema migration to interface naming.
- Final proof scan with zero active legacy-term matches.

Out of scope:
- Rewriting historical artifacts (archived specs/sessions/git history).

## Migration Strategy

1. Convert active code and schema to interface-only naming.
2. Provide one-time migration for existing configs and persisted session rows.
   - Config conflict rule: if both `adapter` and `interface` keys exist with
     different values, migration emits a clear error and aborts (no silent
     override). Matching values are deduplicated to the new key silently.
3. Remove runtime aliases and migration bridge code in the same PR/commit
   sequence that checks off ivh7 and ivh9 — not deferred to a later lane.
4. Preserve only historical references in archived artifacts.

## Release Boundary

This is a single-release hard cut. The completed lane ships migration + new
vocabulary together. There is no release where old commands work alongside new
ones — migration runs on first startup after upgrade and the old vocabulary is
gone from that point forward. Users on the new release see interface-only
surfaces.

## Execution Plan

1. Rename internal identifiers/fields/params from legacy terms to `interface*`
   across `src/interface`, `src/commands/ai`, `src/commands/session`,
   `src/commands/launch`, `src/workspace`, and `src/project`.
2. Cut over command routing to `patina interface` and remove legacy command and
   flag aliases from active clap help/routing.
3. Migrate persisted schema and runtime usage from `adapter_name` to
   `interface_name` in Mother/session state.
4. Update template generation and generated wrapper/resources to interface-only
   wording and keys.
5. Run proof suite and zero-match scan for active sources/resources.

## Proof Commands (Canonical)

- `cargo check --workspace -q`
- `cargo test -q --lib`
- `patina interface --help`
- `patina ai list --json`
- `patina ai session start --help`
- `rg -n "\badapter(s)?\b" src/ mother/ resources/ AGENTS.md --glob "*.rs" --glob "*.md" --glob "*.toml"`

Zero-match scan targets: `src/`, `mother/`, `resources/`, `AGENTS.md`.
Historical trees excluded from scan: `layer/sessions/`, `layer/surface/build/`,
`layer/core/`.
