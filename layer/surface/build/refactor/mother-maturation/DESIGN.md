# Design: Mother Maturation — Federation, Lakes, and Continuity

## Why This Work Exists

Mother started as the catch-all. "Things that don't fit in a project"
landed here — project registry, ref repos, cross-project search. She
grew organically as Patina grew. The architectural pivot
([[session-20260303-190855]], [[session-20260304-120702]]) clarified her
role: Mother is connection + continuity. Not business logic. Not a
catch-all. Registries and routing.

[[mother-is-connection-and-continuity]]: "Projects are sovereign islands;
Mother is the nervous system that makes them aware of each other. Without
Mother, Patina instances are isolated. With Mother, beliefs evolve from
cross-project evidence."

But today Mother is a batch command. `patina mother` runs, syncs
graph.db, exits. The vision is fundamentally different: "at the Mother
level, connectors are pulling, lakes are syncing, belief streams are
flowing. Always on." That's not a CLI command. That's a daemon. That's
a shift from tool to infrastructure.

This container grows Mother from a project registry with federated search
into the nervous system connecting all Patina instances — projects,
apps, lakes, personas. Everything else in the system (protocol,
plugins, beliefs) operates within a project. Mother operates between
them.

**Origin:** [[session-20260303-190855]] ("it is so important for patina
to connect... part of the evolve is patina learns from other projects
or apps"), [[session-20260304-120702]] (scoped Mother to connection +
continuity, designed data flow with lakes and blocks).

## What Mother Does Today

| Capability | Implementation | Status |
|-----------|---------------|--------|
| Project registry | `mother.db → project_registry` | Working |
| Ref repo registry | Cross-project git references | Working |
| Mother-child world | Daemon with heartbeat, tick(), health | Working |
| Cross-project search | `patina mother` + graph.db FTS5 | Working |
| Graph sync | `patina mother graph sync` | Working |

This is a solid foundation. The mother-child plugin world is the key
infrastructure — it already supports daemon-resident plugins with
heartbeat, health checks, and work requests (toys). What's missing
isn't mechanism. It's scope.

## What Mother Needs to Become

### Registries (what Mother knows about)

| Registry | What it tracks | Today | Needed |
|----------|---------------|-------|--------|
| **Projects** | Patina instances, their locations, UIDs | Yes | Expand with status, health |
| **Personas** | Who — identities, visibility, linking | No | New |
| **Lakes** | External data — name, kind, location, credentials, sync | No | New |
| **Connectors** | Plugins that fetch — which connector feeds which destination | No | New |
| **Belief streams** | Knowledge flow — which projects share beliefs, direction, scope | No | New |

### Routing (what Mother does with what she knows)

Mother knows WHERE things are and HOW to reach them. She doesn't process
data, make decisions, or hold beliefs. Her job is connecting:

- Source → Lake: Mother manages the connector, schedules syncs,
  holds credentials
- Lake → Block → Project: Mother routes shaped data to the projects
  that consume it
- Project → Project: Mother routes belief streams between projects
  within a persona's network
- Project → Edge app: Mother coordinates sync to deployed apps
  (future)

Connectors do the fetching. Projects do the believing. Mother connects.

### Continuous Operation (how Mother runs)

Today: `patina mother` is a CLI command. Run it, it does things, it
exits. This is fine for project registry and graph sync.

Future: Mother is a daemon. She runs continuously. Connectors pull on
schedule. Lakes sync. Belief streams flow. Health checks monitor
children. This is [[mother-is-connection-and-continuity]] fully
realized — the nervous system is always on, not activated on demand.

The mother-child plugin world already has the daemon architecture:
`tick()` heartbeat, `health()` checks, `toy` work requests. Continuous
operation extends this from "Mother loads children and ticks them" to
"Mother IS the continuous process that manages everything."

## The Data Flow — From Source to Belief

Your napkin shows the full landscape. Here it is with Mother's role
annotated:

```
SOURCES (external, not ours)
    │
    │  connectors (Mother-managed or project-scoped)
    │
    ├──→ DATA LAKES (Mother-managed, shared)
    │        │
    │        │  data blocks (shaped for purpose)
    │        │
    │        ├──→ DATA BLOCKS ──→ PROJECT/APP
    │        │    (structured,    (action + belief layer)
    │        │     versatile)
    │        │
    │        └──→ PROJECT/APP (direct from lake)
    │
    ├──→ DATA BLOCKS (direct from source, skip lake)
    │        │
    │        └──→ PROJECT/APP
    │
    └──→ PROJECT/APP (direct from source, simple case)
```

**Sources** can connect at any level — lakes, blocks, or projects.
The connector interface is the same regardless of destination (see
[[spec-plugin-infrastructure]] DESIGN.md, "Connector Destination
Independence"). What changes is who manages the connector:

- **Mother manages** connectors that feed lakes (shared infrastructure)
- **Projects manage** connectors that feed directly (simple case)
- **Either manages** connectors that feed blocks (depends on scope)

**Lakes** are raw external data. Mother-managed, shared across projects.
Could be local (SQLite, directory), could be remote (S3, database).
Mother tracks them in the lake registry: name, kind, location,
credentials, sync state.

**Data blocks** are shaped data between lakes and projects. This concept
is intentionally open — see "Data Blocks: Exploration Territory" below.

**Projects/apps** are action + belief layers. They consume data from
any source, process it through the protocol (capture/index/search/
believe/evolve), and produce beliefs. [[patina-is-beliefs-plus-action]]:
"A Patina project IS an action layer with a belief layer inside it."

## Three Dimensions of Growth

The three children represent three dimensions Mother needs to grow in.
Each is relatively independent but they compose into the full vision.

### Dimension 1: Data Infrastructure — [[spec-data-architecture-v3]]

**What it delivers:** Provenance on events, lake registry in Mother.

Mother needs to manage external data infrastructure. Today she knows
about projects and ref repos. She needs to also know about:

- **Lakes** — where external data lives, how to reach it, credentials,
  sync state, freshness
- **Event provenance** — when a fact enters the eventlog, where did it
  come from? A connector? A grammar? Local git? Provenance tracking
  distinguishes "we observed this" from "we derived this" from "a user
  declared this"

This is the "stores" dimension — giving Mother the registries and
metadata to manage data infrastructure across the system.

**Builds on:** [[spec-plugin-infrastructure]] (connectors exist and can
emit facts) and [[spec-data-architecture-v2]] (event store architecture).

### Dimension 2: Identity — [[spec-persona-federation]]

**What it delivers:** Persona registry, belief provenance, linking.

Mother needs to manage identity. Today beliefs are loosely attributed —
they come from a project, they have a persona field, but there's no
registry of who personas are or how they relate.

In a federated world:
- **Personas** have UIDs, visibility settings, linked projects
- **Beliefs carry provenance** — who captured this, from which project,
  grounded in what evidence from which sources
- **Cross-persona discovery** — "this persona in this project holds a
  belief similar to yours" (semantic similarity across projects)
- **Linking** — one human might have multiple personas (work, personal,
  per-client). Mother knows the linking. Projects don't.

This is the "who" dimension — giving Mother identity infrastructure
that makes federation meaningful. Without personas, belief streams are
anonymous. With personas, beliefs have human provenance.

### Dimension 3: Always On — [[spec-continuous-operation]]

**What it delivers:** Mother daemon, streaming, always-on.

Mother needs to run continuously. The current batch model (run command,
do work, exit) can't support:

- Connectors pulling on schedule (minutes, not "when user runs scrape")
- Belief streams flowing between projects (reactive, not polling)
- Health monitoring across all children (continuous, not on-demand)
- Lake sync state management (track what's fresh, what's stale)

The mother-child plugin world already has the daemon primitives:
heartbeat via `tick()`, health checks, toy work requests. Continuous
operation is Mother herself becoming the always-on process, not just
loading children who are always-on.

**Depends on dimensions 1 + 2:** You need lakes and personas before
continuous routing makes sense. What would continuous operation route
if there are no lakes to sync and no personas to stream between?

This is the "how" dimension — giving Mother life. The nervous system
wakes up.

## Data Blocks: Exploration Territory

Data blocks sit between lakes and projects. They're shaped data —
filtered, structured, projected for a purpose. The concept emerged in
[[session-20260304-120702]] and is intentionally open.

**What we know:**
- Lakes are raw (the firehose from connectors)
- Blocks are shaped (data that's been structured for consumption)
- Blocks should be versatile — "could they be semantic or structured?"

**What a block could be:**
- **Structured table** — filtered issues from a GitHub lake, mapped to
  a schema. "Give me only open issues from the patina repo."
- **Semantic projection** — vectorized documents from a document lake.
  "Embed all legal briefs for similarity search."
- **Filtered view** — a slice of raw lake data matching criteria.
  "All emails from this client in the last 30 days."
- **Aggregation** — computed summaries. "Weekly commit activity across
  all repos in this lake."
- **Something more versatile** — blocks might not be one thing. They
  might be a general concept that encompasses all of the above.

**What we're NOT locking down:**
- Block storage format (SQLite? Files? Embedded in Mother?)
- Block definition language (SQL views? Config? Plugin logic?)
- Block lifecycle (who creates them, how they update, when they expire)
- Whether blocks are a Mother concept or a project concept or both

**The DESIGN.md for [[spec-data-architecture-v3]] should explore this.**
Data blocks may need their own spec if the design space is large enough.
The container acknowledges the concept and its place in the data flow
without pretending we know the shape.

## Future Paths — Architecture Preserves, Doesn't Build

These are explicitly NOT in scope for this container. But the
architectural decisions made here must not block them.

### Edge Interface

[[local-first-edge-deployable]]: local Patina is the belief factory,
edge apps are belief consumers. Mother coordinates the connection.

**The question:** How do Patina apps on Cloudflare/Vercel connect back
to local Mother? WebSocket? HTTP polling? Push via R2?

**What preserves the path:** Mother's continuous operation model is
the foundation. A persistent connection from an edge app to Mother is
just another child in Mother's network. The mother-child world already
supports it architecturally. The transport (WebSocket, HTTP long-poll,
etc.) is an implementation question for when edge apps exist.

### E2EE on Belief Streams

The user's blockchain background (Giza, Starknet STWO, ZK circuits)
opens a path: belief streams between personas could be encrypted
end-to-end. Mother routes payloads she can't read. Only linked personas
with keys can decrypt.

**What preserves the path:**
- [[content-addressed-references]] — content-addressed evidence enables
  cryptographic verification. Belief grounding chains can become ZK
  proofs: "I hold a belief grounded in 3 pieces of evidence. Here's a
  proof the evidence exists. You can't see the evidence."
- Append-only eventlog — structurally similar to a local chain. Add
  merkle roots = tamper evidence. Add signatures = provenance proofs.
- Mother as routing pipe — Mother routes belief streams without
  processing content. If the content is encrypted, Mother's job
  doesn't change. She doesn't need to read beliefs to route them.

### Multi-User

Projects as sovereign islands with federation through Mother, not
shared git. Each user's project is a child node. Changes flow through
Mother. Conflicts resolve at the belief level ("your belief contradicts
mine" is meaningful; merge conflicts on YAML are not).

**What preserves the path:** The Mother-child model already treats
projects as autonomous nodes. Multi-user is "more nodes, same pattern."
Persona federation (dimension 2) adds the identity layer that
multi-user requires.

## The Scope Risk

[[mother-is-connection-and-continuity]] explicitly flags this tension:
"Mother as catch-all is a design smell. If Mother's scope keeps growing,
she becomes a monolith."

Lakes, personas, connectors, belief streams, continuous daemon, edge
coordination, E2EE — that's a lot of surface area. The scoping
principle:

**Mother does registries and routing. Not business logic.**

| Mother DOES | Mother DOESN'T |
|------------|---------------|
| Know where lakes are | Process lake data |
| Route facts to destinations | Decide what facts mean |
| Manage connector schedules | Parse connector output |
| Stream beliefs between projects | Hold beliefs herself |
| Track persona identity | Make decisions for personas |
| Monitor child health | Fix unhealthy children |

If a proposed Mother feature involves processing data or making
decisions, it's a plugin or a project concern, not Mother's job.
Mother is the nervous system, not the brain.

## Children Dependency Graph

```
┌──────────────────────────────┐
│  plugin-infrastructure       │  ← must be complete first
│  (host_emit + roles)         │
└──────────────┬───────────────┘
               │
               ↓
┌──────────────────────────────┐
│     mother-maturation        │  ← this container
│        (container)           │
└──────────┬───────────────────┘
           │
     ┌─────┼──────────────┐
     ↓     ↓              ↓
┌────────┐ ┌────────────┐ ┌──────────────┐
│ data-  │ │  persona-  │ │ continuous-  │
│ archi- │ │ federation │ │  operation   │
│ tecture│ │            │ │              │
│  -v3   │ │            │ │              │
│        │ │            │ │              │
│ FIRST  │ │ SECOND     │ │    THIRD     │
│(stores)│ │(or par)    │ │ (depends on  │
│        │ │(identity)  │ │  1 + 2)      │
└────────┘ └────────────┘ └──────────────┘
```

**data-architecture-v3** first — Mother needs lake and provenance
infrastructure before she can manage data flow.

**persona-federation** second or parallel — identity is relatively
independent from data infrastructure. Could proceed alongside v3.

**continuous-operation** third — depends on both. You need lakes to
sync and personas to stream between. The daemon model is the capstone.

## What's NOT In Scope

- **No edge interface design.** Transport mechanism for edge apps is
  future work. Architecture preserves the path.
- **No E2EE implementation.** Cryptographic belief streams are far
  future. Content-addressed references keep the path open.
- **No multi-user coordination.** Mother-child model points the
  direction. Actual multi-user is post-continuous-operation.
- **No data block format.** Blocks are acknowledged, placed in the
  data flow, flagged as exploration territory. May become their own spec.
- **No core extraction work.** Forge, scrape, spec subsystem — that's
  [[spec-core-extraction]]'s scope. Mother-maturation assumes plugins
  exist and can emit facts.

## Belief Anchors

**Identity (what Mother is):**
- [[mother-is-connection-and-continuity]] — the defining belief.
  Connection (federation, registries, routing) + continuity (always on,
  always syncing). Not business logic. Not a catch-all.
- [[patina-is-beliefs-plus-action]] — Mother enables the "evolve" verb
  across projects. Without federation, beliefs can only evolve from local
  evidence. Mother makes cross-project evolution possible.

**Architecture (how Mother fits):**
- [[local-first-edge-deployable]] — Mother coordinates local and edge
  nodes. Local Patina is the belief factory. Edge apps are consumers.
  Mother is the bridge.
- [[content-addressed-references]] — keeps the ZK/E2EE path open.
  Content-addressed evidence references are prerequisite for
  cryptographic verification of belief grounding chains.
- [[patina-is-domain-agnostic-knowledge-system]] — Mother is
  domain-agnostic by construction. She routes beliefs and manages
  lakes regardless of what domain the projects operate in.
- [[wit-is-contract-wasm-is-one-runtime]] — Mother-child plugins
  speak WIT. Connectors managed by Mother speak WIT. The interface
  is the architecture, regardless of runtime.

## Key Files (Current Mother Implementation)

- `src/mother/` — Mother module (registry, graph, daemon)
- `src/mother/graph.rs` — graph.db sync, federated FTS5 search
- `src/commands/mother/` — CLI commands (graph sync, search)
- `wit/mother-child/mother-child.wit` — mother-child world definition
- `src/plugin/internal/mother_child.rs` — WASM runtime, WasmChild adapter

## Open Questions

1. **Lake storage abstraction.** Lakes can be local (SQLite, directory)
   or remote (S3, database). Does Mother need a storage abstraction
   layer for lakes, or does she just track metadata (location, kind,
   credentials) and let connectors handle the details?

2. **Data block ownership.** Are blocks Mother-managed (shared
   infrastructure) or project-managed (local to a project) or both?
   Who creates a block? Who updates it? Who decides when it's stale?

3. **Daemon lifecycle.** How does Mother's daemon start? On login?
   On first `patina` command? Explicit `patina mother daemon start`?
   How does it interact with the existing CLI model? Launchd/systemd
   integration?

4. **Belief stream protocol.** How do beliefs flow between projects?
   Push (project emits, Mother routes)? Pull (project queries Mother)?
   Pub/sub (project subscribes to topics)? The transport matters for
   latency, resource usage, and the future E2EE path.

5. **Graph.db evolution.** Graph.db currently does federated FTS5
   search. As Mother gains lakes, personas, and belief streams, does
   graph.db grow to encompass all of this? Or do new stores emerge
   (lakes.db, personas.db)? The current single-file model is simple
   but may not scale to Mother's expanded scope.
