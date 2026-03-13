---
type: refactor
id: mother-doctrine-cleanup
status: complete
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
  checked: true
- id: sdk-exposes-granted-toys-not-universal-host
  text: The child SDK no longer teaches `GuestHost` plus universal toy construction as the primary authoring path; proof children receive explicit granted toy bundles
  checked: true
- id: toy-authority-is-bound-in-types
  text: At least lake-scoped toys bind granted authority in their object shape instead of accepting arbitrary logical names on every call
  checked: true
- id: taskintent-layering-is-explicit
  text: TaskIntent layering is explicit as either a dedicated toy or an intentionally documented runtime substrate, with no generic authority-backdoor semantics
  checked: true
- id: legacy-bridge-is-clearly-quarantined
  text: Legacy `MotherChild` plus shell-toy code remains only behind explicit migration seams and is no longer mixed into Mother's normal runtime narrative, docs, or defaults
  checked: true
- id: manifest-runtime-sdk-story-is-connected
  text: Manifest declaration, runtime grant, WIT/import shape, SDK exposure, and denied-toy tests form one connected documented story rather than separate enforcement fragments
  checked: true
- id: proof-children-still-prove-doctrine
  text: DuckLake and belief-verifier still pass verification while demonstrating Mother authority, child agency, and toy-granted capability boundaries
  checked: true
- id: interface-work-builds-on-clean-mother
  text: The resulting Mother surface is clean enough that interface work can treat Mother as stable backend authority rather than carrying legacy runtime ambiguity forward
  checked: true
---
# refactor: Mother Doctrine Cleanup — Legacy Quarantine Before Interface Work

> Clean legacy Mother/child paths so the runtime matches the 2026-03-11
> knowledge-child doctrine before more interface work lands.

## Problem

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

## Goal

Ship a doctrine cleanup that makes Mother easy to describe, easy to
trust, and safe to build on.

The build goal is not “remove capabilities” or “polish wrappers.” The
goal is to make Mother's authority language, child runtime, WIT surface,
SDK, proof children, and migration boundaries all tell one coherent
story.

**Target shape:**

- Mother remains the authority, continuity, and runtime substrate owner.
- children remain WASM components with agency over workflow and policy.
- toys remain granted capability bundles, not ambient host bags.
- WIT remains Mother's language to children.
- DuckLake becomes the canonical proof for granted ingress + granted
  lake capability shape.
- `TaskIntent` remains Mother/runtime substrate, not a normal domain toy.
- legacy `mother-child` and shell-toy paths survive only as explicit
  migration seams.

## Status

Implementation completed on 2026-03-12 and the exit criteria now pass in-tree.

Already true in-tree:

- `knowledge-child` exists as the active WASM child world.
- Mother owns runtime state, tasks, subscriptions, checkpoints, and host
  APIs.
- DuckLake and belief-verifier exist as proof children.
- the spec now locks in Mother / Child / Toy / Substrate taxonomy.

Resolved during implementation:

- default daemon path now quarantines legacy runtime behind explicit
  migration mode.
- DuckLake ingress now lands on a granted source capability instead of
  ambient `host_http`.
- lake authority is now bound in the granted toy shape.
- WIT, runtime, SDK, and proof children now tell one connected grant
  story.
- `TaskIntent` is now explicitly documented and exposed as runtime
  substrate machinery.

## Non-Goals

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
- Do not satisfy the spec by removing a capability DuckLake genuinely
  needs just because the current capability shape is too broad. Needed
  capabilities must be re-expressed truthfully, not deleted for
  convenience.
- Do not collapse toys, primitives, substrate, and derived enforcement
  into one bucket. The taxonomy above is part of the architecture being
  preserved.
- Do not atomize toys into overly fine-grained permission wrappers.
  Toys should stay ergonomic and app-like so children still feel like
  small programs using granted tools.
- Do not change the core doctrinal beliefs. This spec enforces them in
  code and documentation.

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
   still need to stop teaching `GuestHost` or any equivalent ambient host
   bag as the primary authoring path, even if compatibility internals
   remain temporarily.

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
- WIT is part of Mother's language to children. The component-model
  import surface, runtime grants, manifest capabilities, and SDK
  ergonomics must agree on the same authority story.
- Legacy mother-child and shell-toy paths are visible migration residue,
  not part of the normal Mother story.

### Taxonomy to preserve

- **Mother** owns authority, continuity, orchestration, and runtime
  substrate. Mother grants capabilities and owns the long-lived
  operational plane.
- **Child** is a WASM component in this platform. The child owns
  workflow, policy, and decisions within the authority Mother granted.
- **Toy** is a granted capability surface the child uses to do bounded
  work. Toys are not defined by local/native vs remote/federated
  implementation.
- **Primitive / substrate** is lower-level mechanism such as pipe,
  leasing, runtime state, checkpoints, and scheduling. These are not
  toys by default, though toys may be built on top of them.
- **Derived enforcement** is policy or enforcement derived from a grant
  (for example proxy/auth injection from a connector grant). Derived
  enforcement is not a peer toy unless it becomes a real standalone
  capability.

This taxonomy is part of the intended end state. A change that makes the
code pass while blurring these categories does not satisfy the spec.

### End-state plan

After this spec lands, the intended child platform story is:

- Mother owns the authority boundary and decides what a child receives.
- The `knowledge-child` world remains the runtime foundation, and in this
  platform a child is a WASM component. The SDK no longer teaches a
  universal ambient host.
- WIT remains the source-of-truth interface language for children. This
  spec is not complete if the Rust SDK improves while WIT/imports still
  tell a broader or more ambient authority story.
- Third-party child authors start from granted bundles and explicit
  imports, not from `GuestHost` plus constructors for every toy.
- DuckLake becomes the canonical example of the corrected model.
- Toy APIs remain ergonomic and coarse, but their shapes reflect granted
  authority where it matters most.
- The toy-facing SDK area is intentionally still evolving and may split
  into its own dedicated SDK surface once the toy model stabilizes.
- The SDK must visibly distinguish granted toys from Mother/runtime
  substrate so child authors can tell what is a capability vs what is
  operational machinery.

### Implementation commitments for this spec

These are not optional interpretations. They are the concrete build
targets for this spec.

1. **Default Mother daemon runs knowledge children only.**
   - Remove legacy ticking from the default heartbeat at
     `src/commands/mother/daemon.rs:120`.
   - Add an explicit legacy opt-in on Mother start options in
     `src/commands/mother/mod.rs:62` and `src/commands/mother/daemon.rs:538`.
   - Keep legacy loader code only behind that explicit mode in
     `src/commands/mother/daemon.rs:563`.

2. **DuckLake ingress is re-expressed as a granted source/connector
   capability, not ambient HTTP.**
   - DuckLake stops using broad `host_http` + generic fetch as its primary
     granted story from `plugins/ducklake/plugin.toml:9`.
   - Add a dedicated granted ingress interface to WIT in
     `wit/knowledge-child/deps/patina-host/host.wit:38` and import it from
     `wit/knowledge-child/knowledge-child.wit:7`.
   - DuckLake uses that grant-shaped ingress surface from
     `plugins/ducklake/src/lib.rs:66` instead of generic fetch.

3. **TaskIntent is explicit Mother/runtime substrate.**
   - Keep task intents out of the normal granted-toy taxonomy.
   - Document and expose them as substrate-backed machinery in
     `crates/patina-child-sdk/src/lib.rs:92`,
     `crates/patina-toy-sdk/src/lib.rs:3`, and
     `src/mother/child.rs:130`.
   - If an ergonomic wrapper remains, it must still read as substrate,
     not as a peer domain toy.

4. **Lake is the first fully grant-shaped toy proof.**
   - Replace the free-form lake-name-per-call shape from
     `crates/patina-toy-sdk/src/lib.rs:251` with a granted/scoped child
     object shape.
   - Update host/runtime glue in `src/plugin/internal/knowledge_child.rs:205`
     and DuckLake usage in `plugins/ducklake/src/lib.rs:102`.

5. **SDK taxonomy must be legible in code.**
   - `crates/patina-child-sdk/src/lib.rs:22` remains the granted-toy area.
   - `crates/patina-child-sdk/src/lib.rs:92` remains the substrate area.
   - Future child authors should be able to tell, from the SDK alone,
     which APIs are granted capabilities and which are Mother/runtime
     operational machinery.

After this refactor, interface work should be able to treat Mother as a
stable backend authority surface without inheriting old dual-runtime
confusion.

### Why this must happen before interface work

Upcoming interface work wants Mother to identify projects, track
installed interface bundles, and serve setup/list/install flows. If that
backend is still teaching two child runtimes and two toy models at once,
interface work will encode transitional behavior into a longer-lived
surface. This spec makes Mother a stable backend authority first.

## Solution

### 1. Quarantine legacy runtime from the default Mother path

Default Mother should only run knowledge children and Mother-owned
substrate. Legacy `mother-child` loading/ticking must require explicit
opt-in.

This preserves migration knowledge without teaching two runtime stories
at once.

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
- The SDK is a convenience layer over Mother's WIT language, not an
  alternate authority model. If the SDK and WIT disagree, the spec is
  not satisfied.

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
- Do not resolve this step by deleting DuckLake ingress if DuckLake still
  needs networked capture. Resolve it by giving DuckLake a truer granted
  interface.

Implementation anchor files:

- `crates/patina-toy-sdk/src/lib.rs`
- `src/plugin/internal/knowledge_child.rs`
- `src/mother/lake_host.rs`
- `plugins/ducklake/src/lib.rs`
- `plugins/ducklake/plugin.toml`

### 4. Make TaskIntent layering explicit

- Decide and document whether `TaskIntent` is a dedicated toy or an
  intentionally exposed Mother runtime substrate.
- Do not allow it to remain an ambiguous catch-all authority path.
- Whatever shape is chosen, the SDK and docs must make clear that
  scheduling/leasing continuity belongs to Mother, not to a child-held
  general-purpose capability bag.

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
- Include WIT/import shape in that continuous story; wrapper-only fixes
  are insufficient.
- Ensure DuckLake's end-state shape is explicitly captured in docs so a
  future agent does not “simplify” back toward ambient host access.
- Update Mother-facing docs/specs so interface work reads the cleaned
  model rather than the transitional one.

Implementation anchor files:

- `src/plugin/internal/tests.rs`
- `layer/surface/build/refactor/mother-doctrine-cleanup/SPEC.md`
- `layer/surface/build/refactor/mother-doctrine-cleanup/DESIGN.md`
- comments in `src/mother/child.rs` and `src/commands/mother/daemon.rs`

## Implementation Order

1. `fix(mother): quarantine legacy runtime from default daemon path`
2. `fix(sdk): make granted toy bundles the primary child surface`
3. `fix(toys): bind lake authority into granted toy objects`
4. `fix(runtime): narrow broad host capability surfaces where needed`
5. `fix(runtime): make TaskIntent layering explicit`
6. `test(mother): prove denied-toy and proof-child doctrine behavior`
7. `docs(mother): align runtime narrative with knowledge-child doctrine`

## Resolved Decisions

These decisions are resolved for this spec and should not be reopened
during implementation:

- child = WASM component in this platform
- Mother = authority + continuity + orchestration + runtime substrate
- toy = granted capability surface for bounded work
- primitive/substrate != toy by default
- derived enforcement != peer toy by default
- DuckLake ingress ends on a granted source/connector capability, not
  ambient `host_http`
- `TaskIntent` is Mother/runtime substrate, even if an ergonomic wrapper
  remains in SDK code
- WIT is part of Mother's language and must change where the authority
  story changes
- legacy runtime is migration-only, not equal target architecture

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
- Mother owns runtime substrate and continuity; children do not
  self-authorize or self-orchestrate outside granted boundaries
- children own workflow and business policy
- toys are bounded, coarse-grained capabilities, not ambient host access
- primitives/substrate are not toys by default
- derived enforcement is not a peer toy unless it becomes a real
  standalone capability
- the SDK must teach the same capability story the runtime enforces
- WIT, manifests, runtime grants, SDK bundles, and proof-child code must
  teach the same capability story
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
6. WIT/import shape is updated wherever needed so Mother's language to
   children matches the narrowed/granted capability story; SDK-only
   cleanup does not satisfy this criterion.
7. `TaskIntent` layering is explicit as either a dedicated toy or an
   intentionally documented runtime substrate, with no generic
   authority-backdoor semantics.
8. Legacy `MotherChild` plus shell-toy code remains only behind
   explicit migration seams and is no longer mixed into Mother's normal
   runtime narrative, docs, or defaults.
9. Manifest declaration, runtime grant, WIT/import shape, SDK exposure,
   and denied-toy
   tests form one connected documented story rather than separate
   enforcement fragments.
10. DuckLake and belief-verifier still pass verification while
   demonstrating Mother authority, child agency, and toy-granted
   capability boundaries.
11. The resulting Mother surface is clean enough that interface work can
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
- WIT/import definitions now express the same granted-capability story as
  the Rust SDK and runtime gates
- lake authority is visibly bound into the child-facing toy shape
- DuckLake no longer relies on an ambient capability story where a more
  precise grant-shaped interface is intended
- `TaskIntent` is clearly bounded and no longer reads like an ambient
  escape hatch

## Remaining Open Design Decision

This spec is ready to build, but one design choice remains intentionally
open and must be resolved during implementation rather than bypassed:

- For DuckLake ingress, should the final granted interface remain generic
  HTTP with tighter shape, or return to a connector/policy-shaped
  capability so authority and source policy travel together?

This decision must be made in a way that preserves DuckLake's real needs,
keeps WIT truthful, and improves the third-party child authoring story.

## Build Readiness

This spec is intended to be implementation-ready enough for a no-context
agent to build autonomously.

Already resolved here:

- doctrine and taxonomy
- Mother's role
- child = WASM contract
- WIT as Mother's language
- DuckLake as the proof child for ingress and lake shape
- `TaskIntent` as substrate
- direct code targets for the primary changes

Remaining implementation choices should now be local code-shape choices,
not architecture decisions. If an implementer feels forced to reopen the
architecture, the spec should be updated before more code is written.
