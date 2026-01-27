# Layer Cleanup

> Remove obsolete layer directories that shouldn't be public.

**Status:** pending
**Created:** 2026-01-26

---

## Problem

Post-public-launch cleanup. Some layer directories were tracked before gitignore was set up, or contain obsolete content that shouldn't be in the public repo.

## Directories to Remove

### layer/lab/

```
layer/lab/queries.json
```

Old lab automation queries. No longer used.

**Action:** `git rm -r layer/lab/`

### layer/personas/

```
layer/personas/persona-20251029-061054.md
layer/personas/persona-20251106-214532.md
```

Old persona experiment files from Oct/Nov 2025. Superseded by session-based knowledge capture.

**Action:** `git rm -r layer/personas/`

## Already Fixed

- [x] `layer/dust/architecture/` - untracked (was gitignored but tracked)

## Exit Criteria

- [ ] `layer/lab/` removed from git
- [ ] `layer/personas/` removed from git
- [ ] Verify with `git ls-files | grep -E "(lab|personas)"`
