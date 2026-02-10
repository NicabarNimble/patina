---
type: belief
id: wasi-sandboxed-filesystem
persona: architect
facets: [wasm, security, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# wasi-sandboxed-filesystem

Plugins access filesystem through WASI with virtual paths mapped to isolated work directories - they cannot escape their sandbox

## Statement

Plugins access filesystem through WASI with virtual paths mapped to isolated work directories - they cannot escape their sandbox

## Evidence

- [[session-20260205-115835]]: [[session-20260205-115835]] - Zed's path_from_extension() pattern: extensions see /work/*, host translates to real paths (weight: 0.9)

## Supports

- [[two-layer-capability-grants]] — Filesystem access is a capability that must be granted
- [[separate-worlds-for-isolation]] — Only plugins with filesystem needs import wasi:filesystem

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- "Some plugins need real path access" — Addressed by host-mediated APIs (not raw WASI)

## Applied-In

- [[wit-interfaces]] — scraper-plugin and work-plugin import wasi:filesystem with virtual paths
- [[patina-platform]] — Plugin storage in ~/.patina/plugins/{name}/work/

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
