---
type: belief
id: wit-hard-links-not-copies
persona: architect
facets: [architecture, plugin-system, wit]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-18
---

# wit-hard-links-not-copies

WIT files across workspaces must share a single source of truth — implemented today with relative symlinks to the canonical host interface so git operations can't desync the copies.

## Statement

WIT files across workspaces should never be standalone copies. They must reference the canonical file so edits propagate automatically. Originally we used hard links; after git checkouts repeatedly broke inodes, we moved to relative symlinks that survive git writes while keeping file-system level sharing.

## Evidence

- [[session-20260213-163217]] - Discovered during [[plugin-host-http]] implementation: `patina-plugin-api/wit/` files share inodes with `wit/`. Verified via `stat -f '%i'` — all 3 host.wit copies and both mother-child.wit copies are hard links. No manual sync step needed. (weight: 0.9)

## Supports

- [[sanitize-at-data-level-not-just-control-flow]] — single source of truth prevents data-level drift

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Relative symlinks require deliberate path management when directories move. The hook + helper script must be updated alongside any layout change or the symlink targets will dangle.

## Applied-In

- All eight `deps/patina-host/host.wit` consumers (four worlds + SDK mirrors) are symlinks to `wit/deps/patina-host/host.wit`.
- Pre-push check `resources/git/pre-push-checks.sh` validates symlink targets via `readlink` + `cd/pwd -P` (pure shell, no Python) so platform tooling never drifts.

## Revision Log

- 2026-02-13: Created — metrics computed by `patina scrape`
- 2026-02-18: Revised — switch from hard links to relative symlinks to survive git rewrites.
