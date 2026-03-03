---
type: fix
id: git-history-cleanup
status: draft
created: 2026-02-25
target: '1'
sessions:
  origin: 20260225-143514
beliefs:
- git-is-the-knowledge-substrate
exit_criteria:
- id: git-directory-under-400-mb-after-cleanup
  text: .git directory under 400 MB after cleanup
  checked: false
- id: all-existing-tags-preserved-and-functional
  text: All existing tags preserved and functional
  checked: false
- id: patina-spec-history-works-for-all-archived-specs
  text: patina spec history works for all archived specs
  checked: false
- id: no-source-code-layer-files-or-config-lost
  text: No source code, layer files, or config lost
  checked: false
---
# fix: Strip historical binary blobs from git history

> `.git` is 714 MB. The actual project history (2,835 commits, 1,665 tags,
> 967 layer files) is small. The bloat is dead binary blobs that were committed
> and later removed — ONNX models, duckdb libraries, grammar build artifacts.

## Problem

`.git/objects` is 712 MB across 53K packed objects. The largest blobs in history:

| File | Size | Status |
|---|---|---|
| `resources/models/all-minilm-l6-v2/model.onnx` | 90 MB | Historical — ONNX models now downloaded at runtime |
| `libduckdb/libduckdb_static.a` | 72 MB | Historical — duckdb dependency removed |
| `libduckdb/libduckdb.so` | 57 MB | Historical — duckdb dependency removed |
| `grammar-cairo/target/debug/deps/...` | 33 MB | Build artifacts that should never have been committed |
| `libduckdb-linux-amd64.zip` | 33 MB | Historical — duckdb dependency removed |
| `.patina/knowledge.db` | 26 MB | Historical — DB files now gitignored |
| `resources/models/all-minilm-l6-v2/model_quantized.onnx` | 23 MB | Historical |
| `grammar-cairo/target/...` (multiple) | 10-23 MB each | Build artifacts |

These files were committed at some point, later deleted or gitignored, but their
blobs persist forever in packfiles. Git doesn't forget.

Meanwhile, the actual knowledge layer this project cares about:
- 757 session files: 3.4 MB total
- 160 belief files: ~200 KB total
- 2,835 commits of source code diffs: small

## Root Cause

Early development committed binary dependencies and build artifacts before
`.gitignore` was properly configured. The files were later removed from the
working tree but remain in git history.

## Fix

Use `git filter-repo` to strip specific blob patterns from all history.

### Pre-flight

1. **Full backup**: `cp -r .git .git-backup` (or clone to separate directory)
2. **Verify tag integrity**: `patina spec history` for all archived specs — record output
3. **List all remotes**: `git remote -v`
4. **Inventory**: confirm blob list with `git rev-list --objects --all | git cat-file --batch-check`

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
  --path-glob '.patina/knowledge.db' --invert-paths \
  --path-glob '*.a' --invert-paths \
  --path-glob '*.so' --invert-paths \
  --path-glob '*.dylib' --invert-paths \
  --path-glob '*.rlib' --invert-paths \
  --path-glob '*.rmeta' --invert-paths
```

### Post-flight

1. **Verify size**: `du -sh .git` — target: under 400 MB
2. **Verify tags**: `git tag -l | wc -l` — should still be 1,665+
3. **Verify spec history**: `patina spec history` for archived specs
4. **Verify build**: `cargo build --release`
5. **Verify layer**: all session/belief/pattern files intact
6. **Re-add remote**: `git remote add origin <url>`
7. **Force push**: `git push --force --all && git push --force --tags`

### Risks

- **Force push required** — rewrites all commit SHAs. Anyone with a local clone
  must re-clone. This is a solo project so impact is minimal.
- **Tag SHAs change** — annotated tags survive filter-repo (they point to new
  rewritten commits) but their commit SHAs change. `patina spec history` uses
  tag names not SHAs, so it should work. Verify.
- **CI/tooling references** — any external system referencing commit SHAs will break.
  Check GitHub PRs, issues referencing commits.

### Irreversibility

This is a one-way operation. The `.git-backup` is the only recovery path.
After force push, the old history is gone from the remote.

## What NOT to Strip

- `grammars/` directory (4.4 GB on disk) — these are compiled tree-sitter
  grammars, currently in the working tree and needed. If they're gitignored,
  they're not in `.git`. If they are tracked, evaluate separately.
- `layer/` files — these ARE the project. Never strip.
- `resources/` (553 MB on disk) — check what's tracked vs gitignored before
  including in filter.

## Exit Criteria

- [ ] `.git` directory under 400 MB after cleanup
- [ ] All existing tags preserved and functional
- [ ] `patina spec history` works for all archived specs
- [ ] No source code, layer files, or config lost
