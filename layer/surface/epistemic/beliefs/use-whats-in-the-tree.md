---
type: belief
id: use-whats-in-the-tree
persona: architect
facets: [rust, dependencies, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-03
revised: 2026-02-03
---

# use-whats-in-the-tree

Before writing code, check cargo tree and existing patterns. Use dependencies already compiled into the binary — don't introduce new crates when the solution exists in the tree. Evolve existing architecture, don't invent parallel ones.

## Statement

Before writing code, check cargo tree and existing patterns. Use dependencies already compiled into the binary — don't introduce new crates when the solution exists in the tree. Evolve existing architecture, don't invent parallel ones.

## Evidence

- [[session-20260203-192041]] - Security hardening needed sha2, zeroize, console — all three already in tree via age and console crates. Discovery happened during spec review, not after coding. Zero new dependencies across all four phases. (weight: 0.9)
- Cargo dependency tree is a sunk cost — every crate already compiled is free to use directly. Adding a new crate has compile time, audit surface, and version coordination costs. (weight: 0.7)

## Supports

- [[dependable-rust]] — small stable interfaces means reusing existing modules, not adding parallel ones
- [[transport-security-by-trust-boundary]] — Phase 2 transport security uses only stdlib `std::os::unix::net`, zero new deps

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Sometimes the right tool genuinely isn't in the tree. This belief is "check first and present tradeoffs," not "never add deps." When an existing dep is a poor fit or a new crate is clearly better, present the pros and cons: compile cost, audit surface, version coordination, maintenance burden vs. hand-rolling risk, code quality, and time. The decision is the human's.

## Applied-In

- `layer/surface/build/refactor/security-hardening/SPEC.md` — Phase 3 uses `sha2` (via `age`), Phase 4 uses `console` (already in tree) and `zeroize` (via `age`). Zero new Cargo.toml entries.
- `CLAUDE.md` — AI workflow rule: "check cargo tree for existing deps" before coding

## Revision Log

- 2026-02-03: Created — metrics computed by `patina scrape`
