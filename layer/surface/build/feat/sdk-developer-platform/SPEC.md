---
type: feat
id: sdk-developer-platform
status: draft
created: 2026-04-10
sessions:
  origin: 20260409-143847-707078000
blocked_by: []
related:
- feat/voice-lake-mvp1
- feat/patina-sdk-rebuild
- fix/sdk-public-surface-alignment
beliefs:
- '[[sdk-is-mct-entry-point]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[compiler-enforced-safety]]'
- '[[pandos-are-shareable-compositions]]'
exit_criteria:
- id: sdp1-child-discovery
  text: "patina child list shows all installed children with their interface shape: name, kind, WIT exports, WIT imports (toys), and stage role (source/extract/transform/write/catalog). A developer can see what's available and what each child does."
  checked: false
- id: sdp2-pando-inspect
  text: "patina pando show <name> displays the full composition graph: children, adapters, wiring, entry point, and config keys. A developer can read the data flow without opening pando.toml."
  checked: false
- id: sdp3-pando-validate
  text: "patina pando validate <path-to-pando.toml> checks composition wiring against installed children's WIT interfaces. Reports mismatches (missing children, wrong toy imports, incompatible stages) before Mother tries to load."
  checked: false
- id: sdp4-pando-scaffold
  text: "patina pando new <name> scaffolds a pando.toml with config section, wiring skeleton, and comments explaining the adapter pattern. Developer fills in children and wiring."
  checked: false
- id: sdp5-config-documented
  text: "SDK docs explain pando config injection: [config] section in pando.toml, how Mother passes config to children at composition time, and how to use toys::config::get(key) in child code."
  checked: false
- id: sdp6-voice-lake-proof
  text: "A developer can assemble voice-lake-mvp1 using SDK tools: discover the 6 pipeline children, inspect folder-text-to-parquet pando, scaffold a voice-scoped variant with [config].voice_id, validate it, and run it through Mother."
  checked: false
- id: sdp7-reference-pando
  text: "folder-text-to-parquet is documented as the reference pando: annotated pando.toml, data flow diagram, stage-by-stage explanation. Lives in SDK docs or resources/pandos/."
  checked: false
- id: sdp8-toy-registry
  text: "patina child toys (or similar) lists available toys with descriptions: what each toy does, which WIT interface it maps to, and what [needs].toys key to use in child.toml."
  checked: false
---
# feat: SDK as MCT Developer Platform

## Problem

The current `patina-sdk` is a toy-wrapper crate — it helps children call 4 toys
without raw WIT. But it doesn't help developers understand MCT, discover existing
children, compose pandos, or verify that their wiring is correct.

A developer who wants to build voice-lake-mvp1 today has to: read source code to
find children, read WIT files to understand interfaces, manually write pando.toml
by copying the reference, and hope the composition is correct until Mother loads
it. There's no discovery, no guidance, no validation.

The next wave of Patina work is pando-driven — composing existing children into
new products. The SDK should be the developer's front door into this workflow.

## Goal

`patina-sdk` becomes the MCT developer platform. A developer should be able to:

1. **Discover** what children and toys exist
2. **Understand** how children compose into pandos (the dual-surface model)
3. **Assemble** a pando by selecting children, wiring stages, and adding config
4. **Validate** that the composition is correct before runtime
5. **Run** the pando through Mother with confidence

The litmus test: a developer can build voice-lake-mvp1 using SDK tools alone —
no source code reading, no manual WIT inspection, no copy-paste from existing
pando.toml files.

## Non-Goals

- Building new toys (toy creation is Mother-side, not SDK-side)
- Cross-mother federation or P2P sync
- GUI or web interface
- Changing the `patina-sdk` Rust crate's public API surface

## Litmus Test: Voice Lake MVP1

A developer who knows nothing about Patina internals should be able to:

```bash
# 1. Discover children
patina child list
# → sees 6 pipeline children with stage roles and interfaces

# 2. Inspect the reference pando
patina pando show folder-text-to-parquet
# → sees full composition graph, 12 components, data flow

# 3. Scaffold a voice-scoped variant
patina pando new voice-lake-local
# → gets a pando.toml skeleton with config section

# 4. Fill in: reuse same children, add voice config
#    [config]
#    voice_id = "default"
#    source_id = "local-docs"
#    output_root = "voice/default/local-docs"

# 5. Validate before running
patina pando validate voice-lake-local/pando.toml
# → "composition valid: 12 components, entry: pando/catalog::run()"

# 6. Run through Mother
patina mother start
# → Mother loads pando, executes, output lands in voice namespace
```

## Architecture

### Child Discovery (sdp1)

`patina child list` currently shows installed .wasm files in `~/.patina/plugins`.
Extend to show:
- Child name, kind, version (from child.toml)
- WIT exports: which `patina:records/*` interface the child implements
- WIT imports: which toys the child needs (logging, keyvalue, measure, config)
- Stage role: source / extract / transform / write / catalog (derived from export)

Data source: child.toml manifests + WIT world introspection from installed .wasm.

### Pando Inspection (sdp2)

New `patina pando show <name>` command. Reads pando.toml and displays:
- Children involved (with IDs and instance aliases)
- Adapters (which children they bridge)
- Wiring graph (from → to on which interface)
- Entry point (which adapter's run() Mother calls)
- Config keys (from [config] section, if present)

### Pando Validation (sdp3)

New `patina pando validate <path>` command. Checks:
- All children referenced in pando.toml are installed (have .wasm)
- Each wiring rule references valid export→import interface pairs
- Push children's exports match what adapters expect to import
- No dangling wires (every adapter has its upstream connected)
- Entry point exists and is exported by the final adapter
- Reports errors with actionable messages ("child 'foo' not installed",
  "wiring 'se→df' expects patina:pando/transform but df imports patina:pando/extract")

### Pando Scaffolding (sdp4)

New `patina pando new <name>` command. Creates:
- `<name>/pando.toml` with sections: [pando], [config], [[children]], [composition]
- Comments explaining what each section does
- Config section with placeholder keys
- Wiring skeleton showing the adapter pattern

### Toy Registry (sdp8)

Expose the toy registry to developers. `patina child toys` or similar command
shows available toys with:
- Toy name and description
- WIT interface it maps to (e.g., `wasi:keyvalue/store@0.2.0`)
- `[needs].toys` key for child.toml (e.g., `keyvalue`)
- Which SDK helper wraps it (e.g., `patina_sdk::toys::keyvalue`)

Data source: `wit/toys/deps/toys-registry.toml` or equivalent.

### Reference Pando Documentation (sdp7)

`folder-text-to-parquet` as the worked example:
- Annotated pando.toml with comments on every section
- Data flow: folder → files → records → validated → deduped → parquet → catalog
- Stage-by-stage: what each child does, what its inputs/outputs are
- Config injection: how voice_id / source_id / output_root parameterize the pando
- How to create variants (change config, reuse children)

### SDK Crate Role

The `patina-sdk` Rust crate continues to provide toy helpers (`toys::log`,
`toys::keyvalue`, `toys::measure`, `toys::config`) for child authors who build
new children. This spec does not change the crate's surface — it extends the
developer platform around it with CLI tooling, documentation, and validation.

## Phasing

**Phase 1 (sdp1, sdp2, sdp7, sdp8)** — Discovery and understanding. A developer
can see what exists and how it works. Prerequisite for everything else.

**Phase 2 (sdp3, sdp4, sdp5)** — Authoring and validation. A developer can create
and validate new pandos. Builds on discovery.

**Phase 3 (sdp6)** — Proof. Voice-lake-mvp1 is assembled using SDK tools. This is
the integration test for the platform.

Phase 1 can proceed immediately. Phase 3 runs concurrently with voice-lake-mvp1
implementation — each hardens the other.

## Resolved Decisions

- **CLI-first, not crate-first.** The platform value is in `patina child` and
  `patina pando` commands, not in expanding the Rust crate's API surface.
- **Composition validation is offline.** `patina pando validate` checks the graph
  without starting Mother. Mother's runtime validation is a separate concern.
- **Reference pando is documentation, not code.** folder-text-to-parquet already
  exists as working code. The spec adds developer-facing explanation.
- **Toy helpers stay in the crate.** The SDK crate's role (toy wrappers for child
  authors) is unchanged. This spec wraps a platform around it.

## Implementation Order

1. Extend `patina child list` with interface/stage/toy display (sdp1)
2. Add `patina pando show` command (sdp2)
3. Document folder-text-to-parquet as reference pando (sdp7)
4. Expose toy registry via CLI (sdp8)
5. Add `patina pando validate` command (sdp3)
6. Add `patina pando new` scaffolding command (sdp4)
7. Document pando config injection model (sdp5)
8. Voice-lake-mvp1 proof walkthrough (sdp6)

## Build Readiness

Foundation exists:
- `patina child list` and `patina pando list` commands exist (basic)
- `folder-text-to-parquet` pando is working code with pando.toml
- Mother already validates composition at load time (sdp3 makes it offline)
- child.toml manifests carry metadata for discovery
- `toys-registry.toml` exists in wit/toys/deps/
- voice-lake-mvp1 spec is unblocked and ready as the proving ground
