# Design: Patina as Domain-Agnostic Knowledge System with Persona Federation

## Relationship to scrape-diff-driven

[[scrape-diff-driven]] is the foundation for KSA Phase 1. The alignment:

| KSA EC | scrape-diff-driven Phase | How |
|--------|------------------------|-----|
| `scrape-is-plugin-dispatched` (EC1) | Phase 1 | SDD builds delta → classify → route dispatch. KSA generalizes source-kind dispatch to support plugin-registered source kinds |
| `schemas-live-with-plugins` (EC2) | — | SDD doesn't touch schemas. KSA adds auto-install from plugin manifests |
| `plugins-can-emit-facts` (EC3) | — | SDD plugins return JSON, host writes. KSA adds `host_emit` to WIT for source-kind plugins |
| `forge-extracted-to-plugin` (EC4) | — | Forge uses mother-child world (needs HTTP). SDD's dispatch interface routes to it by source kind |

SDD should be completed (at least Phase 1) before KSA Phase 1 begins.
SDD's delta computation and lazy loading are prerequisites — without them,
adding more source kinds makes the unconditional-loading problem worse.

## Approach

### Phase 1: Plugin System Completion

**Prerequisite:** [[scrape-diff-driven]] Phase 1 complete (delta-driven
dispatch exists).

**Step 1: Source-kind plugin registration.**
Extend `PluginManifest.provides` with a `source_kinds` field:

```toml
# plugin.toml for a source-kind plugin
[provides]
source_kinds = ["forge"]  # registers as handler for "forge" source kind
```

The dispatch interface from SDD Phase 1 gains a plugin-registered arm:

```rust
// After SDD's built-in source kinds
for (kind, plugin) in registered_source_kind_plugins {
    if delta.has_changes_for(kind) {
        dispatch_to_source_kind_plugin(plugin, &delta)?;
    }
}
```

Evidence: `PluginProvides` already has `pipeline_ops` and `languages`
fields (`mod.rs:183-187`). Adding `source_kinds` follows the same pattern.

**Step 2: Add `host_emit` to WIT.**
New interface in `wit/deps/patina-host/host.wit`:

```wit
interface emit {
    /// Emit a fact event to the eventlog.
    /// event_type: schema-qualified type (e.g., "forge.issue")
    /// data: JSON payload matching the schema's WIT record
    /// Returns the event sequence number or error.
    emit-fact: func(event-type: string, data: string) -> result<u64, string>;
}
```

Add to mother-child and task worlds (source-kind plugins need it).
Pipeline world stays log-only — grammar plugins return data to host.

Host validates: schema must be installed, event_type must match a fact
definition in the schema, data must parse as valid JSON.

**Step 3: Schema auto-install.**
When a plugin with `schemas` in its manifest is loaded, the host checks
`.patina/schemas/<name>/` and auto-installs if missing or outdated.

Plugin manifests already declare schemas (`PluginManifest.schemas:
HashMap<String, String>`, `mod.rs:147`). The wiring is: on plugin load,
compare declared schema version with installed version, copy schema
files from plugin directory to `.patina/schemas/`.

**Step 4: Forge extraction.**
Move `src/forge/` to a mother-child plugin:
- Plugin gets `host_http` for GitHub API calls
- Plugin gets `host_emit` for writing forge facts to eventlog
- Plugin ships `forge` schema (WIT types + schema.toml)
- Plugin registers `source_kinds = ["forge"]`

The forge source kind is NOT triggered by git diff — it's triggered by
user command (`patina scrape forge`) or sync schedule. The dispatch
interface handles this: source kinds declare their trigger mechanism
(delta-driven vs command-driven vs schedule-driven).

### Phase 2: Core Extraction

After forge proves the pattern, extract spec and session subsystems:

- **Spec plugin:** Complex — touches filesystem, database, git, release,
  MCP. Requires `host_emit` for eventlog, host-provided git operations,
  and MCP tool registration. See KSA SPEC "Spec extraction coupling" for
  full analysis.

- **Session plugin:** Simpler — reads/writes layer/sessions/ files,
  creates git tags. Requires filesystem access (via host_layer) and git
  operations (new host interface or toys).

Extraction order: forge first (proves pattern), sessions second (simpler),
spec last (most complex).

### Phase 3: Mother and Personas

After core extraction, domain-specific code lives in plugins. Mother
gains persona and lake management:

- Persona registry with UIDs
- Belief provenance via persona UID
- Persona linking with directional streams
- Lake registry extending ref repo pattern

This phase depends on [[fact-crdt-substrate]] for sync.

## Commits

Phase 1 (estimated):
1. `feat(plugin): source_kinds field in PluginProvides` — manifest parsing
2. `feat(host): add host_emit WIT interface` — emit-fact function
3. `feat(host): implement host_emit for mother-child world` — eventlog write
4. `feat(schema): auto-install schemas from plugin manifest` — on load
5. `refactor(forge): extract to mother-child plugin` — first extraction
6. `verify(scrape): forge plugin dispatched by source kind` — EC1, EC4

## Key Files

- `src/plugin/internal/mod.rs` — `PluginProvides`, `PluginManifest`
- `wit/deps/patina-host/host.wit` — new `emit` interface
- `wit/mother-child/mother-child.wit` — import `host_emit`
- `src/commands/scrape/mod.rs` — source-kind dispatch extension
- `src/commands/schema/internal.rs` — auto-install logic
- `src/forge/` → `plugins/forge/` — extraction target

## Open Questions

1. **Source-kind trigger types:** Git-diff-driven source kinds (code, layer)
   use the delta. API-driven source kinds (forge) use sync commands. How
   does the dispatch interface unify these? Options: source kinds declare
   their trigger type in manifest, or dispatch always passes delta and
   source kinds ignore it if irrelevant.

2. **Spec extraction host boundary:** Spec plugin needs git operations
   (tags, staging, commits), filesystem write, eventlog access, and MCP
   tool registration. These are significant host capabilities not in any
   current WIT interface. New host interfaces needed, or toys pattern?

3. **Schema versioning:** What happens when a plugin updates its schema
   and the installed version is older? Migration strategy needed — the
   schema system has `version` fields but no migration mechanism.

4. **Dependency ordering:** SDD Phase 1 → KSA Phase 1 → SDD Phase 3
   (Mother warm-host depends on KSA expanding Mother's plugin hosting).
   This chain needs explicit tracking in spec `blocked_by` fields.
