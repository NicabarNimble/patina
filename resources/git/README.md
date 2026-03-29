# Git Hooks for Patina

Git hooks keep Patina's knowledge base current after each commit or merge.
The hooks fork `patina scrape` to the background — `git commit` is never blocked.

## Install (Patina's own repo)

```bash
ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit
ln -sf ../../resources/git/post-merge.sh .git/hooks/post-merge
```

The pre-commit and pre-push hooks use exec-style delegation (see `.git/hooks/pre-commit` and `.git/hooks/pre-push`).

## Test gate tiers

- Tier 0 (`resources/git/pre-commit-checks.sh`): `cargo fmt --check` + staged large-file guard (<5s target).
- Tier 1 (`resources/git/pre-push-checks.sh --structural-only`): structural policy checks only, no cargo (<30s target).
- Tier 2 (`resources/git/pre-push-targeted-cargo.sh`): changed-package clippy/tests plus path-triggered parity/schema.
- Tier 3 (`resources/git/preflight-full.sh`): full local suite equivalent to merge-gate semantics.

`resources/git/pre-push-checks.sh` runs Tier 2 by default; use `--structural-only` (or `PATINA_PRE_PUSH_RUN_TARGETED=0`) to run Tier 1 only.

Verify:
```bash
ls -la .git/hooks/pre-commit .git/hooks/pre-push .git/hooks/post-*
```

## Install (other projects)

```bash
cat > .git/hooks/post-commit << 'EOF'
#!/bin/sh
command -v patina >/dev/null 2>&1 || exit 0
patina hook post-commit
EOF
chmod +x .git/hooks/post-commit
```

Repeat for `post-merge` (replace `post-commit` with `post-merge`).

## How it works

1. Git invokes `.git/hooks/post-commit` after each commit
2. The shell shim checks if `patina` is on PATH (exits silently if not)
3. `patina hook post-commit` forks `patina scrape` to background
4. Output goes to `.patina/local/hook.log`
5. Git commit returns immediately

## Debugging

```bash
# Check hook log
cat .patina/local/hook.log

# Check if hook events are being recorded
patina measure --full
```

## Optional Smell Scan

Use the senior-smell scan when you want feedback on architecture/performance
issues that clippy usually misses:

```bash
bash resources/git/senior-smell-checks.sh
```

This is report-only by default. To make it fail on findings:

```bash
bash resources/git/senior-smell-checks.sh --strict
```

Current checks look for:
- runtime regex compilation in scraper hot paths
- whole-file reads in ingestion/listing paths
- `Vec<Vec<f32>>` in numeric code
- UTF-8-unsafe byte slicing in truncation helpers
- avoidable cloning/materialization in benchmark/eval paths
- repeated lowercasing in search/routing code
