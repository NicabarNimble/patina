# Homebrew/Core Readiness Checklist (Patina)

Use this checklist as a pre-flight gate before proposing `patina` to `homebrew/core`.

## Formula identity

- [ ] Formula name is `patina` and command surface is stable.
- [ ] `desc`, `homepage`, `license` are accurate and concise.

## Release artifacts

- [ ] Tagged GitHub release publishes immutable tarballs for supported targets.
- [ ] Tarballs include executable `patina` at root.
- [ ] Release publishes SHA-256 checksums (`checksums.txt`).
- [ ] Checksums match downloaded artifacts.

## Install behavior

- [ ] Formula installs without network activity during install/test.
- [ ] Formula `test do` is deterministic and non-interactive.
- [ ] Binary runs `patina --version` successfully post-install.

## Service behavior

- [ ] Formula includes service block running `patina mother start`.
- [ ] Service keeps alive and writes logs to Homebrew log paths.
- [ ] Service start/stop/restart works via `brew services`.

## Runtime UX

- [ ] README documents brew install + service management clearly.
- [ ] README clarifies update flow (`brew upgrade`, service restart if needed).
- [ ] README clarifies that Homebrew users should prefer `brew services` over manual daemon bootstraps.

## Maintainer readiness

- [ ] Release process is documented and repeatable.
- [ ] Versioning/tagging discipline is consistent.
- [ ] Maintainer response process exists for formula breakage reports.

## Channel model

- [ ] Stable channel is default and first-class.
- [ ] Beta/nightly are opt-in and do not destabilize stable formula behavior.
- [ ] Channel semantics are documented in release notes and installer docs.