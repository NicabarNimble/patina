# Design: Mother Doctrine Cleanup — Legacy Quarantine Before Interface Work

## Why This Design

Start from the tagged `spec/knowledge-child-platform` target, not the
mixed present tree.

The doctrinal baseline is:

- Mother holds authority and continuity
- children hold agency
- toys are granted capabilities
- legacy paths are migration residue, not the target model

The taxonomy this refactor must preserve is:

- Mother = authority, continuity, orchestration, runtime substrate
- child = WASM component with workflow/policy agency
- toy = granted capability surface for bounded work
- primitive/substrate = lower-level mechanism, not automatically a toy
- derived enforcement = policy derived from a grant, not a peer toy by
  default

The design correction in this spec is specifically informed by a
component-model reading of the child platform:

- the host/runtime should provide explicit imports and capabilities
- WIT is part of Mother's language to children, not an incidental wrapper
- the child SDK should reflect those imports truthfully
- the ergonomic path for third-party children should also be the
  capability-honest path
- runtime denial alone is not enough if the child-facing API still feels
  ambient

This refactor is intentionally pre-interface. The goal is to stop
teaching two runtime stories at once before more backend contracts get
built for interface setup, inventory, and launch flows.

This design keeps the stronger flow from `knowledge-child-platform` but
applies the decisions reached in this session:

- Mother is being clarified, not reinvented
- child remains WASM
- toys remain granted capabilities and are still evolving
- WIT remains Mother's language
- DuckLake determines the first serious toy-shape corrections

## Build Target

After this refactor:

- default Mother daemon runs only the knowledge-child target runtime
- legacy `mother-child` remains only as explicit migration mode
- DuckLake no longer teaches ambient HTTP as its primary ingress model
- the SDK visibly separates granted capabilities from Mother/runtime
  substrate
- WIT, manifests, runtime gates, and SDK all tell the same authority
  story

## Baseline Reading

An agent working this spec without prior session context should anchor
on the completed 2026-03-11 build, not the mixed current tree:

1. `git show spec/knowledge-child-platform:layer/surface/build/feat/knowledge-child-platform/SPEC.md`
2. `git show spec/knowledge-child-platform:layer/surface/build/feat/knowledge-child-platform/DESIGN.md`
3. `layer/surface/build/fix/knowledge-child-platform-audit-fixes/SPEC.md`
4. `layer/surface/build/fix/knowledge-child-platform-audit-fixes/DESIGN.md`

Those documents establish the target doctrine and already identify many
of the post-ship gaps this refactor should close.

## Current Mismatch Map

### 1. Default daemon path still mixes target and legacy worlds

- `src/commands/mother/daemon.rs` runs `run_knowledge_cycles()` and then
  immediately runs `tick_legacy_all()` plus `spawn_toy_tracked()`.
- This makes legacy shell-command toys part of Mother's normal runtime
  behavior.

### 2. Registry still normalizes two runtimes

- `src/commands/mother/registry.rs` stores `RegisteredChild::Legacy` and
  `RegisteredChild::Knowledge` in the same registry and exposes both as
  standard flow.
- This is practical for migration, but it is doctrinally noisy unless
  legacy is clearly opt-in.

### 3. SDK exposes ambient host authority

- `crates/patina-child-sdk/src/lib.rs` exposes `GuestHost` and direct
  constructors for every toy.
- That makes all toys appear constructible from ambient host access,
  even when runtime grants are narrower.
- This is exactly the sort of “custom API feel” the component-model
  ecosystem warns against: the runtime may be typed, but the child API
  still feels like a global bag of host powers.

### 4. Proof children teach the wrong primary story

- `plugins/ducklake/src/lib.rs` and `plugins/belief-verifier/src/lib.rs`
  are supposed to be doctrine proofs.
- Today they still encode the universal `GuestHost` pattern instead of
  “Mother grants toys; child receives toys.”

### 5. Scope is not bound into toy objects

- `crates/patina-toy-sdk/src/lib.rs` exposes `LakeToy` methods that take
  free-form lake names on each call.
- This keeps authority enforcement mostly host-side instead of making
  the child-facing object itself represent granted scope.

### 6. DuckLake drifted from the sharper grant model

- The earlier DuckLake design carried authority in a typed grant where
  connector identity, policy, and storage shape traveled together.
- The current plugin DuckLake is easier to ship, but broader in shape:
  `plugins/ducklake/plugin.toml` grants `host_http = true` and the child
  uses generic fetch/lake toys.
- That is workable, but less truthful than the earlier grant-shaped
  model and therefore weaker as the canonical example for third parties.

## Design Rules

### 1. Prefer quarantine over deletion for legacy

This refactor should not lose migration knowledge. Legacy `MotherChild`
and shell-toy code can remain compiled if needed, but they must stop
appearing as normal Mother behavior.

### 2. Child-facing truth matters as much as runtime truth

If runtime denial is correct but child code is authored through an
ambient universal host, the system still teaches the wrong doctrine.
Proof children and SDK shape must be truthful.

### 3. Mother's role must stay explicit

The muddiness in the current system comes partly from Mother's role being
under-specified in day-to-day code. This refactor must keep Mother clear:

- Mother grants authority
- Mother owns continuity and operational substrate
- Mother does not own the child's workflow decisions
- Mother may expose substrate hooks, but those must not masquerade as the
  same kind of thing as granted domain capabilities

### 4. Start scope-binding where authority is most concrete

Lake access is the right first target because it is already central to
DuckLake and clearly represents granted scope. Do not try to redesign
every toy before proving the pattern once.

### 5. Keep the component-model, fix the child-facing contract

This spec does not walk away from WIT or the `knowledge-child` world.
The correction is on top of that foundation:

- keep the world
- keep typed host imports
- keep Mother-owned runtime state and grants
- change the SDK and proof children so they expose only the granted,
  truthful surface as the primary path
- update WIT wherever needed so Mother's declared language matches the
  corrected granted-capability model

### 6. WIT / SDK / runtime alignment is mandatory

This refactor is not done if only one layer becomes truthful.

- If WIT is broad but the SDK is narrow, the contract still lies.
- If runtime gating is strict but WIT and SDK feel ambient, the contract
  still lies.
- If a needed capability is removed instead of re-expressed truthfully,
  the contract is weakened rather than corrected.

Mother's language includes WIT, manifests, runtime grants, and the SDK.
They must converge on one story.

### 7. SDK must distinguish toys from substrate

The SDK should not present everything as one flat capability bag.

- granted toys should read as granted capability bundles
- substrate-facing APIs should be explicitly marked as Mother/runtime
  machinery
- if a substrate hook gets a toy-shaped wrapper for ergonomics, the docs
  must still name it as substrate-backed rather than a normal domain toy

### 8. Preserve the proof children

DuckLake and belief-verifier are not incidental examples. They are the
clearest proof that the doctrine survives contact with real code.

### 9. Third-party authors are part of the design target

The child SDK is not only for internal proof children. It teaches future
plugin authors how Patina thinks about authority. If the SDK's most
ergonomic path is ambient, third-party children will learn the wrong
model even if runtime checks are correct.

### 10. Interface work is downstream

Do not mix project/interface registry work into this refactor. The goal
here is to make Mother stable enough that later interface work can rely
on a clean backend contract.

## Commits

1. `fix(mother): quarantine legacy mother-child runtime by default`
   - Remove legacy heartbeat execution from the default daemon path.
   - Require an explicit migration switch or separate loader.
   - Update runtime comments so the default story is singular.

2. `fix(sdk): inject granted toy bundles into proof children`
   - Replace universal `GuestHost`-first authoring with granted bundles
     in DuckLake and belief-verifier.
   - Keep a compatibility path only if needed for incremental migration,
     but make the proof path explicit and primary.
   - Make the truthful path the ergonomic path for third-party authors.

3. `fix(toys): bind lake authority into granted toy objects`
   - Make lake access reflect granted scope in the child-facing type
      shape.
   - Update knowledge-child runtime glue as needed so granted scope is
      available at toy construction time.

4. `fix(runtime): narrow broad host capability surfaces where needed`
   - Revisit capabilities such as DuckLake's current `host_http = true`
     so the child-facing shape matches the intended grant/policy model.
   - Prefer a grant-shaped toy or interface when broad host power hides
     the real authority boundary.
   - Do not delete genuinely needed DuckLake ingress just to make the
     model look cleaner; re-express it truthfully.

5. `fix(runtime): make TaskIntent layering explicit`
   - Clarify whether `TaskIntent` is a toy or a runtime substrate.
   - Ensure it cannot be read as a generic authority backdoor.
   - Make the SDK language match the taxonomy even if a wrapper remains
     ergonomic.

6. `test(mother): prove doctrine through denied-toy and workflow tests`
   - Lock down denied toy absence, legacy quarantine, and proof-child
      workflow semantics.
   - Prefer tests that prove architecture rather than only API behavior.

7. `docs(mother): align runtime narrative with knowledge-child doctrine`
    - Update docs/specs/comments so Mother's story is singular and
        truthful.

## Resolved Decisions

These are fixed for this design:

- child = WASM component
- Mother owns authority, continuity, orchestration, and runtime
  substrate
- toys are granted capability surfaces
- primitives/substrate are not toys by default
- derived enforcement is not a peer toy by default
- DuckLake ingress ends on a granted source/connector capability, not
  generic ambient HTTP
- `TaskIntent` is Mother/runtime substrate
- WIT must change where the authority story changes
- legacy `mother-child` stays only as migration residue

## Detailed Implementation Notes

### Commit 1: Quarantine legacy runtime

Primary files:

- `src/commands/mother/daemon.rs`
- `src/commands/mother/registry.rs`
- `src/mother/child.rs`
- `src/plugin/internal/mod.rs`

Desired result:

- Default daemon heartbeat performs only knowledge-child work.
- Legacy `MotherChild` execution requires an explicit migration switch,
  loader branch, or separate command path.
- Comments and API docs say “migration bridge,” not “co-equal runtime.”

Direct code targets:

- `src/commands/mother/daemon.rs:120` — remove default
  `tick_legacy_all()` / `spawn_toy_tracked()` path from the normal
  heartbeat.
- `src/commands/mother/mod.rs:62` — add explicit CLI flag for legacy
  child mode.
- `src/commands/mother/daemon.rs:538` — carry that option into
  `DaemonOptions`.
- `src/commands/mother/daemon.rs:563` — gate legacy WASM child loading on
  explicit opt-in.
- `src/commands/mother/registry.rs:94` — legacy ticking remains only as a
  migration-only branch or helper, not default runtime flow.

### Commit 2: Granted toy bundles become primary SDK story

Primary files:

- `crates/patina-child-sdk/src/lib.rs`
- `plugins/ducklake/src/lib.rs`
- `plugins/belief-verifier/src/lib.rs`

Desired result:

- Child code no longer starts from “construct every toy from
  `GuestHost`.”
- Proof children receive explicit bundles shaped around their grants.
- If `GuestHost` survives temporarily, it is clearly secondary or
  compatibility-only.
- A third-party child author reading the SDK sees the truthful model
  first: declare grants, receive a granted surface, code against that.

Direct code targets:

- `crates/patina-child-sdk/src/lib.rs:22` — granted capability area stays
  visible and grows into the primary authoring path.
- `crates/patina-child-sdk/src/lib.rs:92` — substrate area stays visibly
  separate from granted toys.
- `plugins/ducklake/src/lib.rs:8` and `plugins/belief-verifier/src/lib.rs:8`
  — proof children must read as granted-bundle consumers, not ambient-host
  builders.

### Commit 3: Bind lake authority into types

Primary files:

- `crates/patina-toy-sdk/src/lib.rs`
- `src/plugin/internal/knowledge_child.rs`
- `src/mother/lake_host.rs`
- `plugins/ducklake/src/lib.rs`

Desired result:

- The child-facing lake toy represents granted authority, not arbitrary
  name selection.
- DuckLake code reads like “use the granted lake toy” rather than “pass
  lake id through every call.”

Direct code targets:

- `crates/patina-toy-sdk/src/lib.rs:251` — replace free-form lake name
  methods with a scoped/granted lake object shape.
- `src/plugin/internal/knowledge_child.rs:205` — host bindings should
  construct or expose lake capability according to granted scope.
- `plugins/ducklake/src/lib.rs:102` — DuckLake should consume a granted
  lake object rather than threading lake names through operations.

### Commit 4: Narrow broad host capability surfaces

Primary files:

- `plugins/ducklake/plugin.toml`
- `crates/patina-child-sdk/src/lib.rs`
- `crates/patina-toy-sdk/src/lib.rs`
- `src/plugin/internal/knowledge_child.rs`

Desired result:

- Broad capabilities that hide the real authority boundary are reduced or
  re-expressed.
- DuckLake no longer teaches “ambient HTTP is fine” if the real intended
  model is connector/policy-shaped authority.
- The imported/component-facing contract is closer to the earlier typed
  DuckLake grant model while remaining reusable.
- WIT changes are made where needed so the granted model is visible in
  Mother's declared interface language, not only in Rust wrappers.

Concrete decision for this spec:

- DuckLake does **not** end this spec on generic `host_http` as its
  primary granted capability.
- DuckLake ingress becomes a granted source/connector capability.
- Generic HTTP may remain as lower-level substrate for other uses, but it
  is not the doctrinal DuckLake example.

Direct code targets:

- `plugins/ducklake/plugin.toml:9` — replace broad DuckLake `host_http`
  primary story with granted ingress capability declarations.
- `wit/knowledge-child/deps/patina-host/host.wit:38` — add a dedicated
  grant-shaped ingress interface instead of relying on generic HTTP for
  DuckLake's main capability story.
- `wit/knowledge-child/knowledge-child.wit:7` — import the new ingress
  interface so WIT itself carries Mother's truer language.
- `src/plugin/internal/knowledge_child.rs:68` — host implementation for
  that ingress interface.
- `plugins/ducklake/src/lib.rs:80` — consume granted ingress capability,
  not generic fetch.

### Commit 5: Make TaskIntent layering explicit

Primary files:

- `crates/patina-child-sdk/src/lib.rs`
- `crates/patina-toy-sdk/src/lib.rs`
- `src/mother/child.rs`
- `src/plugin/internal/knowledge_child.rs`

Desired result:

- `TaskIntent` no longer reads as an ambient catch-all.
- Docs and types say whether it is a toy or a substrate primitive.
- The SDK makes clear what is a granted domain capability and what is
  Mother/runtime substrate.

Concrete decision for this spec:

- `TaskIntent` is explicit Mother/runtime substrate.
- An ergonomic wrapper may remain in the SDK, but it is not a normal toy
  and must not appear in the granted-toy taxonomy.

Direct code targets:

- `src/mother/child.rs:130` — keep task intent kinds/types documented as
  Mother runtime substrate.
- `crates/patina-toy-sdk/src/lib.rs:3` — task intent types remain shared
  data structures, but docs/comments must mark them as substrate-backed.
- `crates/patina-child-sdk/src/lib.rs:92` — substrate re-exports remain
  the canonical child-facing path for task intents.
- `src/plugin/internal/knowledge_child.rs:341` — task enqueue host path
  remains Mother-controlled orchestration, not a child-owned general
  capability.

### Commit 6: Add doctrinal tests

Primary files:

- `src/plugin/internal/tests.rs`
- SDK tests in `crates/patina-child-sdk` and `crates/patina-toy-sdk`
- targeted tests around daemon runtime mode if needed

Desired result:

- denied toy not present in proof child bundle
- lower-level misuse still rejected
- default daemon path does not execute legacy shell toys
- DuckLake and belief-verifier still validate the Mother/Child/Toy split

Direct test targets:

- `src/plugin/internal/tests.rs` — manifest/WIT/runtime alignment tests
- new daemon tests for default legacy-off behavior
- new DuckLake-oriented tests that prove granted ingress + granted lake
  shape rather than ambient HTTP + lake-name strings

### Commit 7: Rewrite the runtime narrative

Primary files:

- `layer/surface/build/refactor/mother-doctrine-cleanup/SPEC.md`
- `layer/surface/build/refactor/mother-doctrine-cleanup/DESIGN.md`
- comments in `src/mother/child.rs`
- comments in `src/commands/mother/daemon.rs`

Desired result:

- no-context agents read the right doctrine from the tree
- interface work sees a stable backend story
- legacy language is clearly marked migration-only
- the docs make clear that component-model truth includes the child SDK,
  not only the host runtime
- the docs make clear that WIT is part of Mother's language and must
  align with manifests, runtime gates, and SDK shape

## Key Files

- `src/commands/mother/daemon.rs` — default daemon heartbeat still runs
  legacy toys; main quarantine point
- `src/commands/mother/registry.rs` — mixed legacy/knowledge child
  registry model
- `src/mother/child.rs` — doctrine-bearing runtime traits and legacy
  bridge types
- `src/plugin/internal/knowledge_child.rs` — current knowledge-child
  runtime authority enforcement
- `crates/patina-child-sdk/src/lib.rs` — universal `GuestHost` path that
  weakens granted-toy doctrine
- `crates/patina-toy-sdk/src/lib.rs` — child-facing toy surfaces that
  still accept too much free-form authority
- SDK-facing docs and type groupings must make the toy/substrate split
  legible even while the model is evolving
- `plugins/ducklake/plugin.toml` — current capability declaration that is
  broader than the older DuckLake grant/policy story
- `plugins/ducklake/src/lib.rs` — canonical child proof that should read
  as “child receives granted toys”
- `plugins/belief-verifier/src/lib.rs` — second proof child for event /
  task / belief flow
- `layer/surface/build/fix/knowledge-child-platform-audit-fixes/SPEC.md`
  — closest existing cleanup spec; this refactor folds its concerns into
  a broader Mother-first cleanup sequence

## Verification Plan

Minimum commands:

- `cargo test -p patina-ai knowledge_child -- --nocapture`
- `cargo test -p patina-child-sdk -p patina-toy-sdk`
- targeted `cargo test` coverage for daemon runtime mode and denied-toy
  behavior
- `patina spec check mother-doctrine-cleanup --json`

Manual review points:

- can a reader still think default Mother runtime includes shell-command
  toy spawning?
- do DuckLake and belief-verifier visibly receive toys rather than
  construct them ambiently?
- does the lake toy visibly encode granted scope in the child-facing API?
- does DuckLake still expose broad host power where a grant-shaped toy is
  the truer boundary?
- could an implementer still satisfy this work by leaving WIT ambient and
  only cleaning up Rust wrappers?
- does the SDK still flatten Mother substrate and granted capabilities
  into one indistinguishable authoring story?
- do comments/docs still imply two equal runtime stories?

## Build Readiness

This design should now be specific enough for autonomous implementation.

Architecture choices already resolved here:

- Mother / Child / Toy / Substrate taxonomy
- DuckLake ingress direction
- `TaskIntent` taxonomy
- WIT as source-of-truth language
- direct code targets for daemon, SDK, WIT, and DuckLake changes

What remains for implementation should be local coding and test design,
not further architecture discovery.

## Open Questions

- Should legacy `MotherChild` remain compiled in behind a feature flag,
  a daemon flag, or a separate command path?
- After lake scope-binding, do fetch/query toys also need scoped object
  forms now, or can they remain host-validated for this pass?
- For non-DuckLake cases, should generic HTTP remain only a lower-level
  substrate helper, or should future ingress capability shapes also move
  toward connector/policy grants as they emerge?
