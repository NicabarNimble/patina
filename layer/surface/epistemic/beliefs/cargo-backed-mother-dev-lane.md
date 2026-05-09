---
type: belief
id: cargo-backed-mother-dev-lane
persona: patina
facets: [development, installation, mother, launchd]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-05-08
revised: 2026-05-08
---

# cargo-backed-mother-dev-lane

Local Patina development should use the cargo-installed binary with Patina-owned launchd supervision; Homebrew remains the distribution and release-validation lane unless intentionally selected as service owner.

## Statement

Local Patina development should use the cargo-installed binary with Patina-owned launchd supervision; Homebrew remains the distribution and release-validation lane unless intentionally selected as service owner.

## Evidence

- In [[session-20260508-112917-717692000]], Homebrew was installed but shadowed by ~/.cargo/bin/patina; switching to patina mother install from the cargo binary made launchd run the fast local dev build while preserving Homebrew for release distribution.

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-05-08: Created — metrics computed by `patina scrape`
