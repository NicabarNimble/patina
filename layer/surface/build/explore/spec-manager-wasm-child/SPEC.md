---
type: explore
id: spec-manager-wasm-child
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
exit_criteria:
  - Decision on SQL toy implementation approach
  - Decision on git WIT extensions needed
  - Go/no-go on converting spec-manager to WASM child
---
# explore: Convert spec-manager from builtin to WASM child

> Spec-manager is currently a builtin function dispatch inside Mother. Should be a proper WASM child per canon. Requires SQL toy (missing), extended git WIT ops, and porting spec parsing to child.

## Question

What's the real lift to convert spec-manager from a builtin dispatch
(`BuiltinChild::SpecManager` → `patina::spec::execute_command_value()`)
to a proper WASM child per child-construction-canon?

## Findings

### Current architecture

Spec-manager is a builtin — not a WASM child. CLI dispatches to Mother via
`BuiltinChild::SpecManager`, Mother calls `patina::spec::execute_command_value()`
which runs `src/commands/spec/internal/` code inside Mother's process.

A stub exists at `children/spec-manager/` but it only handles health + dispatch
passthrough. The real logic is in `src/commands/spec/internal/mutations.rs`
and `src/commands/spec/internal/archive.rs`.

### Why it matters

- Violates canon: "Children — WASM components that do compute"
- Mother's CWD affects spec operations (relative path bug found in session 20260402)
- Builtin children can't be updated independently
- No sandbox isolation for spec file mutations

### Three gaps identified (session 20260402)

**1. SQL toy — big gap, zero host implementation**
- WIT interface exists: `wasi:sql/readwrite@0.1.0`
- No `mother/src/toys/sql.rs` — not implemented
- Spec-manager queries `patterns` + `spec_deps` tables (~8 query patterns)
- This is shared infrastructure — lake-manager and session-writer would benefit

**2. Extended git ops — medium gap**
- Current git WIT: tag, commit, log-oneline, diff-stat
- Missing for spec lifecycle: `for-each-ref` (tag query), `show` (archived
  content), `rm -rf` (directory removal during archive)
- Needs WIT additions + host methods in `mother/src/toys/git.rs`

**3. Spec parsing in child — small gap**
- Frontmatter parsing, validation, state machine — all pure computation
- Port to child crate or add to SDK
- No host dependency

### Business logic is portable

The spec state machine (draft→ready→active→complete/abandoned), frontmatter
parsing, validation, blocker resolution — all pure Rust with no I/O. Trivially
runs in WASM. The entire difficulty is the I/O surface (SQL + git + filesystem).

### Estimated lift

- Session 1: SQL toy host implementation + git WIT extensions
- Session 2: Port spec logic to WASM child, wire up toys
- Session 3: Remove builtin dispatch, test parity

The patterns exist from 14 existing children. Mechanical, not creative.

## Conclusions

Deferred until after other specs complete. The SQL toy is the real prerequisite
and would unlock other children too. Revisit when:
- child-construction-canon ccc3 (reuse proven) is checked
- Or when the CWD-relative-path bug becomes painful enough to force the move

### Also discovered in this session

- Mother version mismatch: CLI v0.45.7 was talking to Mother v0.43.17 (236
  commits behind). Need a version handshake on connect.
- "spec-manager: missing" status message is misleading — it's a builtin,
  always present. The status display treats it like a WASM child need.
