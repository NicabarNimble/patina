---
type: belief
id: enum-earns-keep-when-dispatched
persona: architect
facets: [plugins, rust, type-design]
entrenchment: low
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# enum-earns-keep-when-dispatched

PluginRole enum is speculative structure until production code matches on it — the first match (Mother dispatch by role, scrape grammar selection) validates the enum choice over a plain string.

## Statement

PluginRole enum is speculative structure until production code matches on it — the first match (Mother dispatch by role, scrape grammar selection) validates the enum choice over a plain string.

## Evidence

- [[session-20260305-132827]]: Eskil Steenberg advisory review: PluginRole has zero match statements in production code. Enum buys exhaustive matching and compile-time safety, but only when someone actually matches. Currently only FromStr, Display, and expected_worlds() use it. Harmless but not yet earning its keep. Track first production match. (weight: 0.7)

## Supports

- [[gjengset-lens-type-integrity]] — enums provide type-level guarantees, but only when matched

## Attacks

- [[role-belongs-on-granted-capabilities]] — if the enum isn't dispatched on, caching it is premature too

## Attacked-By

- Enum prevents typos (`role = "conector"`) at parse time — that's value even without production `match`. This belief may self-defeat when first production match lands.

## Applied-In

- `src/plugin/internal/mod.rs` — `PluginRole` enum with `FromStr`, `Display`, `expected_worlds()`. Zero `match` in production code paths (only in validation and display).

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
