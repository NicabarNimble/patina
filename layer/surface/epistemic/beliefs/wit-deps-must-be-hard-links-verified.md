---
type: belief
id: wit-deps-must-be-hard-links-verified
persona: architect
facets: [plugin, wit, build-hygiene, enforcement]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-18
---

# wit-deps-must-be-hard-links-verified

WIT host interface is single-source-of-truth. All world and guest crate `host.wit` files MUST be symlinks back to canonical `wit/deps/patina-host/host.wit`. Pre-push enforces this.

## Statement

WIT host interface is single-source-of-truth. All world and guest crate `host.wit` files MUST be symlinks back to canonical `wit/deps/patina-host/host.wit`. Stale copies cause silent split-brain: plugins compile against different host imports, and it only explodes when you add one interface. Beliefs without enforcement are vibes — pre-push step [2/5] now enforces `readlink` target equality (no more inode drift from git rewrites).

## Evidence

- [[session-20260213-224126]]: mother-child/deps/patina-host/host.wit was a stale copy missing the query interface; all 4 world dirs AND all 3 guest crate dirs had different inodes despite [[wit-hard-links-not-copies]] belief; 7 stale copies fixed in [[commit-d3e93012]] (weight: 0.95)
- [[session-20260213-224126]]: External agent review identified this as "small violation → future catastrophe" pattern. Host.wit edits should be treated like DB migrations — schema for the entire plugin ecosystem (weight: 0.9)

## Supports

- [[wit-hard-links-not-copies]] — strengthens the single-source WIT invariant by enforcing symlink targets

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Symlink resolution differs between OSes. Mitigated by requiring relative targets (`readlink` can't start with `/`) and verifying resolved paths via pure shell (`cd` + `pwd -P`). No Python — per [[patina-identity]].

## Applied-In

- [[commit-d3e93012]]: Replaced stale `wit/mother-child/deps/patina-host/host.wit` copy with shared reference to canonical. Without this fix, `import patina:host/query@0.1.0` in mother-child world failed with "interface not found in package".
- `resources/git/pre-push-checks.sh` step [2/5]: Checks 8 host.wit symlinks (4 world dirs + 4 SDK mirrors) against canonical target. `readlink` + `cd/pwd -P` (pure shell) ensures both the link string is relative and it resolves to the canonical file.

## Verification

```verify type="assay" label="pre-push script checks host.wit" expect=">= 1"
functions --pattern "pre-push-checks" | count(distinct file)
```

## Revision Log

- 2026-02-13: Created — aspiration belief after mother-child stale copy incident
- 2026-02-13: Strengthened — enforcement tooling added to pre-push, all 7 copies fixed, entrenchment raised to high
- 2026-02-18: Updated — enforcement now expects relative symlinks instead of hard links so git operations can't break invariants.
