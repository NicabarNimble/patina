---
type: explore
id: mother-child-artifact-registry
status: draft
created: 2026-04-07
beliefs:
  - "[[children-are-portable-wasm-artifacts]]"
  - "[[mother-manages-artifact-install-and-runtime]]"
  - "[[pandos-are-shareable-compositions]]"
related:
  - layer/surface/build/feat/pando-platform/SPEC.md
  - src/commands/child.rs
  - src/commands/mother/daemon.rs
  - src/child/internal/mod.rs
exit_criteria:
  - Decide canonical artifact identity schema (name, version, digest, source, publisher persona/user/mother)
  - Define Mother local artifact store model vs runtime instance model (install/cache vs live)
  - Define child install workflow replacing shell copy (`build`, `install`, `list`, `verify`, `pull`)
  - Define trust/provenance model for shared artifacts across Mothers (signing, attestations, trust policy)
  - Produce migration plan from monorepo source-forge installs to artifact-first installs without breaking existing children
---
# explore: Mother child artifact registry workflow

> Replace manual shell artifact handling with a Mother-native child artifact workflow that treats children as portable, versioned WASM artifacts and pandos as shareable compositions.

## Question

How should Patina evolve from repo-local child source + manual copy installs to a
first-class artifact model where Mother manages install/cache/runtime identity,
and artifacts/compositions can be shared across a future P2P Mother network?

## Context

Current runtime proof works: `folder-text-to-parquet` reaches `live` when six
children are installed and loaded. But operations still depend on shell steps
(`cargo build`, `cp wasm`, `cp child.toml`) and artifact identity can drift
from canonical manifest identity if filename conventions leak into runtime
logic.

The desired model is:

1. Children are reusable artifacts.
2. Mother owns install/cache and runtime instance tracking.
3. Pandos reference child artifact identities, not source tree assumptions.

## Areas To Explore

### 1) Artifact identity and metadata

- Canonical key: `[child].name` + version + digest.
- Required metadata: build target, ABI/world, toy grant compatibility, source,
  publisher persona/user/mother.
- Backward compatibility with current `.wasm + .toml` pairs.

### 2) Workflow surface

Candidate command surface (exact verbs TBD):

- `patina child build <name> --release`
- `patina child install <name> --from build`
- `patina child list --installed`
- `patina child verify <name>`
- `patina child pull <ref>` (registry/P2P path)

Goal: eliminate brittle manual copy flows while preserving explicit user control.

### 3) Mother state model

- Artifact registry/cache table(s): what is installed and verified.
- Runtime instance table(s): what is live, healthy, and bound.
- Mapping from pando composition requirements to installed/live artifacts.

### 4) Sharing and trust

- How one Mother publishes artifacts/compositions for others.
- Signature/attestation model.
- Trust policy surface (allowlist, personas, org roots, explicit consent).

## Non-Goals

- Building full P2P distribution transport in this explore.
- Replacing current child runtime interfaces.
- Redesigning pando command dispatch (Phase B scope remains separate).

## Recommended Next Step

Promote to a `feat` spec once identity schema + workflow surface are decided,
then implement in slices: local artifact workflow first, network sharing second.
