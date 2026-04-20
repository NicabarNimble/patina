# Design: Distribution channels: Homebrew tap + curl installer + release pipeline

## Why This Design

Patina needs distribution that matches user intent (`brew install`, `curl | sh`) without fragmenting the runtime surface. The clean approach is one installed binary (`patina`) with multiple distribution channels, not multiple executables.

## Build Target

- GitHub tag releases publish installable artifacts + checksums.
- Homebrew tap formula can consume those artifacts.
- Curl installer can consume those artifacts with checksum verification.
- Stable/beta/nightly channel model is explicit and documented.

## Resolved Decisions

1. Keep one binary: `patina`.
2. Stable is default for both brew and curl paths.
3. Beta/nightly are opt-in channels.
4. Homebrew service model (`brew services`) is preferred for daemon lifecycle.
5. Maintain a repository-local checklist before attempting `homebrew/core`.

## Commits

1. `ci(release): publish multi-target archives with checksums`
2. `feat(distribution): add curl installer with channel + version selection`
3. `build(homebrew): add patina formula template with mother service`
4. `docs(release): add brew/core readiness checklist and distribution docs`

## Direct Code Targets

- `.github/workflows/release.yml` — artifact build/package/upload pipeline
- `install.sh` — curl installer entrypoint
- `packaging/homebrew/Formula/patina.rb` — tap/core formula template
- `layer/surface/build/feat/distribution-homebrew-curl/BREW_CORE_CHECKLIST.md` — readiness gate
- `README.md` — user-facing install/service/update guidance

## Verification Plan

1. Trigger tag release in dry-run branch or workflow_dispatch equivalent and inspect uploaded assets.
2. Run installer locally in temp bin dir with stable channel.
3. Validate formula syntax and service stanza.
4. Confirm README flow matches shipped artifacts and command surface.

## Build Readiness

Ready. This is packaging/distribution infrastructure only; minimal blast radius to runtime behavior.

## Open Questions

1. Should beta/nightly use moving tags (`beta`, `nightly`) or immutable semver prerelease tags only?
2. Do we want automatic tap formula bumping in CI, or a manual release gate PR?
3. Should curl installer support detached signature verification in addition to SHA-256?