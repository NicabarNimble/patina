# Watcher Child Bundle Contract

Slate: `watcher-child-bundle-contract`

## Intent

`patina-child-watcher-system` is a child-bundle repository: one external repository that owns a watcher ecosystem made of multiple independently installable Patina children plus shared WIT contracts, release workflow, docs, and future downstream examples.

This follows the Slate extraction pattern, adapted from a single app-like child to a bundle/ecosystem:

1. Define package/bundle contract before moving code.
2. Prove standalone external builds and local installs.
3. Teach Mother registry how to consume the release shape if needed.
4. Retire in-tree monorepo sources only after external proof succeeds.

## Vocabulary

- **Child-bundle repo**: an external repository that owns more than one related child package and their shared contracts/docs/release workflow.
- **Child package**: one independently buildable/installable child under the bundle repo, with its own `Cargo.toml`, `child.toml`, `src/`, and `wit/`.
- **Release unit**: the artifact set Mother can verify/install for one child: `.wasm`, `child.toml`, sidecar hashes/checksums, and optional package metadata.
- **Producer actor**: a long-lived/service-like child that observes or reacts and emits data/events.
- **Downstream sink**: a child that consumes producer output. A sink may be a reference/null sink, email sink, database sink, webhook sink, etc.
- **Shared WIT contract**: bundle-owned WIT that defines the producer/sink boundary, especially `patina:watch/events` and watch control/types.
- **Package extras**: optional skills, config schema/defaults, examples, and WIT contract documentation that travel with the child or bundle.

## Current Child Classification

### `folder-watch-actor`

Role: **core producer/service actor**.

Current source:

```text
children/folder-watch-actor/
  Cargo.toml
  child.toml
  src/
  wit/
```

Current traits:

- Uses `sdk/patina-sdk` via monorepo path dependency.
- Uses `wit-bindgen` and `export!(FolderWatchActor)`.
- Exposes typed WIT operations:
  - `patina:watch/control@0.1.0.configure`
  - `patina:watch/control@0.1.0.status`
  - `patina:watch/control@0.1.0.scan-now`
  - `patina:watch/control@0.1.0.reset`
- Declares toys:
  - `logging`
  - `keyvalue`
  - `messaging`
  - `measure`
  - `filesystem`
- Current manifest lacks explicit `[child].version` even though `Cargo.toml` is `0.1.0`.
- Current WIT imports more than manifest needs imply, including `http`, `connect`, `sql`, `events-stream`, `task`, `peer`, `git`, and runtime lifecycle shims. The standalone bundle should tighten or justify this surface.

Target meaning:

`folder-watch-actor` is the required core child of the watcher bundle. It watches/scans filesystem state and emits typed watch events over the shared watch contract.

### `watch-null-sink`

Role: **reference/null downstream sink**.

Current source:

```text
children/watch-null-sink/
  Cargo.toml
  child.toml
  src/
  wit/
```

Current traits:

- Uses `sdk/patina-sdk` via monorepo path dependency.
- Uses `wit-bindgen` and `export!(WatchNullSink)`.
- Exports `patina:watch/events@0.1.0`.
- Declares toys:
  - `logging`
  - `measure`
- Current manifest lacks explicit `[child].version` even though `Cargo.toml` is `0.1.0`.

Target meaning:

`watch-null-sink` is not core Patina behavior. It is the watcher bundle's reference downstream: a safe consumer that proves event routing and gives future sink authors a concrete implementation pattern.

Future downstream sinks belong naturally in the same watcher ecosystem unless they become substantial independent products. Examples:

- email sink
- database sink
- webhook sink
- queue sink
- lakehouse sink

## Target External Repository Layout

```text
/Users/nicabar/Projects/Patina/patina-child-watcher-system/
  Cargo.toml
  README.md
  RELEASING.md
  CHANGELOG.md
  .github/workflows/
    ci.yml
    release.yml

  children/
    folder-watch-actor/
      Cargo.toml
      child.toml
      src/
      wit/
      config.schema.toml      # optional/future
      default.config.toml     # optional/future

    watch-null-sink/
      Cargo.toml
      child.toml
      src/
      wit/
      config.schema.toml      # optional/future
      default.config.toml     # optional/future

  wit/
    patina-watch.wit          # optional shared source of truth once deduped

  skills/
    watcher-ops/
      SKILL.md                # optional, if watcher operation guidance becomes useful
```

The initial copy may keep per-child `wit/deps/patina-watch.wit` files to minimize movement risk. A later cleanup can dedupe the shared watch contract into top-level `wit/` if build tooling and child manifests remain clear.

## Manifest and SDK Rules

Each child package should be standalone-buildable outside the Patina monorepo.

Required changes from current in-tree source:

- Replace monorepo path dependency:

```toml
patina-sdk = { path = "../../sdk/patina-sdk" }
```

with a published SDK dependency, following Slate:

```toml
patina-sdk = "0.22.0"
```

or the current published SDK version chosen at extraction time.

- Add/align child manifest versions:

```toml
[child]
name = "folder-watch-actor"
version = "0.1.0"
kind = "child"
role = "app"
```

and:

```toml
[child]
name = "watch-null-sink"
version = "0.1.0"
kind = "child"
role = "app"
```

- Keep `child.toml` as package identity truth. Cargo version and child manifest version should match for the release unit.
- WIT imports should be either declared in `child.toml` needs or explicitly documented as runtime compatibility imports.

## Release Unit Strategy

Chosen v1 strategy: **per-child release tags** for the first extraction, with a follow-up Mother registry Slate for richer bundle manifests/asset selectors.

Reasoning:

- Existing Mother GitHub registry behavior is already proven for one child per release stream through Slate.
- Per-child tags can avoid blocking the standalone build on a new registry format.
- The bundle repo can still own both children and publish both; Mother can temporarily treat each child as a separate source/release stream.

Proposed tag shape:

```text
folder-watch-actor-v0.1.0
watch-null-sink-v0.1.0
```

Each release should include only one child release unit or unambiguous asset names:

```text
patina_ai_child_folder_watch_actor.wasm
patina_ai_child_folder_watch_actor.wasm.sha256
child.toml
child.toml.sha256
checksums.txt
```

and:

```text
patina_ai_child_watch_null_sink.wasm
patina_ai_child_watch_null_sink.wasm.sha256
child.toml
child.toml.sha256
checksums.txt
```

This keeps the first external proof close to Slate.

Future registry strategy is tracked separately in `mother-multi-child-bundle-registry`. That Slate can decide whether Mother should support:

- asset-name selectors,
- tag prefixes,
- a bundle manifest,
- or explicit multi-child release units from one GitHub release.

## Mother Registry Implication

For the first standalone build Slate, local install is enough:

```bash
patina child install /Users/nicabar/Projects/Patina/patina-child-watcher-system/children/folder-watch-actor \
  --wasm <folder-watch-actor.wasm> --force

patina child install /Users/nicabar/Projects/Patina/patina-child-watcher-system/children/watch-null-sink \
  --wasm <watch-null-sink.wasm> --force
```

Mother registry work remains a separate proof gate. Until `mother-multi-child-bundle-registry` is complete, we should not require a bundle registry format to prove external code movement.

## Non-goals for This Slate

This Slate does **not**:

- copy code into the external watcher repo,
- build or install the external watcher children,
- change Mother registry implementation,
- remove `children/folder-watch-actor` or `children/watch-null-sink` from Patina,
- perform broad Patina kernel extraction,
- decide all future sink designs.

## Proof Gate Summary

This contract is satisfied when:

- the bundle vocabulary is explicit,
- `folder-watch-actor` and `watch-null-sink` are classified,
- target repo layout is documented,
- v1 release-unit strategy is chosen,
- future Mother registry work is separated,
- and monorepo removal is explicitly deferred.
