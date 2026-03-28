# Design: sdk-defined child CLI command surfaces

## Why This Design

Patina already treats several command domains as child-routed at the UX level, but current routing still relies on hardcoded command-family dispatch and core-owned business logic paths. This design completes the boundary so child-routed commands become child-owned in execution, and third-party SDK children can add command namespaces without core feature code.

This preserves the product experience (`patina <namespace> ...`) while enforcing role boundaries:

- CLI: parse and render.
- Mother: mediate and route.
- Child: own business logic.

## Build Target

1. Add/lock command-surface WIT contract in `wit/command-handler/` as compile-time interface for command-capable children.
2. Add command-registration metadata in child manifest surface.
3. Enforce manifest-to-needs coherence checks for command capability declarations.
4. Introduce a generic child command dispatch envelope in protocol.
5. Add CLI command-router resolution from child metadata.
6. Add SDK helpers for command request/response handling.
7. Migrate `spec` path to full child ownership.
8. Prove third-party SDK child can provide a command namespace.
9. Prove atomic command-surface refresh on child update/reload.

## Resolved Decisions

- Keep command syntax stable at top-level (`patina spec`, `patina lake`, etc.).
- Reserve core bootstrap/admin namespaces and reject overrides.
- Use deterministic command collision policy and explicit aliases.
- Preserve JSON output and HITL confirmation semantics during migration.
- Command routing never authorizes capability use; grants remain the only authority.
- Migrated command families must not retain silent core fallback execution paths.
- Mother is the authoritative validation layer for command registry/coherence; CLI renders diagnostics.
- Child updates apply command registry changes via atomic snapshot swap (no mixed route states).

## Commits

1. `feat(spec): define child command surface contract` - manifest schema, protocol envelope, and docs.
2. `feat(wit): lock command interface in wit/command-handler` - compile-time contract for command-capable children.
3. `feat(cli): add generic child command router` - command resolution, dispatch, and help rendering.
4. `feat(sdk): add child command handler primitives` - SDK request/response types and registration support.
5. `refactor(spec): migrate spec command family to full child ownership` - remove core fallback path for spec lifecycle.
6. `feat(example): add sdk-only third-party command child proof` - end-to-end non-builtin namespace validation.
7. `feat(mother): atomic command registry refresh` - snapshot swap on child update/reload.
8. `refactor(cleanup): remove dead hardcoded builtin command dispatch` - post-migration simplification.

## Direct Code Targets

- `crates/patina-protocol/src/lib.rs` - generic child command request/response envelope.
- `wit/command-handler/command-handler.wit` - command-capable child interface contract (new package, not the retired one-shot `patina:command` world).
- `wit/command-handler/deps/` - command-handler dependency lane aligned to current per-package toy import conventions.
- `src/main.rs` - command parsing/routing integration entrypoints.
- `src/commands/spec/mod.rs` - migrate from command-family-specific dispatch to generic routing.
- `src/commands/lake.rs` - align to generic routing path.
- `src/commands/doctor.rs` - align to generic routing path.
- `src/commands/mother/daemon.rs` - dispatch to child-owned command handlers.
- `mother/src/http_api.rs` - route-table support for generic child command dispatch.
- `children/spec-manager/src/lib.rs` - full lifecycle ownership.
- `children/spec-manager/child.toml` - command registration metadata.
- `sdk/patina-sdk/src/` - command handler helper APIs and macro support.

## Verification Plan

Core gates:

```bash
cargo check --workspace -q
cargo test -q --workspace
```

Behavior checks:

```bash
patina spec --help
patina spec check child-command-surface --json
patina spec list --json
```

Guardrail checks:

```bash
# 1) No privilege-by-name: command route succeeds, child capability check denies
<integration-test for denied capability path>

# 2) Reserved/collision policy enforcement
<integration-test for reserved namespace registration failure>
<integration-test for deterministic collision precedence>

# 2b) Manifest-needs coherence
<integration-test for [provides.commands].requires mismatch failure>
<integration-test for valid coherence pass>

# 3) No silent fallback: child unavailable should fail clearly
patina mother stop
patina spec list   # should fail with spec-manager unavailable error

# 4) Atomic refresh semantics
<integration-test for child command add/remove/rename across update>
<assert no mixed old/new route availability during swap>
```

Command-surface checks (post-implementation):

```bash
patina <third-party-namespace> --help
patina <third-party-namespace> <verb> --json
```

## Build Readiness

Ready to execute in phased slices. No blocking open decision remains for Phase A start.

## Open Questions

- What minimum command metadata is required for shell completion generation?
