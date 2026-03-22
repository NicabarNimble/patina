---
type: refactor
id: vocabulary-alignment-child-manifest
status: ready
created: 2026-03-22
sessions:
  origin: 20260321-162736-004031000
related:
- layer/surface/build/refactor/patina-pre-v1/SPEC.md
- src/plugin/internal/mod.rs
- src/plugin/internal/tests.rs
- src/plugin/
- children/*/child.toml
- children/template/child.toml
- AGENTS.md
- layer/surface/epistemic/beliefs/core-primitives-are-not-children.md
- layer/surface/epistemic/beliefs/core-baseline-child-strategy-extensions.md
exit_criteria:
- id: VC1
  text: Child-first naming exists in code (`ChildManifest`, `ChildKind`, `ChildEngine`) with compatibility aliases for legacy plugin names
  checked: true
- id: VC2
  text: Manifest schema uses `kind` as canonical field; legacy `world` remains read-compatible during migration with explicit deprecation warning
  checked: true
- id: VC3
  text: File naming supports `child.toml` as canonical while continuing to read `plugin.toml` during bridge period
  checked: true
- id: VC4
  text: Specs/docs/agent guidance consistently use child/toy/kind vocabulary and explicitly reserve WIT `world` for WIT semantics
  checked: true
- id: VC5
  text: Regression checks prove no behavior change in child loading/linking/runtime grants during rename bridge
  checked: true
---
# refactor: refactor: Vocabulary alignment for child/toy architecture

> Rename overloaded plugin/world terminology to child/kind terminology with compatibility bridge and deprecation plan.

## Problem

Patina's current vocabulary drifts between legacy plugin terms and child/toy architecture terms. `world` is overloaded across WIT semantics, runtime category markers, and manifest fields. This causes repeated architectural misreads by agents and contributors, especially around whether core protocol verbs are child-provided or Mother/core-provided.

## Goal

Align code, manifest, and docs vocabulary to the current architecture: children + toys + kinds. Keep behavior stable while introducing a compatibility bridge that avoids breaking existing manifests and workflows.

## Status

Draft. `patina-pre-v1` is blocked on this terminology alignment to prevent further architectural drift during remaining EC closure work.

## Non-Goals

- No behavior rewrite of child execution model.
- No daemon/protocol redesign.
- No removal of compatibility paths in this pass (removal follows in a later cleanup spec once migration is complete).

## Current State

- Runtime/core types still use `Plugin*` naming (`PluginManifest`, `PluginWorld`, `PluginEngine`).
- Child manifests use `plugin.toml` and `world = "knowledge-child"`, which is often confused with WIT world composition in Cargo metadata.
- Docs/specs increasingly use child/toy language, but code-level names and manifest keys still pull contributors toward legacy mental models.

## Target State

- Child-first naming is canonical in code and docs.
- Manifest runtime category field is `kind`, not `world`.
- Child manifest filename is canonicalized to `child.toml` (with bridge compatibility for `plugin.toml`).
- WIT `world` remains a WIT-specific concept only.

### Canonical Mapping Lock

- `PluginManifest` -> `ChildManifest`
- `PluginWorld` -> `ChildKind`
- `PluginRole` -> `ChildRole`
- `KnowledgeChildEngine`/`PluginEngine` naming surface -> `ChildEngine` naming surface
- `plugin.toml` -> `child.toml` (canonical)
- manifest `world = "..."` -> `kind = "..."` (canonical)

Allowed runtime kind values in this spec:

- `knowledge-child`
- `mother-child`
- `task`
- `pipeline`

Legacy values/fields must map 1:1 to the same runtime behavior.

## Solution

Do a bounded two-track migration:

1. Introduce canonical child vocabulary and compatibility aliases.
2. Switch manifest/documentation surfaces to child/kind naming while preserving read compatibility for existing files/fields.

This keeps runtime behavior unchanged while fixing architectural language at the source.

### Bridge Policy (Locked)

- **Read path:** support both legacy and canonical forms.
  - field keys: read `kind` first, fallback to `world`
  - manifest filenames: read `child.toml` first, fallback to `plugin.toml`
- **Write/scaffold path:** emit canonical forms only (`kind`, `child.toml`).
- **Deprecation behavior:** when fallback is used, emit warning:
  - `deprecated manifest key 'world'; use 'kind'`
  - `deprecated manifest filename 'plugin.toml'; use 'child.toml'`

## Implementation Order

1. Add child-first type names and aliases (`ChildManifest`, `ChildKind`, `ChildEngine` + temporary `Plugin*` aliases).
2. Add `kind` manifest field parsing/writing; keep `world` fallback read path with deprecation warning.
3. Add `child.toml` lookup/write support while preserving `plugin.toml` fallback lookup.
4. Update in-repo child manifests/template and docs/spec guidance to child/kind terms.
5. Add regression checks + grep checks to enforce vocabulary boundaries.

## Resolved Decisions

- This is a terminology+contract alignment spec, not an architecture rewrite spec.
- Compatibility bridge is required to avoid churn and preserve current runtime behavior.
- WIT `world` remains valid only for WIT/component composition contexts.
- This spec includes canonical filename migration (`child.toml`) and dual-read compatibility in the same pass (not deferred).

## Verification

- `cargo test` passes for child loading/linking tests before and after rename bridge.
- Child manifest parsing tests validate both old (`world`, `plugin.toml`) and new (`kind`, `child.toml`) forms during transition.
- Grep checks confirm canonical docs/spec wording uses child/kind vocabulary and avoids ambiguous world usage outside WIT contexts.

Required grep checks (bridge-safe):

- Allowed (compatibility only): legacy `Plugin*` names may remain only in explicit `type` aliases or deprecation adapters.
- Forbidden in new code/docs introduced by this spec:
  - `PluginManifest`/`PluginWorld`/`PluginRole` in public-facing docs/spec text
  - references claiming manifest `world` is canonical
  - write paths that generate `plugin.toml`

## Exit Criteria

See frontmatter `exit_criteria` (VC1-VC5).

## Build Readiness

Ready once a commit plan is populated in `DESIGN.md` and closure work is explicitly bounded to vocabulary migration + compatibility bridge only.
