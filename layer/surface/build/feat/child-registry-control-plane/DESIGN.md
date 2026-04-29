# DESIGN — child-registry-control-plane

## Intent

Establish Mother as the control plane for external child distribution and usage:

1. discover child artifacts from GitHub first,
2. normalize them into a provider-agnostic registry model,
3. require approval before install/assignment,
4. install pinned+verified artifacts into local runtime paths,
5. assign approved child versions to specific projects.

This enables external child repos (starting with Slate) without weakening trust or reproducibility.

---

## Boundary and ownership

### State seam requirement (dependable-rust / unix)

Child-registry persistence logic must not continue as monolithic growth in `mother/src/state.rs`.

Introduce a dedicated seam:

- `ChildRegistryStore` (focused API for sources, entries, installs, assignments)
- implementation isolated in `mother/src/state/children_registry.rs`
- `MotherRuntimeStore` delegates to this seam

This keeps the public surface small, implementations swappable, and review scope bounded.


- **Mother (`mother` crate):** authority for registry state, approval policy, install verification, assignment policy.
- **CLI (`patina-ai`):** operator surface only (`patina mother children ...`).
- **Children:** execution capabilities after assignment/activation.
- **Providers (GitHub, Gitea, etc.):** metadata/artifact discovery adapters; no policy authority.

Policy remains in Mother. Providers are data sources.

---

## Domain model (canonical)

### Registry Source

Represents where entries are discovered.

- `source_id` (stable local id)
- `provider_kind` (`github|gitea|custom`)
- `provider_config_json` (owner/repo, base_url, auth profile)
- `enabled` (bool)
- `last_sync_at`, `last_sync_status`, `last_error`

### Child Release Entry

Represents a specific installable child version.

- `entry_id` (stable id)
- `child_name` (canonical runtime child name)
- `version` (semver-ish string)
- `source_id`
- `source_release_ref` (tag/release id)
- `artifact_url` (wasm)
- `manifest_url` (toml)
- `checksums_url` (optional)
- `artifact_sha256` (required before install)
- `manifest_sha256` (required before install)
- `signature_ref` (optional, policy-gated)
- `patina_min` (optional)
- `operations_json` (declared contract allowlist)
- `needs_toys_json` (declared `[needs].toys`)
- `needs_scopes_json` (declared `[needs.scopes]`)
- `state` (`candidate|approved|blocked|deprecated`)
- `state_reason`
- `created_at`, `updated_at`

### Install Record

Tracks local materialization and provenance.

- `install_id`
- `entry_id`
- `installed_name`
- `installed_version`
- `wasm_path`, `manifest_path`
- `artifact_sha256_verified`
- `manifest_sha256_verified`
- `installed_at`
- `installed_by`
- `status` (`installed|superseded|removed|failed`)
- `last_error`

### Project Assignment

Binds project identity to an approved installed child version.

- `assignment_id`
- `project_uid`
- `project_id` (optional canonical id)
- `child_name`
- `entry_id`
- `pinned_version`
- `status` (`active|revoked`)
- `reason`
- `created_at`, `updated_at`

Uniqueness constraint:
- one `active` assignment per (`project_uid`, `child_name`) unless explicit multi-version mode is introduced later.

---

## State schema (Mother DB additions)

Add tables in `mother/src/state/mod.rs` schema init, with child-registry operations implemented via the dedicated child-registry seam:

- `mother_child_sources`
- `mother_child_registry_entries`
- `mother_child_installs`
- `mother_project_child_assignments`
- `mother_child_audit_events` (optional if not reusing existing eventlog)

Indexes:

- entries by `(child_name, version)`
- entries by `state`
- assignments by `(project_uid, status)`
- installs by `(entry_id, status)`

Guardrails:

- assignment insert requires entry `state='approved'`
- install requires non-empty hashes
- block state denies new install/assignment

---

## Provider adapter contract

Introduce provider trait (host-side, not policy):

```rust
trait ChildRegistryProvider {
  fn kind(&self) -> &'static str;
  fn sync(&self, source: &ChildSourceConfig) -> Result<Vec<DiscoveredRelease>>;
  fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>>;
}
```

`DiscoveredRelease` normalized fields:

- child identity metadata
- version/release refs
- artifact/manifest URLs
- checksums/signature refs

### GitHub adapter (v1)

`provider_config_json` includes:
- `owner`, `repo`
- optional `asset_name_wasm`, `asset_name_manifest`, `asset_name_checksums`
- optional auth connection id

Uses GitHub Releases API and deterministic asset resolution.

### Gitea adapter (v1.1)

Same normalized output contract; only API client differs.
No domain/schema changes.

---

## Approval state machine

States:

- `candidate` (default on sync)
- `approved`
- `blocked`
- `deprecated`

Transitions:

- `candidate -> approved` (manual/policy)
- `candidate -> blocked`
- `approved -> blocked`
- `approved -> deprecated`
- `deprecated -> approved` (explicit restore)

Rules:

- only `approved` may be installed/assigned
- `blocked` hard-denies
- `deprecated` warns; existing assignments remain but new assignment policy is configurable (default deny)

Every transition emits structured audit event.

---

## Install flow (pin-first, fail-closed)

Input: `child@version` (or entry id)

1. Resolve entry in registry
2. Ensure state is `approved`
3. Download wasm + manifest
4. Compute SHA256 and compare to pinned registry hashes
5. Stage to temp location
6. Atomic move to `~/.patina/children/<name>.wasm|.toml`
7. Write/refresh hash sidecars (existing integrity mechanism)
8. Record install row
9. Trigger warmup/refresh path in Mother

Failure at any point -> rollback staged files, keep prior installed artifacts intact.

---

## Project assignment flow

Input: `<project> <child@version>`

1. Resolve project identity (`project_uid`, optional `project_id`)
2. Verify entry is approved and installed
3. Upsert active assignment row
4. Emit audit event
5. Apply runtime routing/availability update (or on next warmup)

Policy check point:
- child usage for project should consult assignments (enforce allowlist, fail closed if required by policy mode).

---

## CLI contract (`patina mother children ...`)

### Sources

- `sources add github <owner>/<repo>`
- `sources add gitea <base-url> <owner>/<repo>`
- `sources list`
- `sources disable <source-id>`

### Registry

- `sync [--source <id>] [--dry-run]`
- `search <query>`
- `show <child> [--version <v>]`
- `approve <child>@<version> [--reason ...]`
- `block <child>@<version> [--reason ...]`
- `deprecate <child>@<version> [--reason ...]`

### Install/assignment

- `install <child>@<version> [--dry-run]`
- `assign <project> <child>@<version> [--reason ...]`
- `unassign <project> <child> [--reason ...]`
- `status [--project <project>]`

All mutating commands support structured JSON output and explicit failure reasons.

---

## Backward compatibility

- Existing local child loading from `~/.patina/children` remains valid.
- Registry install is additive.
- Assignment enforcement can begin in `observe` mode (audit-only), then switch to `enforce`.

---

## External child proof (Slate)

Acceptance path:

1. Slate lives in external repo (GitHub first).
2. Build publishes wasm/toml (+ checksums) as release assets.
3. Mother source sync discovers version.
4. Operator approves pinned Slate version.
5. Mother installs and verifies hashes.
6. Assign Slate to project.
7. Verify routed spec/slate execution uses assigned version.

---

## Implementation slices

### Slice A0 — State seam refactor (required)
- Create `ChildRegistryStore` and isolate child-registry state logic in `mother/src/state/children_registry.rs`.
- Keep `MotherRuntimeStore` public API stable via delegation.
- Add deterministic seam tests (happy path + fail-closed assignment guard).

### Slice A — Schema + state APIs
- DB tables, structs, CRUD methods, guardrails (executed through `ChildRegistryStore`).

### Slice B — GitHub provider
- source sync + normalize releases -> entries.

### Slice C — Approval and install
- state transitions, hash verification, atomic install.

### Slice D — Assignment and runtime policy
- project->child binding and enforcement hooks.

### Slice E — CLI surface
- `patina mother children ...` commands + JSON outputs.

### Slice F — Slate external proof
- first external child onboarding end-to-end.

---

## Open decisions

1. Signature policy default: warn-only vs strict required.
2. Deprecated assignment policy: allow existing only vs deny all new + optional force.
3. Multi-version assignment strategy per project (defer unless needed).
4. Whether provider auth binds to `connect` identities or source-local tokens only.
