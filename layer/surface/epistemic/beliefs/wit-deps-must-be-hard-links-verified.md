---
type: belief
id: wit-deps-must-be-hard-links-verified
persona: architect
facets: [plugin, wit, build-hygiene, enforcement]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-13
---

# wit-deps-must-be-hard-links-verified

WIT host interface is single-source-of-truth. All world and guest crate host.wit files MUST be hard links to canonical wit/deps/patina-host/host.wit. Pre-push enforces this.

## Statement

WIT host interface is single-source-of-truth. All world and guest crate host.wit files MUST be hard links to canonical `wit/deps/patina-host/host.wit`. Stale copies cause silent split-brain: plugins compile against different host imports, and it only explodes when you add one interface. Beliefs without enforcement are vibes — pre-push step [2/5] enforces inode identity with content-match fallback.

## Evidence

- [[session-20260213-224126]]: mother-child/deps/patina-host/host.wit was a stale copy missing the query interface; all 4 world dirs AND all 3 guest crate dirs had different inodes despite [[wit-hard-links-not-copies]] belief; 7 stale copies fixed in [[commit-d3e93012]] (weight: 0.95)
- [[session-20260213-224126]]: External agent review identified this as "small violation → future catastrophe" pattern. Host.wit edits should be treated like DB migrations — schema for the entire plugin ecosystem (weight: 0.9)

## Supports

- [[wit-hard-links-not-copies]] — strengthens from aspiration to enforced invariant

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Filesystems that don't preserve hard links (rare: some archive tools, network mounts). Mitigated by Strategy 2 content-match fallback in pre-push check.

## Applied-In

- [[commit-d3e93012]]: Replaced stale `wit/mother-child/deps/patina-host/host.wit` copy with hard link to canonical. Without this fix, `import patina:host/query@0.1.0` in mother-child world failed with "interface not found in package".
- `resources/git/pre-push-checks.sh` step [2/5]: Checks 7 host.wit copies (4 world dirs + 3 guest crates) against canonical inode. Content-match fallback distinguishes "not linked but matching" from "content diverged (split-brain)".

## Verification

```verify type="assay" label="pre-push script checks host.wit" expect=">= 1"
functions --pattern "pre-push-checks" | count(distinct file)
```

## Revision Log

- 2026-02-13: Created — aspiration belief after mother-child stale copy incident
- 2026-02-13: Strengthened — enforcement tooling added to pre-push, all 7 copies fixed, entrenchment raised to high
