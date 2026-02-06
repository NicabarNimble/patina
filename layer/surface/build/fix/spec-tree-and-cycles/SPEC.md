---
type: fix
id: spec-tree-and-cycles
status: ready
created: 2026-02-06
sessions:
  origin: 20260206-060219
blocked_by: []
blocks: []
related:
  - layer/surface/build/feat/spec-as-work-item/SPEC.md
beliefs:
  - stale-context-is-hostile-context
  - process-checkpoints-catch-what-tooling-misses
---

# fix: Spec Tree and Cycle Detection

> Without cycle detection, a circular dependency silently deadlocks the ready queue.
> Without spec tree, you can't debug "why is this blocked?"

## Problem

`patina spec ready` and `patina spec blocked` shipped in [[spec-as-work-item]] but the
dependency graph has no visibility or safety:

1. **No tree view** — `patina spec blocked` shows "blocked by: X" but not the full chain.
   If A blocks B blocks C, you see B is blocked by A, but C just shows blocked by B. You
   can't see the root cause without manually tracing.

2. **No cycle detection** — If spec A lists `blocked_by: [B]` and spec B lists
   `blocked_by: [A]`, both specs vanish from `patina spec ready`. Neither is actionable.
   No warning. Silent deadlock.

These were v0.13.0 items in spec-as-work-item. Rehomed here so that spec can close at
100% exit criteria.

---

## Design

### `patina spec tree <id>`

Show the full dependency graph for a spec:

```bash
$ patina spec tree cli-reorganization

cli-reorganization (ready, blocked)
├── blocked by: system-introspection (draft)
│   └── (no blockers)
└── blocked by: scrape-layer-unify (complete ✓)
```

Implementation: recursive query on `spec_deps` table. Walk `blocked_by` edges, print
indented tree. Detect and mark cycles during traversal.

### Cycle Detection in `patina doctor`

Add a "Spec Health" check:

```
Spec Dependencies:
  ✅ 12 specs with clean dependency chains
  ❌ Cycle detected: A → B → C → A
```

Implementation: Tarjan's algorithm or DFS with visited set on the `spec_deps` graph.
Surface in doctor output with critical severity.

Also surface in `patina spec ready`:

```
WARNING: Circular dependency detected:
  A → B → C → A
These specs can never become ready.
```

---

## Exit Criteria

- [ ] `patina spec tree <id>` shows full dependency chain with indentation
- [ ] Tree view marks completed blockers with ✓
- [ ] Cycle detection runs during `patina doctor`
- [ ] Cycles surfaced as warnings in `patina spec ready` output
- [ ] At least 1 test with circular dependency confirms detection works

---

## Files to Change

```
src/commands/spec/
├── mod.rs           # Add tree subcommand
├── internal.rs      # Add tree traversal + cycle detection queries
└── (existing)

src/commands/doctor.rs   # Add spec cycle check
```
