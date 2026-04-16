---
type: fix
id: js-mjs-grammar-claim-alignment
status: active
created: 2026-04-15
beliefs:
  - "[[spec-driven-design]]"
  - "[[dependable-rust]]"
  - "[[unix-philosophy]]"
related:
  - grammars/javascript/child.toml
  - src/commands/scrape/code/languages/mod.rs
  - src/commands/setup/grammars.rs
  - tests/grammar_manifest_conformance.rs
exit_criteria:
  - id: jmgca1-javascript-plugin-claims-mjs
    text: "`grammar-javascript` manifest claims `mjs` in addition to `js`/`jsx`, so JS module files route to the JavaScript parser."
    checked: true
  - id: jmgca2-language-detection-locked
    text: "Language-extension mapping includes deterministic coverage for `.mjs` -> JavaScript and unknown extension fail-closed behavior."
    checked: true
  - id: jmgca3-manifest-conformance-lock
    text: "A conformance test locks JavaScript grammar manifest language claims (`js`, `jsx`, `mjs`)."
    checked: true
  - id: jmgca4-oxidize-regression-cleared
    text: "`patina repo update juxt/allium --oxidize` succeeds without `No pipeline plugin for JavaScript` parse skips or dependency zero-edge failure."
    checked: true
---

# fix: js/mjs grammar claim alignment

## Problem

External repos containing `.mjs` JavaScript modules can fail scrape/oxidize despite JavaScript grammar being installed, because language detection recognizes `.mjs` as JavaScript but the JavaScript grammar plugin manifest does not claim `mjs`.

This produces parse skips (`No pipeline plugin for JavaScript`) and can cascade into dependency projection failure (`No functions with call relationships found`).

## Goal

Align extension claims and lock regression tests so `.mjs` JavaScript files are parsed by the JavaScript grammar plugin in default setup flows.

## Scope

- Add `mjs` to `grammars/javascript/child.toml` language claims.
- Add deterministic extension mapping tests.
- Add manifest conformance lock test.
- Verify oxidize success on the failing external repo path.

## Non-goals

- No new command flags.
- No parser/runtime contract redesign.
- No broad scrape architecture changes.

## Verification commands

```bash
cargo test -p patina-ai commands::scrape::code::languages::tests:: -- --nocapture
cargo test -p patina-ai --test grammar_manifest_conformance -- --nocapture
patina setup grammars --force
patina repo update juxt/allium --oxidize
```
