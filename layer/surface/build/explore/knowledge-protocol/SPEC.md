---
type: explore
id: knowledge-protocol
status: complete
created: 2026-02-15
sessions:
  origin: 20260215-063600
related:
- layer/core/patina-identity.md
- layer/core/oxidized-knowledge.md
- layer/surface/build/explore/belief-mechanics/SPEC.md
- layer/surface/build/feat/mother-v2/SPEC.md
beliefs:
- patina-is-knowledge-protocol
- patina-is-knowledge-layer
- beliefs-are-entities-not-documents
references:
- "Git internals: objects, refs, packfiles"
- "Content-addressable storage (CAS)"
---

# explore: Knowledge Protocol — Content-Addressed Substrate

> Patina calls itself "git for development knowledge." Today that's metaphor,
> not mechanism. Git's power comes from content-addressed objects + refs +
> history — not from commands. This explore determines whether Patina can
> have a real git-like protocol substrate underneath its product UX, starting
> with beliefs as the proving ground.

## Problem

### The git analogy is 2/8 real

Patina shares two properties with git: layer/ distributes via git (inherited,
not earned), and there's a rough plumbing/porcelain split. The six properties
that make git actually git are missing:

| Property | Git | Patina today |
|----------|-----|-------------|
| Content-addressed | SHA = identity. Change content = new hash. | No. Human slug = identity. Mutable in place. |
| Append-only history | Commits form a DAG. Every state reachable. | Piggyback on git's history of layer/ files. |
| Branch/merge | Parallel knowledge lines reconcile. | Git merge conflicts on markdown. No semantic merge. |
| Diff | Changes expressible as diffs. Cherry-pick across branches. | No knowledge diff. Just file diffs. |
| Refs | Lightweight named pointers to objects. | Session tags and spec tags point at git commits, not knowledge states. |
| Composable primitives | 4 object types compose into everything. | Fixed pipeline (scrape -> oxidize -> search). Not composable. |

### What this costs us

Without the substrate, questions that should be trivial require archaeology:

- "What did we believe before grammar-extraction?" — grep git log
- "Did this belief change since I last saw it?" — trust the file
- "How many beliefs emerged from this session?" — count files by date
- "What's the knowledge delta between v0.20 and v0.23?" — impossible
- "Merge beliefs from another developer's branch?" — hope the markdown doesn't conflict

### The opportunity

Patina already has the **product** right — sessions, specs, beliefs, adapters,
persona. Nobody else has this workflow. What's missing is the **substrate**
that makes knowledge a first-class versionable artifact, not just files that
happen to be in git.

If the substrate works, Patina earns the git analogy. If it doesn't, Patina
is a good RAG tool with an opinionated workflow — still valuable, but not
foundational.

---

## The Substrate: Three Primitives

Git has 4 object types. Patina's knowledge reduces to 3 primitives:

### 1. Assertion

A statement about how things are or should be. Content-addressed — the hash
of the content IS the identity.

Everything in Patina is an assertion at different confidence levels:

| What we call it | Really is | Distinguishing metadata |
|-----------------|-----------|------------------------|
| Belief | Assertion | entrenchment, persona, facets |
| Pattern | Assertion | layer (core/surface/dust) |
| Spec | Assertion | exit_criteria, status, blocked_by |

### 2. Evidence

Something that supports or attacks an assertion. Evidence is also
content-addressed.

| Evidence type | Source |
|--------------|--------|
| Session | Bounded unit of work with observations |
| Commit | Git commit message + diff stats |
| Code reference | Function, type, or file that demonstrates the assertion |
| Observation | Raw data point from scrape |

### 3. Relationship

A typed edge between assertions or between assertions and evidence.

| Edge type | Meaning |
|-----------|---------|
| supports | Evidence or assertion strengthens another |
| attacks | Evidence or assertion weakens another |
| challenges | Assertion may invalidate another — review needed |
| blocks | Assertion (spec) cannot proceed until another completes |
| related | Informational link |
| derived_from | Assertion was created during this session/commit |

---

## The Anchor Move: Content-Addressed Beliefs

Beliefs are the smallest surface to prove this on. 120 files, 2 write paths,
well-defined read path.

### Current state

```
write: content -> layer/surface/epistemic/beliefs/sync-first.md
read:  scrape reads file -> eventlog -> tables -> scry
```

- Slug IS identity (mutable, can silently change content)
- Edit overwrites history (git tracks file, but Patina doesn't leverage it)
- No way to snapshot "what we believed" independent of a git commit

### Proposed state

```
write: content -> hash
       -> layer/objects/{hash}.md (immutable content)
       -> layer/refs/beliefs/sync-first (pointer to hash)
read:  scrape walks refs -> follows to objects -> eventlog -> tables -> scry
```

- Hash IS identity (immutable, content-verifiable)
- Edit creates new object, updates ref (old object survives)
- Snapshot = record all refs at a point in time

### What the file layout looks like

```
layer/
├── objects/                    # content-addressed store
│   ├── a7f3c2d1.md           # belief object (immutable)
│   ├── b3e8f912.md           # belief object (immutable)
│   └── ...
├── refs/                      # named pointers
│   ├── beliefs/
│   │   ├── sync-first        # contains: a7f3c2d1
│   │   └── compiler-enforced-safety  # contains: b3e8f912
│   └── tags/
│       └── v0.23.0           # snapshot of all refs at release
├── graph/                     # relationship edges
│   └── edges.jsonl            # append-only edge log
├── core/                      # (unchanged — eternal patterns)
├── surface/                   # (specs, build, architecture)
│   └── epistemic/beliefs/     # DEPRECATED — migrated to objects/
└── sessions/                  # (unchanged for now)
```

### The internal services (plumbing)

Five small APIs that the product commands call:

| Service | Operations | Git parallel |
|---------|-----------|-------------|
| `objects` | write(content) -> hash, read(hash) -> content, exists(hash) -> bool | `hash-object`, `cat-file` |
| `refs` | set(name, hash), get(name) -> hash, list(namespace) -> vec | `update-ref`, `show-ref` |
| `graph` | add_edge(from, to, type), edges_for(node) -> vec, walk(node, depth) | N/A (Patina-specific) |
| `snapshot` | create(name) -> freeze all refs, restore(name) -> list refs | `tag` (annotated) |
| `diff` | diff(snap_a, snap_b) -> knowledge delta | `diff` |

These live in `src/` as internal modules (not user-facing commands). The
product commands (belief, session, spec, persona) call them. Scrape reads
through them. Scry/assay are unchanged — they query the derived SQLite state.

### What changes in product commands

| Command | Change |
|---------|--------|
| `belief create` | Writes object + sets ref + adds graph edges (instead of writing markdown file) |
| `persona note` | Same, but into user-level object store (~/.patina/layer/objects/) |
| `scrape layer` | Walks refs to find objects (instead of globbing belief files) |
| `session end` | Snapshots current refs as session boundary |
| `spec status` | (future) Writes spec object + updates ref |

Scrape, oxidize, scry, assay, context are **unchanged**. They read from
SQLite tables which are materialized from the eventlog. The object store
feeds the eventlog through scrape, same as today. The substrate change is
below the scrape layer.

### What you get immediately

- **Knowledge diff**: `patina diff v0.20.0 v0.23.0` — what did we learn?
- **Integrity**: Hash verifies belief hasn't drifted
- **History**: Old objects survive edits (git tracks them too, but now Patina
  knows about its own history)
- **Session-scoped knowledge**: Snapshot at session boundaries shows what
  emerged from each work session
- **Foundation for merge**: Content-addressed objects are the prerequisite
  for knowledge reconciliation (not built yet, but unblocked)

### What you DON'T build yet

- **Merge** — No multi-user scenario today. Protocol supports it, product
  doesn't need it yet.
- **Cross-project federation** — Mother v2 can use content-addressed objects
  for federated belief search, but that's a separate spec.
- **Knowledge branches** — One knowledge state per project is fine for now.
- **Spec/session objects** — Prove on beliefs first, extend later.
- **Packfiles** — Git packs objects for efficiency. 120 beliefs don't need it.

---

## Exploration Questions

### 1. Does content addressing actually work for beliefs?

Beliefs have mutable metadata (entrenchment changes, status changes). Options:

- **Hash content only, not metadata** — Metadata lives in the ref or a sidecar.
  Same content = same hash regardless of entrenchment. This means the "assertion"
  is stable, the "confidence" evolves separately.
- **Hash everything** — Any metadata change = new object. Full history but
  more objects. Entrenchment going medium -> high creates a new object.
- **Separate content object + metadata ref** — Content in objects/, metadata
  in refs/. Cleanest separation but more moving parts.

**Leaning:** Hash content only. The assertion "prefer sync over async" is the
same assertion whether entrenchment is medium or high. Confidence is about the
ref, not the object.

### 2. What's the hash function?

- **SHA-256 truncated to 8 hex chars** (like Patina's existing UID system) —
  Collision risk is real at 120 objects (birthday bound ~65K for 32-bit).
- **SHA-256 truncated to 16 hex chars** — Collision-safe for millions of
  objects. Still readable as filenames.
- **Full SHA-256** — Maximum safety but 64-char filenames are painful.

**Leaning:** 16 hex chars. Matches git's short-hash UX while being safe.

### 3. How does scrape find objects?

Today scrape globs `layer/surface/epistemic/beliefs/*.md`. After:

- **Walk refs** — scrape reads refs/beliefs/*, follows each to objects/,
  reads the object. Only indexes what's currently pointed to.
- **Glob objects** — scrape reads all objects/ files. Indexes everything,
  including superseded versions. Richer history but bigger index.

**Leaning:** Walk refs (only current state). Historical objects are available
for diff but don't pollute the search index. You search what you believe NOW,
not what you used to believe.

### 4. Migration path from current belief files?

120 existing beliefs need to move. This should be:

- Idempotent (safe to run multiple times)
- Reversible (can go back to flat files if the experiment fails)
- `patina rebuild` compatible (can reconstruct objects/ from git history)

A migration command: `patina belief migrate` — reads current belief files,
writes objects + refs, verifies roundtrip integrity.

### 5. Does the graph need its own store?

Beliefs already have `supports` and `attacks` in their YAML frontmatter.
Should relationships move to a separate graph store?

- **Keep in frontmatter** — Relationships travel with the object. Simple.
  But means the graph is scattered across files.
- **Separate graph** — `layer/graph/edges.jsonl` is an append-only edge log.
  Graph queries are fast. But relationships are in two places.
- **Both** — Frontmatter is the source, graph store is derived (like SQLite).

**Leaning:** Both. Frontmatter is authoritative (travels with the object in
git). Graph store is derived by scrape for fast queries. Same pattern as
eventlog -> tables.

### 6. Does this subsume belief-mechanics?

The [[belief-mechanics]] explore proposed `speculative` entrenchment and
`challenged` status. Content addressing gives challenged/speculative beliefs
a cleaner model:

- A **challenge** is a new assertion object with a `challenges` edge to the
  target. The target's ref doesn't change — the challenge exists alongside it.
- A **speculative** belief is an assertion object with minimal evidence edges.
  Entrenchment is metadata, not content.

If this explore succeeds, belief-mechanics becomes a product feature built on
the protocol substrate rather than a standalone schema change.

---

## Exit Criteria

- [x] Determine if content addressing works for beliefs (question 1-3) — **No.** Slug is identity, not hash. Beliefs are meant to be mutable.
- [x] Design the object format — **N/A.** Content addressing rejected.
- [x] Design the ref format — **N/A.** Git tags already serve as refs.
- [x] Design the migration path — **N/A.** No migration needed.
- [ ] Prototype `patina diff` against two knowledge snapshots — **Deferred.** Valid command, doesn't need substrate. Standalone feat spec.
- [x] Determine if this is a real protocol or over-engineering for 120 files — **Over-engineering.** Git already provides the substrate.
- [x] If not real: document why and what alternative serves the same needs — **See Findings below.**

---

## Findings (Session 20260215-075638)

### Outcome: C — the git analogy doesn't hold at the storage layer

Read all code paths: `create-belief.sh` (write), `scrape/beliefs/mod.rs` (read),
`persona/mod.rs` (persona write), `scrape/layer/mod.rs` (router), `paths.rs`
(filesystem layout). 120 belief files examined in context.

### Why content addressing fails for beliefs

**Belief identity is the slug, not the hash.** In git, changing a byte creates
a new object — that's the entire point. In beliefs, you WANT to change content
(add evidence, refine the statement, update relationships) while preserving
identity. The slug `sync-first` is stable across all edits. A hash would
change on every evidence addition, creating meaningless object churn.

Belief content decomposes into assertion (rarely changes) and everything else
(changes constantly — evidence, relationships, entrenchment, metrics, applied-in).
Even hashing "just the assertion" fails because rewording for clarity should not
create a new identity. The slug already handles this correctly.

### Why refs are redundant with git

Git tags already serve as knowledge snapshots:
- Session tags: `session-20260215-075638-claude-start`
- Release tags: `v0.23.0`

Each tag points to a commit containing the belief files in their state at that
moment. `git show v0.20.0:layer/surface/epistemic/beliefs/` gives you the
exact knowledge state at v0.20. No filesystem refs needed.

### What IS genuinely missing (and doesn't need a substrate)

1. **`patina diff` command** — "What did we learn between v0.20 and v0.23?"
   Implementable with `git2` crate: read belief files at two tags, parse
   frontmatter, compute delta (new/changed/removed beliefs, relationship
   changes, entrenchment shifts). Pure Rust, no filesystem changes.

2. **Queryable relationship graph** — Supports/Attacks/Attacked-By are
   embedded in markdown sections but not efficiently queryable. A
   `belief_edges` table in SQLite (populated by scrape) enables graph walks:
   "find all beliefs that attack X", "walk the support chain from Y".

3. **Session provenance tracking** — Which session created which belief?
   Partially exists (evidence links embed session IDs) but not materialized
   as a first-class relationship in scrape.

### Every proposed service has a git-native equivalent

| Proposed | Git-native |
|----------|-----------|
| `objects.write(content) -> hash` | `git add` hashes files |
| `refs.set(name, hash)` | Git tags (session + release) |
| `snapshot.create(name)` | `git tag -a vX.Y.Z` (already done) |
| `diff(snap_a, snap_b)` | NEW: `patina diff` command using git2 |
| `graph.add_edge(from, to, type)` | Markdown sections + scrape materialization |

### The protocol/product split is real — it just lives at a different layer

The five protocol verbs (capture, index, search, believe, evolve) are the
real protocol. They don't need a custom storage substrate because git IS
the storage substrate. The protocol is the pipeline (scrape → oxidize →
scry/assay/context), not the filesystem layout.

---

## Possible Outcomes

### Outcome A: The substrate is real — write feat spec

Content-addressed beliefs prove out. The internal services (objects, refs,
graph, snapshot, diff) work. Knowledge diff is useful. Write a phased feat
spec to implement the substrate and migrate beliefs, then extend to specs
and sessions.

### Outcome B: Content addressing is overkill, but refs are valuable

The hashing adds complexity without proportional value (120 beliefs don't
need integrity verification). But named refs and snapshots are genuinely
useful for knowledge diffing. Build a lighter "refs-only" system where
beliefs keep their current format but get snapshot refs at session/release
boundaries.

### Outcome C: The git analogy doesn't hold — stay the course

The substrate adds architecture without solving real problems. Patina is
a good product with an opinionated workflow. The value is in sessions,
specs, beliefs, and adapters — not in the storage mechanism. Focus on
the product, extract tooling to plugins, and stop chasing the git metaphor.

All three outcomes are valid. The explore's job is to find out which is true.

---

## References

- [[patina-is-knowledge-protocol]] — Current identity as protocol
- [[patina-is-knowledge-layer]] — Git-style substrate framing
- [[beliefs-are-entities-not-documents]] — Beliefs as first-class objects
- [[belief-mechanics]] — Challenge/speculative extensions (may be subsumed)
- `layer/core/patina-identity.md` — The Protocol Test
- `layer/core/oxidized-knowledge.md` — Knowledge separation model
- Git internals: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-15 | ready | Emerged from session deep dive into core values, spec landscape, and the question "is git-for-knowledge real or smoke?" Anchored by outside analysis: content-addressed objects + refs is the minimal substrate that earns the git analogy. Beliefs are the proving ground. |
| 2026-02-15 | complete | **Outcome C.** Read all belief write/read/scrape code paths. Content addressing fails because belief identity is the slug, not the hash — beliefs are meant to be mutable. Git already provides history, diffing, refs (tags), and snapshots. The real missing capabilities (knowledge diff, queryable graph, session provenance) are commands and scrape enhancements, not a filesystem substrate. |
