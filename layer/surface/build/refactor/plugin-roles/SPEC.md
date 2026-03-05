---
type: refactor
id: plugin-roles
status: active
created: 2026-03-04
sessions:
  origin: 20260304-120702
beliefs:
- wit-is-contract-wasm-is-one-runtime
- patina-is-knowledge-protocol
- code-is-not-core
exit_criteria:
- id: role-field-in-manifest
  text: plugin.toml has a `role` field — one of connector, grammar, extension, app
  checked: false
- id: role-validated-at-load
  text: host validates role against world — connectors must be mother-child, grammars must be pipeline, etc.
  checked: false
- id: role-queryable
  text: '`patina plugin list` shows role for each installed plugin'
  checked: false
---
# refactor: Plugin Roles — Connector, Grammar, Extension, App Metadata

> Formalize plugin roles in manifests. WIT defines the capability
> contract. The role declares the purpose. Connectors, grammars,
> extensions, apps — same WASM, different intent.

## Problem

The plugin system has 4 worlds (mother-child, command, task, pipeline)
that define **capability boundaries**. But there's no vocabulary for what
plugins **do**. A mother-child plugin could be a connector (fetches
external data), a model resolver, or a scheduler. The host has no way
to know.

This matters for:
- Dispatch: "run all connectors" vs "run all grammars" requires role
- Discovery: "what connectors does this project have?" requires role
- Documentation: users need to understand what a plugin does, not just
  what world it runs in

**Code references:**
- `src/plugin/internal/mod.rs` lines 123-200 — `PluginManifest` struct
  and `PluginProvides` fields. Currently has `child`, `commands`,
  `pipeline_ops`, `languages` — but no `role` field.
- `src/plugin/internal/mod.rs` lines 53-73 — `PluginWorld::allowed_capabilities()`
  capability gating by world. Role would add a second validation axis.

**Architecture context:**
- [[session-20260303-190855]] — identified 5 plugin roles: connector,
  grammar, normalizer, extension, subsystem
- [[session-20260304-120702]] — refined to: connector, grammar,
  extension, app. Normalizer merged into connector (a normalizer IS a
  connector from lake to project). Subsystem renamed to app (they're
  full action layers).

## Current State

Plugin manifests declare:
- `world` — capability boundary (mother-child, command, task, pipeline)
- `capabilities` — what host functions it needs (http, query, log, etc.)
- `provides` — what it offers (child name, commands, languages, etc.)

Missing: **role** — what the plugin's purpose is.

## Target State

Manifests include a `role` field:

```toml
[plugin]
name = "forge-connector"
version = "0.1.0"
world = "mother-child"
role = "connector"        # NEW
```

| Role | Purpose | Typical World | Examples |
|------|---------|---------------|----------|
| `connector` | Fetch from external source, emit facts | mother-child | forge, email, salesforce |
| `grammar` | Parse local files into structured facts | pipeline | rust, python, markdown, pdf |
| `extension` | Add commands, analysis, monitoring | command or task | doctor, models, report |
| `app` | Full action layer, may run standalone | mother-child or task | chat-agent, crm, game-ai |

Role-world validation:
- `connector` → must be `mother-child` (needs http, credentials, emit)
- `grammar` → must be `pipeline` (pure compute, no side effects)
- `extension` → `command` or `task` (CLI surface)
- `app` → `mother-child` or `task` (needs full capabilities)

## Steps

1. Add `role` field to `PluginManifest` in `src/plugin/internal/mod.rs`
2. Add role-world validation in `check_capabilities()`
3. Update `patina plugin list` to show role column
4. Update existing plugins (grammar-rust, grammar-forge, models, doctor)
   to declare their role
5. Update SDK documentation with role descriptions

## Design Decisions (resolved in [[spec-plugin-infrastructure]] DESIGN.md)

- **Role-based dispatch is consumer-defined.** This spec adds the
  vocabulary (connector, grammar, extension, app). How roles are
  consumed is defined by the consumers: [[spec-scrape-simplification]]
  dispatches grammar-role plugins for code indexing,
  [[spec-continuous-operation]] schedules connector-role plugins via
  Mother's daemon. Role metadata enables dispatch; this spec doesn't
  prescribe dispatch strategy.

- **App is a plugin role.** Edge apps (Cloudflare Workers) that
  communicate via HTTP/WebSocket are a future concern per
  [[local-first-edge-deployable]]. For now, app is a local plugin role
  in the mother-child or task world. The role is "what you're for," not
  "where you run."

## Non-Goals

- **Building new plugins.** This spec adds role metadata. Actual
  connectors/grammars are separate specs.
- **Role-based auto-dispatch.** This spec adds the metadata. Dispatch
  logic is [[scrape-simplification]] and [[continuous-operation]].
- **Subsystem extraction roles.** Spec and session subsystem extraction
  is [[core-plugin-extraction]]'s concern. Roles for those emerge when
  the WIT interfaces exist.
