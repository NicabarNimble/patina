---
type: belief
id: sdk-wit-worlds-are-copies
persona: architect
facets: [wit, plugins, sdk, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-25
revised: 2026-02-25
---

# sdk-wit-worlds-are-copies

SDK WIT world files are independent copies, not symlinks — they must be manually synced when adding or changing WIT interfaces

## Statement

SDK WIT world files are independent copies, not symlinks — they must be manually synced when adding or changing WIT interfaces

## Evidence

- [[session-20260225-182257]]: [[session-20260225-182257]] - Phase 1 measurement-coverage WASM build failed because SDK copies of command.wit, mother-child.wit, task.wit lacked the new measure import. host.wit is symlinked but world files are not. (weight: 0.95)

## Supports

- [[plugins-are-three-prong-bundles]] — WIT/SDK/host triad means WIT changes ripple to SDK copies

## Attacks


## Attacked-By

- Could be eliminated by symlinking world files like host.wit, but worlds diverge per-package (different `package` declarations) so copies are correct

## Applied-In

- [[commit-763aabd8]] — Fixed SDK WIT sync for `measure` interface addition
- Pre-push check step 1 ("WIT consistency") catches this automatically

## Revision Log

- 2026-02-25: Created — metrics computed by `patina scrape`
