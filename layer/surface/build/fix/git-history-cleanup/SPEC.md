---
type: fix
id: git-history-cleanup
status: active
created: 2026-02-25
target: '1'
sessions:
  origin: 20260225-143514
beliefs:
- git-is-the-knowledge-substrate
exit_criteria:
- id: git-directory-under-200-mb-after-cleanup
  text: .git directory under 200 MB after cleanup
  checked: false
- id: all-existing-tags-preserved-and-functional
  text: All 1,825 tags preserved and functional (1,796 annotated + 29 lightweight)
  checked: false
- id: patina-spec-history-works-for-all-archived-specs
  text: patina spec history works for all archived specs
  checked: false
- id: no-source-code-layer-files-or-config-lost
  text: No source code, layer files, or config lost
  checked: false
- id: cargo-build-release-succeeds
  text: cargo build --release succeeds after rewrite
  checked: false
- id: cargo-test-passes
  text: cargo test passes after rewrite
  checked: false
---
# fix: Strip historical binary blobs from git history

> `.git` is 733 MB (as of 2026-03-03). The actual project history (1,825 tags,
> session/belief/pattern files) is small. The bloat is 399 dead binary blobs
> over 1 MB (1.5 GB raw) — ONNX models, duckdb libraries, grammar build
> artifacts, cargo registry cache, and old database snapshots.

## Problem

`.git/objects` is 709 MB in packfiles (4 packs, largest 406 MB).
399 blobs over 1 MB total 1.5 GB raw (compressed in packs). Top blobs:

| File | Size | Status |
|---|---|---|
| `resources/models/all-minilm-l6-v2/model.onnx` | 90 MB | Historical — ONNX models now downloaded at runtime |
| `libduckdb/libduckdb_static.a` | 72 MB | Historical — duckdb dependency removed |
| `libduckdb/libduckdb.so` | 57 MB | Historical — duckdb dependency removed |
| `grammar-cairo/target/debug/deps/...` | 33 MB | 254 blobs > 1 MB from this subtree alone |
| `libduckdb-linux-amd64.zip` | 33 MB | Historical — duckdb dependency removed |
| `.patina/knowledge.db` | 26 MB | 58 blobs > 1 MB from `.patina/` DB snapshots |
| `resources/models/all-minilm-l6-v2/model_quantized.onnx` | 23 MB | Historical |
| `grammar-cairo/target/` (multiple) | 10-23 MB each | Build artifacts, 254 large blobs total |
| `grammar-go/target/` (multiple) | 10-18 MB each | Build artifacts, 51 large blobs total |
| `.cargo/registry/cache/` | 1-5 MB each | 7 blobs — cargo registry accidentally committed |
| `patina-metal/grammars/c/src/` | ~18 MB | Dead experiment |

By category (blobs > 1 MB):

| Category | Count | Notes |
|---|---|---|
| `grammar-cairo/target/` | 254 | Biggest offender |
| `.patina/` DB snapshots | 58 | knowledge.db versions |
| `grammar-go/target/` | 51 | Build artifacts |
| `grammars/*/src/parser.c` | ~10 | Large generated C — **currently tracked, do NOT strip** |
| `.cargo/registry/cache/` | 7 | Leaked cargo cache |
| `libduckdb*` | 4 | Dead dependency |
| `resources/models/*.onnx` | 2 | Models now gitignored |
| `patina-metal/` | 1 | Dead experiment |

These files were committed at some point, later deleted or gitignored, but their
blobs persist forever in packfiles. Git doesn't forget.

## Root Cause

Early development committed binary dependencies and build artifacts before
`.gitignore` was properly configured. The files were later removed from the
working tree but remain in git history.

## Fix

Use `git filter-repo` to strip specific blob patterns from all history.

### Pre-flight

1. **Full backup**: `cp -r .git .git-backup` (or clone to separate directory)
2. **Verify tag integrity**: `patina spec history` for all archived specs — record output
3. **Record tag count**: `git tag -l | wc -l` — expected: 1,825 (1,796 annotated + 29 lightweight)
4. **List all remotes**: `git remote -v`
5. **Inventory**: confirm blob list with `git rev-list --objects --all | git cat-file --batch-check`
6. **Inventory lightweight tags**: `git for-each-ref refs/tags --format='%(objecttype) %(refname:short)' | grep ^commit` — record these 29 tags

### Execution

```bash
# Install if needed
brew install git-filter-repo

# Strip by path pattern (destructive, rewrites history)
git filter-repo \
  --path-glob 'libduckdb*' --invert-paths \
  --path-glob '*.onnx' --invert-paths \
  --path-glob 'grammar-cairo/target/*' --invert-paths \
  --path-glob 'grammar-go/target/*' --invert-paths \
  --path-glob 'patina-metal/*' --invert-paths \
  --path-glob '.patina/knowledge.db' --invert-paths \
  --path-glob '.patina/patina.db' --invert-paths \
  --path-glob '.patina/events.db' --invert-paths \
  --path-glob '.cargo/registry/*' --invert-paths \
  --path-glob '*.a' --invert-paths \
  --path-glob '*.so' --invert-paths \
  --path-glob '*.dylib' --invert-paths \
  --path-glob '*.rlib' --invert-paths \
  --path-glob '*.rmeta' --invert-paths
```

### Post-flight

1. **Verify size**: `du -sh .git` — target: under 200 MB
2. **Verify tags**: `git tag -l | wc -l` — must match pre-flight count
3. **Verify lightweight tags**: compare against pre-flight inventory
4. **Verify spec history**: `patina spec history` for archived specs — compare against pre-flight output
5. **Verify build**: `cargo build --release`
6. **Verify tests**: `cargo test`
7. **Verify layer**: all session/belief/pattern files intact
8. **Verify WASM fixtures**: `ls -la tests/fixtures/*.wasm` — all 6 present
9. **Re-add remote**: `git remote add origin <url>`
10. **Force push**: `git push --force --all && git push --force --tags`

### Risks

- **Force push required** — rewrites all commit SHAs. Anyone with a local clone
  must re-clone. This is a solo project so impact is minimal.
- **Tag SHAs change** — 1,796 annotated tags survive filter-repo (rewritten to
  new commits). 29 lightweight tags also rewritten. `patina spec history` uses
  tag names not SHAs, so it should work. Verify both types.
- **CI/tooling references** — any external system referencing commit SHAs will break.
  Check GitHub PRs, issues referencing commits.

### Irreversibility

This is a one-way operation. The `.git-backup` is the only recovery path.
After force push, the old history is gone from the remote.

## What NOT to Strip

- `grammars/` directory — 119 files tracked (Cargo.toml, build.rs, grammar.json,
  parser.c). The `parser.c` files are large (up to 18 MB) but are generated
  tree-sitter source needed at build time. No filter-repo globs match `.c` files.
- `tests/fixtures/*.wasm` — 6 WASM plugin fixtures (total ~964 KB) needed for
  `cargo test`. No filter-repo globs match `.wasm` files. Verified safe.
- `layer/` files — these ARE the project. Never strip.
- `resources/` — only `README.md`, `registry.toml`, bench data, and claude
  skill templates are tracked (68 files). ONNX models are gitignored. Safe.

## Exit Criteria

- [ ] `.git` directory under 200 MB after cleanup
- [ ] All 1,825 tags preserved and functional (1,796 annotated + 29 lightweight)
- [ ] `patina spec history` works for all archived specs
- [ ] No source code, layer files, or config lost
- [ ] `cargo build --release` succeeds after rewrite
- [ ] `cargo test` passes after rewrite

## Audit Notes (2026-03-03)

Audited in session 20260303-153301. Key findings:
- `.git` grew from 714 MB → 733 MB since spec was written
- 399 blobs > 1 MB totaling 1.5 GB raw — much worse than original inventory
- `grammar-cairo/target/` alone has 254 large blobs (spec showed 1 example)
- Added `.cargo/registry/*`, `patina-metal/*`, `.patina/*.db` to filter patterns
- 6 WASM test fixtures and `grammars/*/parser.c` confirmed safe from globs
- Tag breakdown: 1,796 annotated (survive rewrite) + 29 lightweight (also rewritten)
- Tightened size target from 400 MB → 200 MB based on estimated reclaim
- Execution should be a separate focused session
