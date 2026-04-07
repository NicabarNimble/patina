---
type: feat
id: pando-platform-phase-a
status: active
created: 2026-04-07
related:
- layer/surface/build/feat/pando-platform/SPEC.md
- mother/src/pando.rs
- mother/src/http_api.rs
- mother/src/http_routes.rs
- src/commands/mother/daemon.rs
- src/commands/pando.rs
- src/main.rs
- crates/patina-protocol/src/lib.rs
exit_criteria:
- id: pp1-pando-manifest
  text: '`pando.toml` format defined and parsed by Mother. Declares name, description, children, composition wiring, and commands.'
  checked: true
- id: pp2-mother-pando-registry
  text: Mother reads `pando.toml` files, builds a pando registry, and maps command namespaces to pandos. Rejects registration when a command namespace collides with an existing pando or a native binary command.
  checked: true
---
# feat: Pando Platform Phase A

## Scope

This spec isolates the completed platform foundation work that originally lived
inside `pando-platform`: manifest parsing and Mother-side pando registration.

## Delivered

- Strict `pando.toml` parsing for `[pando]`, `[[children]]`, `[commands.*]`,
  and `[composition]` with typed args (`string`, `flag`, `int`, `strings`).
- Mother pando registry loading from `~/.patina/pandos/`.
- Three-tier collision rejection:
  - native command collisions,
  - pando-vs-pando namespace collisions,
  - alias-vs-pando collisions.
- Lifecycle status projection (`loaded`, `degraded`, `error`) via registry list.
- `patina pando list` command routed through Mother.

## Verification

```bash
cargo check -p patina-protocol
cargo check -p mother
cargo check -p patina-ai
cargo test -p mother
patina pando list
```

## Notes

Workspace-wide `cargo check --workspace -q` is currently blocked by an existing
compile error in `children/belief-verifier` that is outside this Phase A scope.
