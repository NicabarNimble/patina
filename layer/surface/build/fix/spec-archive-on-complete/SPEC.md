---
type: fix
id: spec-archive-on-complete
status: complete
created: 2026-02-02
sessions:
  origin: 20260202-063713
related:
  - layer/surface/build/feat/belief-verification/SPEC.md
  - layer/surface/epistemic/beliefs/archive-completed-work.md
---

# fix: Archive Specs on Completion

**Problem:** Completed specs with `status: complete` linger in `layer/surface/build/`. The
`archive-completed-work` belief correctly detects this (4 specs, contested). The established
archival pattern exists in git history (46 `spec/*` tags) but is entirely manual. The version
system (`patina version milestone`) marks specs complete but doesn't archive them.

**Root cause:** Missing automation step between "spec marked complete" and "spec archived via
git tag + file removal."

**Fix:** Add `patina spec archive <id>` command that follows the established three-step pattern:
1. Create `spec/<id>` git tag (preserves content)
2. Remove spec file from working tree
3. Update `build.md` Archives section
4. Commit with message: `docs: archive spec/<id> (complete)`

Additionally, `patina version milestone` should hint when a spec is fully done:
"Spec fully complete. Archive with: `patina spec archive <id>`"

**Design rationale (Jon Gjengset):** Separate command, not embedded in `milestone`. Destructive
actions (file deletion) should be explicit verbs, not side effects. Composable: can archive old
specs, run in bulk, test independently.

---

## Build Steps

- [x] 1. Add `spec` command group to CLI with `archive` subcommand
- [x] 2. Implement `spec archive <id>`: find spec file by id in patterns table, validate
  `status: complete`, create `spec/<id>` git tag, remove file + directory, update build.md
  Archives section, commit
- [x] 3. Add `--dry-run` flag to show what would happen without executing
- [x] 4. Add hint to `version milestone` when `current_milestone` becomes None after bump:
  print archive suggestion
- [x] 5. Test: archived 4 completed specs (session-092-hardening, reports-layer,
  version-semver-alignment, verification-module-split), verified tags exist, files removed,
  build.md updated, `archive-completed-work` belief now passes (47/47)

---

## Exit Criteria

- [x] `patina spec archive <id>` creates tag, removes file, updates build.md, commits
- [x] `--dry-run` shows plan without executing
- [x] `archive-completed-work` belief passes after archiving existing 4 specs (0 contested)
- [x] `patina version milestone` prints hint when spec fully completes
