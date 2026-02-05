---
type: belief
id: two-layer-capability-grants
persona: architect
facets: [wasm, security, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# two-layer-capability-grants

Plugin capabilities require two-layer grants: manifest declares what the plugin wants, host decides what to allow

## Statement

Plugin capabilities require two-layer grants: manifest declares what the plugin wants, host decides what to allow

## Evidence

- [[session-20260205-115835]]: [[session-20260205-115835]] - Zed's CapabilityGranter pattern: manifest.allow_exec() + granted_capabilities check (weight: 0.9)

## Supports

- [[dependable-rust]] — Explicit capability boundaries align with "small, stable interfaces"
- [[separate-worlds-for-isolation]] — Two-layer grants complement world-level isolation

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Convenience argument: "Just trust plugins, two layers is overhead" — Defeated by security-first design

## Applied-In

- [[patina-platform]] — Defines capability system for WASM plugins
- [[wit-interfaces]] — Host interfaces expose only granted capabilities

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
