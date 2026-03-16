---
type: belief
id: cli-facing-rust-changes-require-reinstall
persona: architect
facets: [rust, cli, workflow, patina]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-12
revised: 2026-03-12
---

# cli-facing-rust-changes-require-reinstall

In Patina, Rust changes that affect CLI behavior are not complete until the binary is rebuilt, reinstalled, and validated through the installed CLI surface.

## Statement

In Patina, Rust changes that affect CLI behavior are not complete until the binary is rebuilt, reinstalled, and validated through the installed CLI surface.

## Evidence

- [[20260312-001728]] During `deterministic-spec-scaffolds`, source changes made `cargo run -- spec show deterministic-spec-scaffolds --handoff` work before the installed `patina` binary did, proving source-only validation was insufficient for CLI-facing changes.
- [[20260312-001728]] Reinstalling with `cargo install --path . --force` restored the installed CLI contract, after which `patina spec show deterministic-spec-scaffolds --handoff` and `patina spec create ...` both behaved as expected.
- [[deterministic-spec-scaffolds]] directly improved CLI-visible spec behavior, which made the mismatch between source behavior and installed-binary behavior materially important to correctness.

## Supports

- [[specs-describe-current-code-not-aspirations]]

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[deterministic-spec-scaffolds]]
- [[20260312-001728]]

## Revision Log

- 2026-03-12: Created — metrics computed by `patina scrape`
