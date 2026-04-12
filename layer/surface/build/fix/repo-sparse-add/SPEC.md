---
type: fix
id: repo-sparse-add
status: active
created: 2026-04-11
related:
- src/commands/repo/mod.rs
- src/commands/repo/internal.rs
- sdk/ba/repos.toml
beliefs:
- '[[unix-philosophy]]'
- '[[oxidized-knowledge]]'
- '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
- id: rsa1-cli-sparse-flag
  text: "`patina repo add <url> --sparse <path>` is supported (single or repeated flag), with help text using sparse terminology (no `--subdir` alias)."
  checked: true
- id: rsa2-sparse-clone-behavior
  text: "Repo add uses sparse checkout + partial clone (`--filter=blob:none`) so working tree contains only requested sparse path(s), while git history remains upstream-valid."
  checked: true
- id: rsa3-registry-shape
  text: "Registry stores sparse configuration in RepoEntry and preserves it across `repo update` / `repo show` / `repo list` surfaces."
  checked: true
- id: rsa4-separate-storage-lane
  text: "Sparse entries are stored in deterministic, separate cache paths from full clones (same upstream URL can exist as full + sparse variants without collision)."
  checked: true
- id: rsa5-fail-closed-validation
  text: "Sparse paths fail closed on empty, absolute, traversal (`..`), or `.git`-targeting values with explicit error messages."
  checked: true
- id: rsa6-tests-proof
  text: "Deterministic tests cover CLI parsing, registry roundtrip, sparse path validation, and clone command construction; proof commands are documented."
  checked: true
---
# fix: Add sparse repo intake for reference knowledge

## Problem

`patina repo add` only supports full repository clones. For upstreams where a single path is authoritative and actively maintained (e.g. `WebAssembly/component-model/design/mvp`), full-clone intake introduces avoidable noise and storage cost.

## Goal

Add a **generic sparse intake mode** for external repos:

- primary flag name: `--sparse`
- supports one or more sparse paths
- stores sparse variants separately from full clones
- keeps normal git update flow

## Scope

- `repo add`, `repo list`, `repo show`, `repo update`
- registry entry model for sparse metadata
- clone/update shelling in repo internal implementation

## Non-goals

- Path-rewritten history mirrors (`git filter-repo`) inside Patina
- New package manager features
- Mother control-plane API changes in this patch

## Notes

Sparse checkout is a working-tree selection mechanism, not path-rewritten history.
This patch intentionally keeps upstream-correct commit history while reducing working tree + blob transfer.

## Verification

```bash
cargo test -q repo
patina repo add https://github.com/WebAssembly/component-model --sparse design/mvp --no-oxidize
patina repo show WebAssembly/component-model::design-mvp
patina repo update WebAssembly/component-model::design-mvp
```
