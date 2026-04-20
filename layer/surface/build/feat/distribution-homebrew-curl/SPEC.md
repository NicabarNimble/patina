---
type: feat
id: distribution-homebrew-curl
status: active
created: 2026-04-20
sessions:
  origin: 20260416-133521-394965000
related:
- .github/workflows/release.yml
- .github/workflows/test.yml
- Cargo.toml
- README.md
- install.sh
- packaging/homebrew/Formula/patina.rb
- layer/surface/build/feat/distribution-homebrew-curl/BREW_CORE_CHECKLIST.md
exit_criteria:
- id: dhc1-release-artifact-matrix
  text: Release workflow builds tagged binaries for macOS (arm64/x86_64) and Linux (x86_64), packages `patina-<target>.tar.gz`, and uploads checksums.
  checked: false
- id: dhc2-curl-installer-stable-first
  text: Root `install.sh` installs Patina from GitHub Releases with checksum verification and supports explicit channel selection (stable/beta/nightly) plus version pinning.
  checked: true
- id: dhc3-homebrew-formula-template
  text: Repository contains a Homebrew formula template for `patina` with launchd service stanza using `patina mother start`.
  checked: true
- id: dhc4-core-readiness-checklist
  text: Brew/core readiness checklist exists and is actionable as an internal release gate.
  checked: true
- id: dhc5-docs-operator-flow
  text: README documents stable distribution flow (tap install, brew services, curl install) and update semantics.
  checked: true
validated_against_commit: c2c73c42
last_freshness_check: 2026-04-20
freshness_scope:
- .github/workflows/release.yml
- README.md
---
# feat: Distribution channels: Homebrew tap + curl installer + release pipeline

> Build stable-first distribution infrastructure for Patina: reproducible release artifacts, Homebrew tap path, curl installer path, and a documented readiness gate for eventual `homebrew/core` submission.

## Problem

Patina currently has source-first distribution (`cargo install --path .`) and a minimal tag release workflow that only creates a GitHub release shell. This does not produce installable artifacts for Homebrew or curl-based install UX.

## Goal

1. Produce release assets suitable for package managers.
2. Provide a secure curl installer with checksum verification.
3. Provide a Homebrew formula template with service semantics for Mother.
4. Define a release-ready checklist aligned to Homebrew expectations.
5. Keep stable as default while allowing beta/nightly channels.

## Status

Active — implementation underway.

## Non-Goals

- Automatic push into `homebrew/core` in this slice.
- Building every Linux target/variant in this slice.
- Replacing single-binary `patina` with multi-binary distribution.
- Implementing a full self-update subsystem.

## Target Shape

- Tagged release creates binary tarballs + checksums.
- `install.sh` resolves channel/version, downloads artifact, verifies checksum, installs `patina` binary.
- Homebrew formula template supports:
  - `brew install <tap>/patina`
  - `brew services start patina` for Mother.
- Checklist captures objective criteria for core submission readiness.

## Solution

1. **Release workflow hardening**
   - Replace minimal release workflow with matrix build packaging and upload.
   - Include checksums artifact for installer verification.

2. **Curl installer**
   - Add root `install.sh` with stable default channel.
   - Support `--channel stable|beta|nightly`, `--version`, `--bin-dir`, `--no-verify`.
   - Verify asset against release `checksums.txt` by default.

3. **Homebrew bootstrap path**
   - Add `packaging/homebrew/Formula/patina.rb` as canonical template.
   - Include service stanza to run `patina mother start`.

4. **Operational docs + checklist**
   - Add `BREW_CORE_CHECKLIST.md` in this spec directory.
   - Update README with distribution commands and service-management guidance.

## Implementation Order

1. Add/upgrade release workflow for artifacts + checksums.
2. Add curl installer script and validate argument behavior.
3. Add Homebrew formula template.
4. Add brew/core readiness checklist.
5. Update README distribution section.

## Resolved Decisions

- Keep one CLI binary (`patina`) for all channels.
- Stable remains default for both brew and curl paths.
- Beta/nightly are opt-in channels and do not replace stable semantics.
- Homebrew users should prefer `brew services` management over ad-hoc `nohup`.

## Verification

```bash
# workflow lint (basic)
rg "patina-\$\{\{ matrix.target \}\}\.tar\.gz" .github/workflows/release.yml

# installer behavior
bash install.sh --help
bash install.sh --channel stable --bin-dir /tmp/patina-bin --no-verify

# formula sanity
ruby -c packaging/homebrew/Formula/patina.rb

# docs surface
rg "brew install|brew services|curl" README.md
```

## Exit Criteria

See frontmatter `exit_criteria`.

## Build Readiness

High. Changes are isolated to release packaging, install surfaces, and docs; no protocol/runtime behavior changes required.