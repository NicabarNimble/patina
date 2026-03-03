# Git Hooks for Patina

Git hooks keep Patina's knowledge base current after each commit or merge.
The hooks fork `patina scrape` to the background — `git commit` is never blocked.

## Install (Patina's own repo)

```bash
ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit
ln -sf ../../resources/git/post-merge.sh .git/hooks/post-merge
```

Verify:
```bash
ls -la .git/hooks/post-*
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
