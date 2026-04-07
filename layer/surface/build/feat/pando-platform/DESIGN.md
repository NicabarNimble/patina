# Design: pando-platform

## Why This Design
- Phase A is constrained to two deliverables only: parse `pando.toml` (PP1) and build Mother-side registry plus collision checks (PP2), with no Phase B routing work.
- The parser and registry logic are isolated into focused modules so format parsing, collision policy, and daemon wiring stay separable and testable.
- `patina pando list` is wired as a thin CLI-to-Mother control-plane call and serves as the Phase A verification surface.

## Build Target
- Implement PP1 and PP2 end-to-end: strict `pando.toml` parsing, Mother registry loading from `~/.patina/pandos/`, native/pando/alias collision rejection, lifecycle status reporting, startup native-command registration, and `patina pando list` output.

## Resolved Decisions
- Keep pando parser/registry logic in the Mother crate for Phase A so daemon-side loading and state evaluation are local and testable.
- Use strict serde decoding (`deny_unknown_fields`) for manifest structs to reject unknown fields during PP1.
- Use explicit control-plane endpoints for pando registry init/list instead of overloading builtin child dispatch.
- Treat pando namespace as `pando.name` from manifest in Phase A; command-level dispatch remains out of scope.

## Commits
1. `spec: add pando-platform Phase A design` — lock concrete PP1/PP2 implementation targets and verification.
2. `feat: add pando.toml manifest parser — PP1` — add strict parser types and parsing tests.
3. `feat: add Mother pando registry model and collision checks — PP2` — add registry construction, lifecycle evaluation, and collision tests.
4. `feat: add Mother pando registry init/list control-plane — PP2` — wire daemon/runtime protocol and startup initialization.
5. `feat: add patina pando list command — PP2` — add CLI command and output path through Mother.

## Direct Code Targets
- `mother/src/pando.rs` — new parser + registry model, validation, and tests.
- `mother/src/lib.rs` — export new pando module.
- `crates/patina-protocol/src/lib.rs` — add typed protocol contracts for pando init/list.
- `mother/src/http_routes.rs` — add pando registry routes.
- `mother/src/http_api.rs` — add pando init/list handlers + runtime trait hooks.
- `src/commands/mother/daemon.rs` — host pando registry state, load manifests, apply native list init, serve list responses.
- `src/mother/internal.rs` — add client methods for pando init/list endpoints.
- `src/commands/pando.rs` — add `patina pando list` command surface.
- `src/commands/mod.rs` — register pando command module.
- `src/main.rs` — add `Pando` top-level command and startup native-command registry init.
- `src/paths.rs` — add `~/.patina/pandos/` path helper.

## Verification Plan
- After each commit: `cargo check --workspace -q` then `cargo test -q --lib`.
- Parser checks: valid manifest parses; missing required fields fail; unknown fields fail.
- Registry checks: native collision rejection, pando-vs-pando rejection, alias-vs-pando rejection, lifecycle state transitions (`loaded`/`degraded`/`error`).
- End-to-end: start Mother, run `patina pando list`, verify output for empty/registered pando set.

## Build Readiness
- Ready. SPEC already defines Phase A scope, collision policy, arg types, registry protocol shape, and lifecycle state semantics.

## Open Questions
- None for Phase A scope in current SPEC.
