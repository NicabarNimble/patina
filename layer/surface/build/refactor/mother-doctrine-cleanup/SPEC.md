---
type: refactor
id: mother-doctrine-cleanup
status: ready
created: 2026-03-12
sessions:
  origin: 20260312-001728
related:
- layer/surface/build/fix/knowledge-child-platform-audit-fixes/SPEC.md
- layer/surface/build/refactor/mother-maturation/SPEC.md
- layer/surface/build/refactor/agentic-surface-architecture/SPEC.md
beliefs:
- children-have-agency-toys-are-capabilities
- initialize-is-capability-grant
- connector-toy-is-indivisible-authority
exit_criteria:
- id: default-daemon-is-knowledge-child-only
  text: Default Mother daemon heartbeat and load path run only knowledge-child plugins; legacy mother-child requires an explicit migration mode
  checked: false
- id: sdk-exposes-granted-toys-not-universal-host
  text: The child SDK no longer teaches `GuestHost` plus universal toy construction as the primary authoring path; proof children receive explicit granted toy bundles
  checked: false
- id: toy-authority-is-bound-in-types
  text: At least lake-scoped toys bind granted authority in their object shape instead of accepting arbitrary logical names on every call
  checked: false
- id: taskintent-layering-is-explicit
  text: TaskIntent layering is explicit as either a dedicated toy or an intentionally documented runtime substrate, with no generic authority-backdoor semantics
  checked: false
- id: legacy-bridge-is-clearly-quarantined
  text: Legacy `MotherChild` plus shell-toy code remains only behind explicit migration seams and is no longer mixed into Mother's normal runtime narrative, docs, or defaults
  checked: false
- id: manifest-runtime-sdk-story-is-connected
  text: Manifest declaration, runtime grant, SDK exposure, and denied-toy tests form one connected documented story rather than separate enforcement fragments
  checked: false
- id: proof-children-still-prove-doctrine
  text: DuckLake and belief-verifier still pass verification while demonstrating Mother authority, child agency, and toy-granted capability boundaries
  checked: false
- id: interface-work-builds-on-clean-mother
  text: The resulting Mother surface is clean enough that interface work can treat Mother as stable backend authority rather than carrying legacy runtime ambiguity forward
  checked: false
---
# refactor: Mother Doctrine Cleanup — Legacy Quarantine Before Interface Work

> Clean legacy Mother/child paths so the runtime matches the 2026-03-11 knowledge-child doctrine before more interface work lands.

## Current State

The 2026-03-11 `knowledge-child-platform` build established the correct
target doctrine, but the working tree still carries mixed runtime
shapes:

- Mother now owns runtime continuity and authority through
  `src/mother/state.rs`, host APIs, and knowledge-child heartbeat
  execution.
- Children now have a real knowledge-child lane with typed host APIs and
  proof plugins.
- But the default daemon path still ticks legacy `MotherChild` plugins
  and spawns shell-command toys in `src/commands/mother/daemon.rs`.
- The SDK still teaches a universal `GuestHost` construction path in
  `crates/patina-child-sdk/src/lib.rs`, which weakens the “Mother grants
  toys, child receives toys” model.
- The current child-facing SDK shape is less truthful than the earlier
  DuckLake grant design: runtime checks exist, but the imported
  capability story is too ambient to feel like a component-model-native
  contract.
- Toy scope is still too free-form in `crates/patina-toy-sdk/src/lib.rs`
  and host bindings, especially lake authority.

This leaves Mother conceptually correct but operationally ambiguous.
Before interface setup/list/install work deepens the backend contract,
Mother's own runtime doctrine should be made explicit and enforced.

### Concrete doctrine violations in-tree

1. **Default daemon path still runs legacy shell-toy behavior.**
   `src/commands/mother/daemon.rs` runs knowledge-child cycles and then
   immediately runs `tick_legacy_all()` plus `spawn_toy_tracked()`.
   That keeps the old shell-command toy model in the default Mother
   runtime.

2. **Registry still models legacy and target runtimes as peer lanes.**
   `src/commands/mother/registry.rs` stores both `RegisteredChild::Legacy`
   and `RegisteredChild::Knowledge` and exposes both as normal runtime
   behavior.

3. **SDK still teaches ambient toy construction.**
   `crates/patina-child-sdk/src/lib.rs` exposes `GuestHost` methods like
   `GuestHost::fetch()`, `GuestHost::lake()`, and `GuestHost::belief()`.
   This lets child code construct every toy directly instead of
   receiving granted toy bundles.

4. **Proof children still encode the ambient-host pattern.**
   `plugins/ducklake/src/lib.rs` and `plugins/belief-verifier/src/lib.rs`
   both define toy bundles parameterized by `GuestHost`, which teaches a
   truthful runtime with a misleading child-facing API.

5. **Toy authority is still too free-form at the child boundary.**
   `crates/patina-toy-sdk/src/lib.rs` exposes `LakeToy` methods that take
   the lake name on every call. This does not bind granted lake scope
   into the object the child receives.

6. **Legacy shell-toy runtime still exists as active code, not merely as
   archival compatibility.** `src/mother/child.rs` still defines the
   legacy `Toy { command, args }` model and `src/plugin/internal/task.rs`
   still filters shell-command toys through `allowed_toy_commands`.

7. **DuckLake's current host capability shape is broader than the older
   typed-grant design.** `plugins/ducklake/plugin.toml` grants
   `host_http = true` and the child uses generic `FetchToy` and generic
   `LakeToy` access. That is simpler to ship, but less precise than the
   earlier connector-grant model where policy and authority traveled
   together.

## Target State

Mother is once again easy to describe and easy to trust:

- Mother holds authority and continuity.
- Children hold agency and workflow ownership.
- Toys are granted capabilities, not ambient host bags.
- Toys remain coarse, app-like granted bundles rather than being split
  into tiny permission atoms.
- Child imports/SDK surfaces are component-model-native: what a child can
  use is visible in the granted surface it receives, not merely rejected
  later at host-call time.
- Legacy mother-child and shell-toy paths are visible migration residue,
  not part of the normal Mother story.

### End-state plan

After this spec lands, the intended child platform story is:

- Mother owns the authority boundary and decides what a child receives.
- The `knowledge-child` world remains the runtime foundation, and in this
  platform a child is a WASM component. The SDK no longer teaches a
  universal ambient host.
- Third-party child authors start from granted bundles and explicit
  imports, not from `GuestHost` plus constructors for every toy.
- DuckLake becomes the canonical example of the corrected model.
- Toy APIs remain ergonomic and coarse, but their shapes reflect granted
  authority where it matters most.
- The toy-facing SDK area is intentionally still evolving and may split
  into its own dedicated SDK surface once the toy model stabilizes.

After this refactor, interface work should be able to treat Mother as a
stable backend authority surface without inheriting old dual-runtime
confusion.

### Non-goals

- Do not redesign Mother project registry, interface inventory, or
  install/update UX in this spec. Those should build on the cleaned
  runtime after this work lands.
- Do not remove the legacy runtime if doing so would destroy migration
  information. Quarantine is sufficient; invisibility in the default
  path is the requirement.
- Do not redesign every toy in one pass. Lake-scoped authority is the
  minimum required proof point; other toys can remain runtime-validated
  unless they block the doctrine.
- Do not rewrite the component-model foundation. This spec corrects the
  SDK/import story on top of the existing `knowledge-child` world rather
  than abandoning WIT/components.
- Do not atomize toys into overly fine-grained permission wrappers.
  Toys should stay ergonomic and app-like so children still feel like
  small programs using granted tools.
- Do not change the core doctrinal beliefs. This spec enforces them in
  code and documentation.

### Why this must happen before interface work

Upcoming interface work wants Mother to identify projects, track
installed interface bundles, and serve setup/list/install flows. If that
backend is still teaching two child runtimes and two toy models at once,
interface work will encode transitional behavior into a longer-lived
surface. This spec makes Mother a stable backend authority first.

## Steps

### 1. Quarantine legacy runtime paths

- Remove legacy `tick_legacy_all()` behavior from the default daemon
  heartbeat.
- Require an explicit migration flag, loader, or command path for
  `MotherChild` plugins.
- Make code, tests, and docs treat `MotherChild` as migration-only.

Implementation anchor files:

- `src/commands/mother/daemon.rs`
- `src/commands/mother/registry.rs`
- `src/mother/child.rs`
- `src/plugin/internal/mod.rs`

### 2. Make granted toys the primary child authoring surface

- Replace universal `GuestHost`-plus-constructor guidance with explicit
  granted toy bundles for proof children.
- Keep runtime denial checks, but make toy absence part of the authority
  story.
- Make the SDK read like a component-model contract: Mother grants the
  child a surface, and the child codes against that granted surface.
- Ensure the SDK is suitable for third-party children by making the
  truthful path also the ergonomic path.

Implementation anchor files:

- `crates/patina-child-sdk/src/lib.rs`
- `crates/patina-toy-sdk/src/lib.rs`
- `plugins/ducklake/src/lib.rs`
- `plugins/belief-verifier/src/lib.rs`

### 3. Bind authority into toy shapes

- Start with lake authority.
- A child should receive granted lakes as concrete toy objects or scoped
  toy bundles, not generic access methods parameterized by arbitrary lake
  names.
- Preserve coarse toy ergonomics while doing this. The goal is truthful
  granted authority, not decomposing toys into tiny capability atoms.
- Use DuckLake to determine the initial toy taxonomy: which capabilities
  deserve first-class toys, which remain runtime substrate, and which
  need grant-shaped redesign.
- Revisit broad host capabilities that weaken the story, especially
  DuckLake's current `host_http = true` shape. If the earlier
  connector-policy model is still the right mental model, the child API
  should reflect that rather than exposing ambient fetch.

Implementation anchor files:

- `crates/patina-toy-sdk/src/lib.rs`
- `src/plugin/internal/knowledge_child.rs`
- `src/mother/lake_host.rs`
- `plugins/ducklake/src/lib.rs`

### 4. Make TaskIntent layering explicit

- Decide and document whether `TaskIntent` is a dedicated toy or an
  intentionally exposed Mother runtime substrate.
- Do not allow it to remain an ambiguous catch-all authority path.

Implementation anchor files:

- `crates/patina-child-sdk/src/lib.rs`
- `crates/patina-toy-sdk/src/lib.rs`
- `src/mother/child.rs`
- `src/plugin/internal/knowledge_child.rs`

### 5. Tighten proof and documentation

- Preserve DuckLake and belief-verifier as canonical doctrine proofs.
- Verify child-owned workflow, Mother-owned state/safety, and denied-toy
  behavior together.
- Make manifest -> runtime grant -> SDK bundle -> denied-toy behavior
  read as one continuous story in docs and tests.
- Ensure DuckLake's end-state shape is explicitly captured in docs so a
  future agent does not “simplify” back toward ambient host access.
- Update Mother-facing docs/specs so interface work reads the cleaned
  model rather than the transitional one.

Implementation anchor files:

- `src/plugin/internal/tests.rs`
- `layer/surface/build/refactor/mother-doctrine-cleanup/SPEC.md`
- `layer/surface/build/refactor/mother-doctrine-cleanup/DESIGN.md`
- comments in `src/mother/child.rs` and `src/commands/mother/daemon.rs`

## Suggested Commit Sequence

1. `fix(mother): quarantine legacy runtime from default daemon path`
2. `fix(sdk): make granted toy bundles the primary child surface`
3. `fix(toys): bind lake authority into granted toy objects`
4. `fix(runtime): narrow broad host capability surfaces where needed`
5. `fix(runtime): make TaskIntent layering explicit`
6. `test(mother): prove denied-toy and proof-child doctrine behavior`
7. `docs(mother): align runtime narrative with knowledge-child doctrine`

## Agent Guidance

Any agent implementing this spec should read these first:

1. `git show spec/knowledge-child-platform:layer/surface/build/feat/knowledge-child-platform/SPEC.md`
2. `git show spec/knowledge-child-platform:layer/surface/build/feat/knowledge-child-platform/DESIGN.md`
3. `layer/surface/build/fix/knowledge-child-platform-audit-fixes/SPEC.md`
4. `src/commands/mother/daemon.rs`
5. `src/commands/mother/registry.rs`
6. `src/mother/child.rs`
7. `crates/patina-child-sdk/src/lib.rs`
8. `crates/patina-toy-sdk/src/lib.rs`
9. `plugins/ducklake/src/lib.rs`
10. `plugins/belief-verifier/src/lib.rs`
11. `plugins/ducklake/plugin.toml`

The agent should preserve the doctrine even if local APIs change:

- Mother grants and persists authority
- children own workflow and business policy
- toys are bounded, coarse-grained capabilities, not ambient host access
- the SDK must teach the same capability story the runtime enforces
- component-model imports should be explicit enough that third-party
  child authors can understand what is actually granted
- children are WASM components in this platform even though toys may be
  local, native, remote, virtual, or federated in implementation
- legacy paths are opt-in migration seams only

## Exit Criteria

1. Default Mother daemon heartbeat and load path run only
   knowledge-child plugins; legacy mother-child requires an explicit
   migration mode.
2. The child SDK no longer teaches `GuestHost` plus universal toy
   construction as the primary authoring path; proof children receive
   explicit granted toy bundles.
3. The child SDK is suitable for third-party child authors because the
   truthful granted-capability path is also the primary ergonomic path.
4. At least lake-scoped toys bind granted authority in their object
   shape instead of accepting arbitrary logical names on every call.
5. Broad host capability surfaces that weaken the authority story
   (especially DuckLake's current ambient HTTP shape) are narrowed or
   explicitly re-expressed through a grant-shaped toy/interface.
6. `TaskIntent` layering is explicit as either a dedicated toy or an
   intentionally documented runtime substrate, with no generic
   authority-backdoor semantics.
7. Legacy `MotherChild` plus shell-toy code remains only behind
   explicit migration seams and is no longer mixed into Mother's normal
   runtime narrative, docs, or defaults.
8. Manifest declaration, runtime grant, SDK exposure, and denied-toy
   tests form one connected documented story rather than separate
   enforcement fragments.
9. DuckLake and belief-verifier still pass verification while
   demonstrating Mother authority, child agency, and toy-granted
   capability boundaries.
10. The resulting Mother surface is clean enough that interface work can
   treat Mother as stable backend authority rather than carrying legacy
   runtime ambiguity forward.

## Verification

Run at minimum:

- `cargo test -p patina-ai knowledge_child -- --nocapture`
- `cargo test -p patina-child-sdk -p patina-toy-sdk`
- targeted tests covering legacy daemon quarantine, denied-toy absence,
  and proof-child behavior
- `patina spec check mother-doctrine-cleanup --json`

Review manually:

- default Mother daemon path does not run legacy shell-toy execution
- proof children no longer teach universal `GuestHost` as the primary
  authoring surface
- third-party child authors would learn the truthful granted-capability
  story from the SDK itself
- lake authority is visibly bound into the child-facing toy shape
- DuckLake no longer relies on an ambient capability story where a more
  precise grant-shaped interface is intended
- `TaskIntent` is clearly bounded and no longer reads like an ambient
  escape hatch
