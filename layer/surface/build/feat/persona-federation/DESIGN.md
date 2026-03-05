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

The persona system is NOT greenfield. There are two distinct persona
codepaths already built — one for belief frontmatter and one for
cross-project knowledge capture.

### Persona as Belief Label

Every belief file carries `persona: architect` in its frontmatter.
All 191+ beliefs use this value. It's never varied. The SPEC.md
calls it "a dead string field" — accurate. There's no registry behind
it, no UID, no validation. It's a label with no identity.

In graph.db, the `beliefs` table has `source TEXT NOT NULL` which
tracks which project/persona contributed a belief, and the
`belief_applied_in` table tracks which projects a belief appears in.
But `source` is a project path slug, not a persona identifier.

### Persona Knowledge System (`src/commands/persona/mod.rs`, 565 LOC)

A working event-sourced knowledge capture system at
`~/.patina/personas/default/`:

| Component | Location | What it does |
|-----------|----------|-------------|
| Events (source) | `~/.patina/personas/default/events/*.jsonl` | Daily JSONL event files — `PersonaEvent` with `id`, `content`, `domains`, `supersedes` |
| Cache (derived) | `~/.patina/cache/personas/default/persona.db` | SQLite `knowledge` table — materialized from events |
| Embeddings | `~/.patina/cache/personas/default/persona.usearch` | 768-dim E5-base-v2 vectors via `usearch` |
| Paths | `src/paths.rs:79-91` | Hardcoded to `personas/default/` — both events and cache |

The system has 5 entry points:
- `note(content, domains)` — capture knowledge event to JSONL
- `materialize()` — rebuild SQLite + usearch index from events
- `query(text, limit, min_score)` — semantic search via embeddings
- `list(limit, domains)` — list recent entries
- `status()` — check oracle availability

**Key observations:**
- Hardcoded `"default"` persona in `src/paths.rs:84,89`
- Event-sourced architecture (JSONL → materialize → SQLite + vectors)
  parallels the project-level pattern (git → scrape → patina.db)
- Already uses `uuid::Uuid` for event IDs (`evt_{uuid}`)
- Already supports `domains` for facet-like filtering
- Already supports `supersedes` for knowledge evolution
- NO connection to Mother — entirely standalone
- NO connection to beliefs — separate knowledge system

### Mother's Current Registries

Mother manages registries in two places:
- `~/.patina/registry.yaml` — projects (empty) and ref repos (25 repos)
- `~/.patina/graph.db` — federated belief search (`beliefs`,
  `belief_supports`, `belief_attacks`, `belief_applied_in` tables)

Neither has a persona concept. The `beliefs.source` field in graph.db
is the closest thing — it identifies which project contributed a belief,
but not which persona.

## What Changes

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

### The Path — From Hardcoded Default to UID-Based

The `src/paths.rs` persona module currently hardcodes `"default"`:
```rust
pub fn events_dir() -> PathBuf {
    patina_home().join("personas/default/events")
}
```

This becomes parameterized by persona UID:
```
~/.patina/personas/{uid}/events/
~/.patina/cache/personas/{uid}/
```

`patina init` selects or creates a persona, writing the UID to
project config. Every project is linked to exactly one persona.

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

### 1. Persona Knowledge System — Merge or Parallel?

Two persona-related systems would coexist:
- **Belief layer** (git-backed, project-scoped) — the "product"
- **Persona knowledge** (JSONL events, cross-project) — the "oracle"

**Option A: Keep parallel.** Persona knowledge is cross-project
user preferences ("I prefer Result<T,E> over panics"). Beliefs are
project-scoped truths ("this codebase uses error-chain"). Different
concerns, different lifecycles.

**Option B: Merge.** Persona knowledge events become beliefs in a
persona-scoped belief layer. "I prefer Result<T,E>" is a belief
held by the "architect" persona, not a separate knowledge system.

**Lean toward B, eventually.** [[beliefs-are-the-product]] says
"everything else exists to support the capture, evolution, and
delivery of beliefs." Persona knowledge IS belief — it's just stored
differently right now. But the merge is a FUTURE step. This spec
adds the registry and linking. The knowledge system migration is a
separate concern — and the event-sourced architecture means no data
is lost.

### 2. Migration Path for Existing Beliefs

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

### 3. Where persona_registry Lives

**Option A: In graph.db** — extend existing Mother database.
**Option B: In registry.yaml** — alongside project/repo registries.
**Option C: New personas.db** — separate concern.

**Lean toward A.** graph.db already holds federated belief data.
Persona registry is metadata FOR beliefs. It belongs with the beliefs
it describes. registry.yaml is YAML (projects, repos); persona
registry is relational (UIDs, links, queries). SQL is the right tool.

### 4. Persona ↔ Project Relationship

A persona can own multiple projects. A project belongs to exactly
one persona. This is 1:N, not N:M.

Why? [[persona-is-a-patina-instance]]: a persona IS a Patina instance.
Projects are workspaces within that instance. If two personas both
need the same project, they federate beliefs through Mother — they
don't share the project directory.

The project config (`.patina/config.toml` or equivalent) gains a
`persona_uid` field linking it to Mother's registry.

## Key Files

**Persona system (current implementation):**
- `src/commands/persona/mod.rs` (565 LOC) — note, materialize, query, list, status
- `src/paths.rs:79-91` — hardcoded `personas/default/` paths

**Mother registries (where persona_registry goes):**
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

5. **Persona knowledge events → belief provenance.** When persona
   knowledge events (JSONL) eventually merge with the belief system,
   the event provenance becomes belief provenance. The current
   `PersonaEvent.source` field is just `"direct"`. It needs richer
   provenance — which project was the user working in? What triggered
   the capture? This parallels [[spec-data-architecture-v3]]'s event
   provenance design. **Lean toward: align persona event provenance
   with the v3 provenance model (local/external/derived) before
   merging.**
