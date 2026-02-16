---
type: belief
id: wit-hard-links-not-copies
persona: architect
facets: [architecture, plugin-system, wit]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-13
---

# wit-hard-links-not-copies

WIT files across workspaces should be hard links, not copies — hard links eliminate sync drift by making the canonical source and all consumers the same inode.

## Statement

WIT files across workspaces should be hard links, not copies — hard links eliminate sync drift by making the canonical source and all consumers the same inode.

## Evidence

- [[session-20260213-163217]] - Discovered during [[plugin-host-http]] implementation: `patina-plugin-api/wit/` files share inodes with `wit/`. Verified via `stat -f '%i'` — all 3 host.wit copies and both mother-child.wit copies are hard links. No manual sync step needed. (weight: 0.9)

## Supports

- [[sanitize-at-data-level-not-just-control-flow]] — single source of truth prevents data-level drift

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Hard links break if one copy is deleted and recreated (e.g., by `git checkout` or editor save-as-new-file). Symlinks would survive this but introduce their own issues (relative path resolution across directories).

## Applied-In

- `wit/deps/patina-host/host.wit` ↔ `patina-plugin-api/wit/deps/patina-host/host.wit` (same inode)
- `wit/mother-child/deps/patina-host/host.wit` ↔ `patina-plugin-api/wit/mother-child/deps/patina-host/host.wit` (same inode)
- `wit/mother-child/mother-child.wit` ↔ `patina-plugin-api/wit/mother-child/mother-child.wit` (same inode)
- Pre-push check `resources/git/pre-push-checks.sh` validates WIT consistency as safety net

## Revision Log

- 2026-02-13: Created — metrics computed by `patina scrape`
