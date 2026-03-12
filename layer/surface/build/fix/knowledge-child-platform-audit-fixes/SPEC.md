---
type: fix
id: knowledge-child-platform-audit-fixes
status: draft
created: 2026-03-11
sessions:
  origin: 20260310-230611
related:
- knowledge-child-platform
beliefs:
- children-have-agency-toys-are-capabilities
- protocol-boundaries-must-be-typed
- reads-via-host-writes-via-intents
- bridges-become-permanent
exit_criteria:
- toy grants are first-class authority — ungranted toys cannot be constructed or used from the SDK, not merely rejected inside host calls
- knowledge children receive explicit granted toy bundles from the SDK/runtime instead of building toys from a universal GuestHost
- TaskIntent layering is explicit — either a dedicated toy or an explicitly documented runtime substrate, with no generic backdoor semantics
- legacy mother-child runtime is quarantined behind an explicit migration switch or separate loader and is no longer part of the default daemon path
- toys bind granted scope in their types where practical (especially lake; fetch/query if feasible) rather than accepting fully free-form authority on every call
- DuckLake semantic tests prove per-type cursor behavior, partial success, auth escalation, and child-owned workflow through toys alone
- manifest -> runtime grant -> SDK bundle -> denied-toy test coverage forms one connected story in docs and tests
- all existing knowledge-child verification still passes after the architecture hardening
---
# fix: Knowledge Child Platform Audit Fixes — Toy Grants, Bundle Injection, Legacy Quarantine

> Fix the post-ship architecture gaps in the knowledge-child platform so the Mother / Child / Toy model is enforced in types and runtime shape, not merely described in docs and examples.

## Problem

The initial `knowledge-child-platform` build shipped the new world,
runtime store, host APIs, SDK crates, and two proof children. It is
working, but a post-ship audit found that the architecture is not yet as
strict as the spec intends:

1. **Toy grants are advisory in the SDK shape.** Runtime authority is
   enforced mainly at host-call time. A child can still construct all toy
   wrappers from a universal `GuestHost`, even when the manifest did not
   grant that toy.

2. **Children do not receive granted toy bundles.** The SDK currently
   exposes a universal construction path instead of a runtime-granted
   `DuckLakeToys` / `BeliefVerifierToys`-style bundle.

3. **Legacy `mother-child` is still live in the daemon heartbeat path.**
   The new model is supposed to be the target system, but the daemon
   still treats legacy and knowledge-child as two active runtime lanes.

4. **Toy scope is still too free-form.** `LakeToy` in particular still
   accepts arbitrary names/identifiers per call instead of expressing the
   granted scope in the toy object itself.

5. **DuckLake proof is mostly platform-mechanical, not semantic.**
   Important learned behavior such as per-type cursor handling and partial
   success policy is not yet explicitly locked down in tests.

6. **`TaskIntent` layering is underspecified.** It currently acts both as
   a child-facing tool and a Mother runtime primitive. That is workable,
   but not yet clearly bounded.

The result is a platform that works, but still leaves room for
architectural drift back toward "generic host bag + dual runtimes."

## Root Cause

The first spec optimized for shipping the new platform end to end:

- get the new world in tree
- make Mother own state and tasks
- prove two real WASM children
- isolate the old shell-toy path

That was the right first milestone, but it left the child-facing toy
model one step short of full enforcement. The runtime boundary is typed;
the child authoring surface is still too universal.

## Fix

### Commit 1: `fix(sdk): inject granted toy bundles instead of universal GuestHost`

**Files:** `crates/patina-child-sdk/src/lib.rs`, `crates/patina-toy-sdk/src/lib.rs`, proof children

- Replace the universal child construction pattern with explicit granted
  toy bundle injection.
- Child code should receive a typed granted bundle, not call static
  constructors for every possible toy.
- Preserve the Mother / Child / Toy model directly in the SDK types.

### Commit 2: `fix(runtime): enforce toy grants as first-class authority`

**Files:** `src/plugin/internal/knowledge_child.rs`, `src/plugin/internal/mod.rs`, SDK/runtime glue

- Enforce toy grants before construction/exposure, not only at host-call
  time.
- If `fetch`, `query`, `measure`, `graph`, `belief`, or specific lake
  scope is not granted, the child should not receive that toy.
- Add denied-toy tests that prove absence and rejection are aligned.

### Commit 3: `fix(runtime): make TaskIntent layering explicit`

**Files:** `crates/patina-child-sdk/src/lib.rs`, `crates/patina-toy-sdk/src/lib.rs`, spec/design docs

- Decide whether tasks are:
  - a dedicated `TaskToy`, or
  - explicit runtime substrate exposed intentionally to children.
- Remove ambiguity so `TaskIntent` does not become a generic escape
  hatch.

### Commit 4: `fix(daemon): quarantine legacy mother-child path`

**Files:** `src/commands/mother/daemon.rs`, `src/commands/mother/registry.rs`, loader config/docs

- Move legacy `mother-child` behind an explicit migration switch,
  separate loader, or equivalent quarantine.
- Default daemon path should be the knowledge-child runtime only.
- Legacy remains available only as an intentional migration aid.

### Commit 5: `fix(toys): bind granted scope into toy objects`

**Files:** `crates/patina-toy-sdk/src/lib.rs`, `crates/patina-child-sdk/src/lib.rs`, host/runtime glue

- Bind granted scope into toy objects where practical.
- Highest priority: `LakeToy` should represent granted lakes/surfaces,
  not accept unconstrained authority every call.
- Apply the same approach to fetch/query if it materially improves the
  child-facing authority story without overcomplicating the API.

### Commit 6: `test(ducklake): lock semantic behavior into proof tests`

**Files:** plugin/runtime tests, `plugins/ducklake`, Mother runtime tests

- Add semantic tests for:
  - per-type cursor behavior
  - partial success
  - auth escalation
  - Mother not deciding workflow order
  - child operating end to end through granted toys alone

### Commit 7: `docs(test): unify manifest -> runtime -> SDK story`

**Files:** spec/design docs, manifest parsing tests, SDK docs/tests

- Make toy enforcement read as one continuous story:
  - manifest declares toys
  - runtime grants toys
  - SDK exposes only granted toys
  - tests prove denied toys stay denied

## Exit Criteria

1. Toy grants are first-class authority — ungranted toys cannot be constructed or used from the SDK, not merely rejected inside host calls
2. Knowledge children receive explicit granted toy bundles from the SDK/runtime instead of building toys from a universal `GuestHost`
3. `TaskIntent` layering is explicit — either a dedicated toy or an explicitly documented runtime substrate, with no generic backdoor semantics
4. Legacy `mother-child` runtime is quarantined behind an explicit migration switch or separate loader and is no longer part of the default daemon path
5. Toys bind granted scope in their types where practical (especially lake; fetch/query if feasible) rather than accepting fully free-form authority on every call
6. DuckLake semantic tests prove per-type cursor behavior, partial success, auth escalation, and child-owned workflow through toys alone
7. Manifest -> runtime grant -> SDK bundle -> denied-toy test coverage forms one connected story in docs and tests
8. All existing knowledge-child verification still passes after the architecture hardening
