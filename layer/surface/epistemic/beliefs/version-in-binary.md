---
type: belief
id: version-in-binary
persona: architect
facets: [wasm, plugins, versioning]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# version-in-binary

Embed the plugin API version in the WASM binary at build time so the host can dispatch to the correct interface version at load time

## Statement

Embed the plugin API version in the WASM binary at build time so the host can dispatch to the correct interface version at load time

## Evidence

- [[session-20260205-115835]]: [[session-20260205-115835]] - Zed embeds version bytes in link_section, host reads to pick code path (weight: 0.9)

## Supports

- [[dependable-rust]] — Explicit versioning enables stable interfaces with evolution

## Attacks

- Manifest-only versioning — Binary-embedded is more reliable (can't be edited separately)
- Runtime version negotiation — Static embedding is simpler and faster

## Attacked-By

- "Adds build complexity" — Mitigated by patina_plugin_api crate handling it automatically
- "Version mismatch errors at load time" — Actually a feature: fail fast, clear errors

## Applied-In

- [[wit-interfaces]] — API crate embeds version in WASM link section
- [[patina-platform]] — Host reads version from binary to select dispatch path

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
