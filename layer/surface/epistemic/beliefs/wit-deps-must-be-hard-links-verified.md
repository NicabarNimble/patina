---
type: belief
id: wit-deps-must-be-hard-links-verified
persona: architect
facets: [plugin, wit, build-hygiene]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-13
---

# wit-deps-must-be-hard-links-verified

After any change to wit/deps/patina-host/host.wit, verify all world-level copies share the same inode — stale copies cause silent build failures that only surface when new imports are added

## Statement

After any change to wit/deps/patina-host/host.wit, verify all world-level copies share the same inode — stale copies cause silent build failures that only surface when new imports are added

## Evidence

- [[session-20260213-224126]]: [[session-20260213-224126]] - mother-child/deps/patina-host/host.wit was a stale copy missing the query interface; all 4 world dirs had different inodes despite [[wit-hard-links-not-copies]] belief; fixed by replacing with hard link in [[commit-d3e93012]] (weight: 0.95)

## Supports

- [[wit-hard-links-not-copies]] — strengthens the original belief with enforcement guidance

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[commit-d3e93012]]: Replaced stale `wit/mother-child/deps/patina-host/host.wit` copy with hard link to canonical `wit/deps/patina-host/host.wit`. Without this fix, `import patina:host/query@0.1.0` in mother-child world failed with "interface not found in package".

## Revision Log

- 2026-02-13: Created — metrics computed by `patina scrape`
