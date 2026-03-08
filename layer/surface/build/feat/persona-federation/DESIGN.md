# Design: Persona Federation — From Default String to Sovereign Identity

## Why This Spec Exists

This is Dimension 2 (Identity) of [[spec-mother-maturation]]. Mother
needs to know WHO — not just what projects exist, but who owns them,
who generated this belief, who is talking to whom. Without identity,
federation is anonymous. Belief streams (Dimension 3) can't carry
provenance. Cross-project learning (the "evolve" verb) can't attribute
knowledge.

[[persona-is-a-patina-instance]]: "A persona is a full Patina instance
— its own beliefs, plugins, projects, and knowledge — not a filter or
mode within a single instance." This means persona federation isn't
adding a tagging system to Mother. It's adding an identity layer that
makes every other federation feature meaningful.

**Origin:** [[session-20260302-072907]] (persona-as-instance design),
[[session-20260303-190855]] ("Mother has personas which divide belief
networks"), [[session-20260304-120702]] (multi-user is federation, not
shared state).

## What Exists Today

Persona-federation is greenfield at the Mother level. There is no
identity infrastructure today. What exists are the pieces it builds on:

### The Dead String on Beliefs

Every belief file carries `persona: architect` in its frontmatter.
All 191+ beliefs use this value. It's never varied. The SPEC.md
calls it "a dead string field" — accurate. There's no registry behind
it, no UID, no validation. It's a label with no identity.

This string field is the ONE piece that gets a new life — it becomes
a UID linking to Mother's persona registry.

### Mother's Current Registries

Mother manages registries in two places:
- `~/.patina/registry.yaml` — projects (empty) and ref repos (25 repos)
- `~/.patina/graph.db` — federated belief search (`beliefs`,
  `belief_supports`, `belief_attacks`, `belief_applied_in` tables)

In graph.db, the `beliefs` table has `source TEXT NOT NULL` which
tracks which project contributed a belief, and the `belief_applied_in`
table tracks which projects a belief appears in. But `source` is a
project path slug, not a persona identifier.

**This is the starting point** — Mother's registry infrastructure
needs a new dimension: identity.

## What Changes

### Step Zero — Remove the Pre-Pivot Persona Code

Before building the new identity infrastructure, remove the legacy
`patina persona` command. It was a user-facing knowledge oracle built
before the architectural pivot — a different concept that shares the
name. Leaving it in creates confusion for future sessions about what
"persona" means in this codebase.

**Remove:**
- `src/commands/persona/mod.rs` (565 LOC) — note, materialize, query
- `src/paths.rs` persona module (hardcoded `personas/default/` paths)
- `~/.patina/personas/default/` directory structure
- CLI registration for `patina persona note/query/materialize/status/list`

This is a clean cut. The legacy command has no dependents — it's
standalone, not connected to Mother or the belief system. Removing
it first means the word "persona" in the codebase only ever means
the new Mother-level identity concept.

### The Registry — Mother Learns About Personas

A `persona_registry` table in graph.db (same as [[spec-data-architecture-v3]]'s
lean toward extending graph.db rather than adding new files):

```sql
CREATE TABLE persona_registry (
    uid         TEXT PRIMARY KEY,   -- UUID
    name        TEXT NOT NULL,      -- human name: "architect", "consultant"
    visibility  TEXT NOT NULL,      -- "private", "public", "shared"
    created     TEXT NOT NULL,      -- ISO 8601
    metadata    TEXT                -- JSON: description, tags, etc.
);
```

This is metadata, not storage. Mother knows who personas ARE —
names, UIDs, visibility. She doesn't store their beliefs or knowledge.
Same principle as the project registry: Mother knows where projects
are, she doesn't hold their code.

### The Provenance — Beliefs Know Their Origin

The `persona` field in belief frontmatter stops being a dead string
and starts mapping to a Mother-registered UID. When beliefs flow
between personas via Mother, provenance tracks who said what.

In graph.db, the `beliefs` table's `source` field gains a companion:
the persona UID that originated the belief. This is different from
the project that holds the belief — a belief originated by
"developer-nick" might appear in multiple projects under that persona.

### The Federation — Linking Personas Through Mother

```sql
CREATE TABLE persona_links (
    from_uid    TEXT NOT NULL,
    to_uid      TEXT NOT NULL,
    direction   TEXT NOT NULL,      -- "push", "pull", "bidirectional"
    scope       TEXT,               -- JSON: facet filters, belief IDs
    created     TEXT NOT NULL,
    UNIQUE(from_uid, to_uid)
);
```

This is the wiring diagram for Dimension 3 (continuous-operation).
Links define WHO talks to whom and WHAT flows between them.
[[spec-continuous-operation]] implements the HOW — the actual stream
delivery. This spec builds the routing table.

## Design Decisions

### 1. Migration Path for Existing Beliefs

191+ beliefs with `persona: architect`. All in one project.

**Option A: Create default persona, retroactive UID.** Generate a
UID for "architect", register it in Mother, update all belief files.

**Option B: Lazy migration.** Existing beliefs keep `persona: architect`
as a string. New beliefs get UIDs. Mother resolves both forms.

**Lean toward A.** One-time migration is cleaner than permanent
dual-resolution. `patina scrape` already reads and rewrites belief
frontmatter. Add the persona UID during a scrape cycle. The migration
is: register "architect" persona in Mother → write UID to project
config → scrape updates belief files.

### 2. Where persona_registry Lives

**Option A: In graph.db** — extend existing Mother database.
**Option B: In registry.yaml** — alongside project/repo registries.
**Option C: New personas.db** — separate concern.

**Lean toward A.** graph.db already holds federated belief data.
Persona registry is metadata FOR beliefs. It belongs with the beliefs
it describes. registry.yaml is YAML (projects, repos); persona
registry is relational (UIDs, links, queries). SQL is the right tool.

### 3. Persona ↔ Project Relationship

A persona can own multiple projects. A project belongs to exactly
one persona. This is 1:N, not N:M.

Why? [[persona-is-a-patina-instance]]: a persona IS a Patina instance.
Projects are workspaces within that instance. If two personas both
need the same project, they federate beliefs through Mother — they
don't share the project directory.

The project config (`.patina/config.toml` or equivalent) gains a
`persona_uid` field linking it to Mother's registry.

## Key Files

**Mother registries (where persona_registry goes — the core work):**
- `src/mother/graph.rs` (1,927 LOC) — graph.db schema, sync, queries
- `~/.patina/registry.yaml` — project/repo registry (YAML, stays separate)
- `~/.patina/graph.db` — federated belief search (SQL, gains persona tables)

**Belief system (provenance changes):**
- `layer/surface/epistemic/beliefs/*.md` — frontmatter `persona` field
- `src/commands/scrape/` — belief parsing, frontmatter read/write

**CLI entry points (new commands):**
- `src/commands/mother/` — `patina persona list/create/link` commands
- `src/commands/init.rs` — persona selection during `patina init`

## Open Questions

1. **Persona discovery protocol.** "Public" visibility means
   discoverable. But discoverable by whom? Other personas on the same
   machine? Other machines on the network? Discovery implies Mother
   needs a query interface for "show me public personas." The scope
   of discovery determines the networking requirements — local-only
   is simple (graph.db query), network-wide needs an entirely different
   mechanism. **Lean toward: local-only first. Same machine, same
   Mother instance.**

2. **Org as persona group.** The SPEC mentions "shared" visibility
   (org-scoped). What IS an org in Mother? A group of linked personas
   with a shared label? A separate registry? Or just a convention —
   "these personas are linked bidirectionally" = an org? **Lean toward:
   defer. Org is a label on persona_links, not a first-class entity.
   Build the linking mechanism first, add org semantics when a real
   use case emerges.**

3. **Edge app identity.** A Cloudflare Worker acting as a persona
   needs to authenticate with Mother. This requires the edge interface
   design from [[spec-continuous-operation]] — not in scope here.
   But the persona registry must accommodate non-local personas (a
   persona whose events are generated remotely). **The UID is opaque
   — it doesn't encode location. Remote personas are just personas
   Mother knows about but can't directly reach without a transport
   layer.**

4. **Cross-persona semantic search.** Mother currently does FTS5
   search across projects via `belief_search`. With personas, the
   question becomes: can you search across personas you're linked to?
   This is [[spec-mother-maturation]] DESIGN.md Dimension 2: "this
   persona in this project holds a belief similar to yours." The
   search mechanism exists (graph.db FTS5). The scoping mechanism
   (which personas can you see?) depends on visibility + linking.
   **Lean toward: search respects links. You can only see beliefs from
   personas you're linked to, filtered by the link's scope.**

