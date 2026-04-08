---
type: fix
id: sdk-knowledge-child-shim-removal
status: complete
created: 2026-04-08
related:
  - sdk/patina-sdk/src/lib.rs
  - sdk/patina-sdk/src/child.rs
  - children/
exit_criteria:
  - id: kcsr1
    text: "All children import from `patina_sdk::child` and implement `Child` trait — no references to `knowledge_child` or `KnowledgeChild` remain in children/"
    checked: true
  - id: kcsr2
    text: "Deprecated shims removed from `sdk/patina-sdk/src/lib.rs` — `knowledge_child` re-export, `KnowledgeChild` alias, and `KnowledgeChildPlugin` alias are deleted"
    checked: true
  - id: kcsr3
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass with zero deprecation warnings from the removed shims"
    checked: true
---
# fix: Remove SDK knowledge_child migration shims

## Problem

The child-rename migration (v0.46.0) added deprecated re-exports so existing
children would keep compiling. The shims were marked "remove in v0.47.0." We are
now at v0.49.0 and all 13 children still import the deprecated path. The shims
mask incomplete migration and block clean typed-interface work.

## Scope

1. Update all `children/*/src/lib.rs` to use canonical imports.
2. Remove the three `MIGRATION-SHIM` lines from `sdk/patina-sdk/src/lib.rs`.
3. Verify clean build and tests.

## Non-Goals

- No trait signature changes.
- No child behavior changes.
- No manifest or WIT changes.
