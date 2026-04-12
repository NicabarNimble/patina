# Design: Atlas Pando Mother UI

## Value anchors

- **Spec-driven design**: spec first, then narrow slices.
- **Dependable Rust**: small public seams; atlas internals remain private.
- **Unix philosophy**: Mother hosts control-plane routes; atlas command stays single-purpose fallback.

## Slice A — atlas pando identity

- Add `resources/pandos/atlas/pando.toml` first-party manifest.
- Seed this manifest in `commands::pando` init path.
- Ensure pando namespace policy allows atlas pando ownership while native CLI atlas remains additive wrapper.

## Slice B — Mother-hosted atlas web lens

Add read-only routes:

- `GET /atlas`
- `GET /atlas/index.html`
- `GET /atlas/atlas.json`

Implementation reuses existing atlas snapshot + HTML render model.

Auth policy follows existing Mother route posture:

- when router requires auth, atlas web routes also require auth
- UDS local mode remains no-token (file-permission boundary)

## Slice C — always-on posture docs

Document and demonstrate launchd supervisor flow:

- `patina mother install`
- `patina mother status`
- atlas routes queried through Mother socket/address

## Direct code targets

- `resources/pandos/atlas/pando.toml`
- `src/commands/pando.rs`
- `mother/src/http_routes.rs`
- `mother/src/http_api.rs`
- `mother/src/daemon_bootstrap_config.rs`
- `src/commands/mother/daemon.rs`
- `src/commands/atlas/mod.rs`
- `README.md`
