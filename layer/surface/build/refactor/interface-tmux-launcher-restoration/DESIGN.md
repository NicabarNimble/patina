# Design: refactor: restore tmux interface launch lanes safely

## Why This Design

This is a constrained restoration spec.

We are restoring a proven launcher capability (tmux lanes for Claude/OpenCode/Gemini) while
explicitly preserving architecture wins from recent refactors:

- Mother/session runtime owns liveness and durability.
- Interface launch remains bundle-driven and policy-governed.
- No return to hidden coupling where tmux lane existence implies session truth.

The design intentionally splits "transport ergonomics" from "runtime truth" to avoid reintroducing
the failures that motivated the refactor wave.

## Build Target

Deliver tmux-capable launch transport for the three AI interfaces with explicit policy controls,
stable lane naming, and safe fallback behavior, while maintaining Mother-owned session semantics.

Minimum functional target:

1. `patina ai {claude|opencode|gemini}` can launch in tmux when requested/permitted.
2. `--no-tmux` always forces direct launch.
3. Missing/unsuitable tmux never blocks launch; it degrades to direct launch with clear warning.
4. No code path treats tmux lane as authoritative session liveness.
5. Bundle metadata can express launch policy (foundation for tarball-vibe interfaces).

## Resolved Decisions

1. **Policy surface**: use explicit launch flags (`--tmux`, `--no-tmux`) at AI command layer.
2. **Truth boundary**: Mother/session runtime remains liveness and archive source of truth.
3. **Bundle evolution**: launch policy belongs in interface bundle metadata, not ad-hoc command branches.
4. **Fallback contract**: launch never hard-fails solely due to tmux availability/version mismatch.
5. **Regression guard**: no reintroduction of end_tag claims without matching real git tags.

## Commits

1. `spec: lock tmux restoration boundaries and guardrails`
   - Finalize TL criteria and this design contract before code edits.

2. `refactor(interface): restore launch request tmux contract`
   - Re-extend `LaunchRequest` and interface launch plumbing to carry tmux policy + lane identity.

3. `feat(interface): restore tmux launcher transport with safe fallback`
   - Add tmux decision/lane derivation/exec path in launcher internals.

4. `feat(ai): add explicit tmux launch flags`
   - Add `--tmux` and `--no-tmux` to AI launch args and wire decision inputs.

5. `refactor(interface): encode launch policy in bundle metadata`
   - Add launch policy fields/defaults for claude/opencode/gemini bundles.

6. `test: cover tmux decision, lane stability, and fallback behavior`
   - Add targeted tests across interface and AI launch surfaces.

7. `chore: align scripts/docs with launcher policy`
   - Remove stale assumptions and ensure CLI/help/scripts reflect restored tmux behavior.

## Direct Code Targets

- `src/commands/ai/mod.rs` — add tmux flags in `AiLaunchArgs`, parse behavior tests.
- `src/commands/ai/surface.rs` — restore tmux decision inputs and launch contract wiring.
- `src/interface/mod.rs` — extend `LaunchRequest`; keep trait surface minimal.
- `src/interface/internal/launcher.rs` — tmux transport path + direct fallback.
- `src/interface/internal/bundle.rs` — launch policy metadata for interface bundles.
- `src/interface/internal/surface.rs` — consume bundle launch defaults where appropriate.
- `resources/scripts/check-core-verb-policy.sh` — align any stale launcher flag assumptions.

## Verification Plan

1. Baseline compile/test:
   - `cargo check -q`
   - `cargo test -q`

2. Targeted launcher tests:
   - decision precedence (`--tmux` vs `--no-tmux` vs env/runtime conditions)
   - lane naming stability and per-interface uniqueness
   - tmux unavailable/too-old fallback behavior

3. CLI surface checks:
   - `patina ai --help`
   - `patina ai claude --help`
   - `patina ai opencode --help`
   - `patina ai gemini --help`

4. Runtime behavior smoke (manual):
   - launch with tmux and verify attach/reuse semantics
   - launch with `--no-tmux` and verify direct path
   - verify no new session/archive path relies on tmux lane state as truth

## Build Readiness

Ready for implementation.

Risk controls are explicit:

- do not rebind session truth to tmux,
- do not regress tag integrity semantics,
- do not bypass interface-bundle policy modeling.

## Open Questions

- Should default launch policy be `auto` or explicit-opt-in tmux for each interface bundle?
- Should tmux telemetry/logging be restored now or deferred to a follow-on observability slice?
