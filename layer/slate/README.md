# Slate

Slate is Patina's project-living work system for build/refactor/fix change transactions.

Durable Slate work items live with the project so collaborators and agents see the same work context alongside [[Allium]] intent and [[beliefs]]. Mother may project these files into a per-project `slate.db` for fast WIT/WASI routing, but the project artifacts remain the shareable source of truth.

Initial work-item layout:

```text
layer/slate/work/<slate-id>/work.toml
```

`patina spec` is a separate island. Explicit bridge/import/export operations may exist, but `SPEC.md` and `DESIGN.md` are not canonical Slate storage.
