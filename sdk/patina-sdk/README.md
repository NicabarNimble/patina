# patina-sdk

`patina-sdk` is the authoring surface for Patina WASM children.

## SDK Tiers

- `patina-sdk-core`: child trait + core toys (`log`, `state`, substrate types)
- `patina-sdk-data`: data toys (`lake`, `checkpoint`, `measure`, `github`)
- `patina-sdk-agent`: agent/session toys (`query`, `emit`, `session`)
- `patina-sdk`: umbrella crate that re-exports tier APIs

Use the umbrella crate unless you are building advanced tooling around the tiers directly.

## 5-Minute Onramp

1. Generate a child from the template:

```sh
cargo generate --path children/template
```

2. Build the child (WASM):

```sh
cargo build --target wasm32-wasip2
```

3. Ensure `child.toml` uses `[child].name` as canonical identity and `[needs].toys` for grants.

4. Install the child artifact + manifest into Patina's children directory.

5. Start Mother and verify the child loads:

```sh
patina mother start
patina mother status
```

## Child Baseline

Use this feature set for a minimal child:

```toml
[dependencies]
patina-sdk = { version = "0.21", features = ["child", "toy-log"] }
```

Add toys incrementally (`toy-state`, `toy-checkpoint`, `toy-lake`, `toy-github`, `toy-session`, etc.)
as your `child.toml` grants expand.

## World Features

Enable exactly one world feature per crate:

- `child` (default path)
- `pipeline` (experimental lane)
- `task` (legacy compatibility scaffold)
- `command` (legacy compatibility scaffold)

M5 stabilization target in this repo is `child` plus tier crates.

## Stability Policy

| Lane | Status | Policy |
| --- | --- | --- |
| `child` | stable | canonical child authoring surface |
| `pipeline` | experimental | opt-in, no stability promises yet |
| `task` | migration shim | compatibility-only, removal-gated |
| `command` | migration shim | compatibility-only, removal-gated |

## Breaking Change (2026-03)

- `mother-child` SDK feature is retired.
- `MotherChild` trait and `register_mother_child!` are removed from `patina-sdk`.
- Migrate child crates to `child` (preferred) or `task`/`command` where appropriate.

Shim removal gates:

1. SDK compatibility matrix compiles cleanly across supported lanes.
2. Scaffold output parity remains green for child-first naming and manifests.
3. Removal is spec-authorized with rollback-safe migration slices.

## Toy Definition

A toy is a controlled opening in the WASM sandbox wall.

- Mother defines the opening; children do not invent new toys.
- Grants are explicit via `child.toml` (`[needs].toys` + optional `[needs.scopes]`).
- Scopes shape authority (domains, sources, names, resources); they do not create new toy kinds.
- Domain logic belongs in children; toys provide boundary access only.

Litmus test for adding/keeping a toy:

- "Why can't the child do this itself from pure WASM compute?"
- If the child can do it without host authority, it is an SDK/library concern, not a toy.

Anti-goals:

- Toys are not convenience wrappers for provider-specific product logic.
- Toys are not a child-defined extension surface.
- Toys are not a way to bypass scoped grants or host-side policy checks.

## Canonical Toybox (v1)

Canonical lock fields are tracked in `sdk-toybox-definition` design (`direction`, `host boundary`, `scope knobs`, `tier`, rationale).

Quick index by tier:

- Core: `log`, `state`, `layer`, `layer-fs`, `git`, `peer`, `task`
- Data: `lake`, `checkpoint`, `measure`, `github`, `connector`
- Agent: `query`, `emit`, `session`, `events`, `ingress`, `http`, `belief`, `graph`
- Boundary-deferred: `schema`
- Support contract only: `types`

Tier ownership note:

- Canonical ownership sits in `patina-sdk-core` / `patina-sdk-data` / `patina-sdk-agent`.
- `patina-sdk` may provide sdk-local wrapper toys for cross-tier ergonomics, but wrappers do not redefine toy authority boundaries.

Treat this as finite platform surface. New toy proposals must pass the toy litmus test and spec-authorized migration policy.

## Child Relationships

Children can declare mediated event relationships in `child.toml`:

```toml
[relationships]
emits = ["data-ingested"]
listens = ["data-ingested"]
```

Use this to describe child-to-child flow while keeping Mother as the routing authority.

Example pattern:

- `ducklake` emits `data-ingested` after sync
- `session-writer` listens to `data-ingested` and appends activity notes

## License

MIT
