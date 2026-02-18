---
type: feat
id: patina-polymorphic-extraction
status: draft
created: 2026-02-17
sessions:
  origin: 20260217-081150
  review-1: 20260217-070309
  review-2: 20260217-164506
related:
  - fact-schema-registry
  - fact-crdt-substrate
beliefs:
  - unix-philosophy
  - dependable-rust
  - adapter-pattern
  - patina-identity
---

# feat: Polymorphic Extraction — Extensible Pipeline Plugin Contract

> The pipeline plugin system (v0.17.0) lets people extend Patina beyond its
> core scope. But the pipeline world only speaks code — `ExtractedData` has
> 7 code-specific fields and the host hardcodes deserialization to that one
> type. A plugin author who wants to capture issues, email, or any non-code
> artifact literally can't. The capture verb's extension point is broken.
>
> This spec fixes the contract: plugins declare what *kind* of data they
> produce, the host routes by kind. Forge migrates as proof.

## Problem

Patina's protocol is **capture, index, search, believe, evolve**. The
plugin system exists so people can extend these verbs beyond Patina's core
scope ([[patina-identity]]). Pipeline plugins extend *capture* — but today
they can only capture code.

Three things are hardcoded:

1. **`ExtractedData`** — the only type a pipeline plugin can return. 7
   code-specific fields (symbols, functions, types, imports, call_edges,
   constants, members). A plugin that wants to return issue metadata or
   email headers has no way to express it.

2. **`Language` enum** — only recognizes programming language file
   extensions. Files that aren't source code never reach plugin dispatch.

3. **Response deserialization** — the host always does
   `serde_json::from_str::<ExtractedData>()`. There's no dispatch by
   payload kind.

This means the pipeline world (the first and most mature plugin world)
can't be used for its intended purpose: letting people extend capture to
their domain.

**Evidence the contract is broken:** Forge (GitHub/Gitea integration)
captures development artifacts (issues, PRs) but bypasses the pipeline
entirely — it calls `gh` CLI directly, writes to the eventlog, creates
its own materialized views, and populates FTS5. All outside the plugin
system. Forge had no choice: the pipeline couldn't accept its data.

## What Exists Today (Reuse Map)

| Need | Already Have | Where |
|------|-------------|-------|
| Issue/PR domain types | `Issue`, `PullRequest`, `Comment` structs | `src/forge/types.rs` |
| Platform abstraction | `ForgeReader` trait (GitHub, Gitea) | `src/forge/mod.rs` |
| Issue/PR tables | `forge_issues`, `forge_prs`, `forge_refs` | `src/commands/scrape/forge/` |
| FTS5 for issues/PRs | `code_fts` with `forge.issue`/`forge.pr` event_types | `src/commands/scrape/forge/` |
| Eventlog as raw store | All scrapers write events → eventlog | `src/eventlog/` |
| WASM plugin pipeline | Plugins claim languages, produce JSON responses | `src/commands/scrape/code/` |
| Plugin dispatch | Host routes files to plugins by extension | `src/commands/scrape/code/extract_v2.rs` |
| Embedding offsets | `FORGE_ID_OFFSET = 5_000_000_000` (enrichment exists) | `src/commands/oxidize/`, `src/commands/scry/` |

**The pipeline infrastructure exists. The plugin contract is the gap.**

## Spec Staircase

This spec is the first step in a three-spec chain. Each step assumes the
previous one left the right seams:

```
1. polymorphic-extraction  →  open the pipeline contract beyond code
2. fact-schema-registry    →  make fact types declarative (WIT schemas)
3. fact-crdt-substrate     →  make fact storage sync-able via Mother
```

The tagged union introduced in Phase A is a **bridge**, not a destination.
[[fact-schema-registry]] will generate variants from schema metadata instead
of hard-coding them. [[fact-crdt-substrate]] will route facts into a CRDT
store before materializing into SQLite. The host's routing match (Phase B)
will evolve from a hand-written `match` to schema-driven dispatch.

Decisions in this spec are shaped by what comes next:

- The tagged union must stay open so schema-defined kinds can land without
  binary edits (don't seal the enum).
- The host handles all impure concerns (dedup, storage, FTS5, eventlog) so
  plugins remain pure compute — this split is what makes WASM viable and
  what the CRDT substrate assumes.
- Staging under `.patina/local/data/` aligns with the CRDT spec's replica
  path (`.patina/local/data/facts/`) and the existing `paths::project::data_dir`.

This spec focuses on the minimal plumbing so schema/CRDT work can land
incrementally without rewrites.

## What To Build

### Phase A: Polymorphic `ExtractedPayload`

Define the tagged union as a **bridge type**. Start with `Code` + `Issue` +
`PullRequest` — enough to prove the contract. This enum will be superseded
by schema-generated variants once [[fact-schema-registry]] lands; keep it
`#[non_exhaustive]` so downstream code doesn't assume a closed set.

```rust
/// Pipeline plugins return JSON matching one of these variants.
/// If no `kind` field is present, defaults to Code (backward compat).
///
/// Bridge type: will be superseded by schema-generated variants
/// once [[fact-schema-registry]] lands. Keep #[non_exhaustive].
#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind")]
pub enum ExtractedPayload {
    #[serde(rename = "code")]
    Code(ExtractedData),

    #[serde(rename = "issue")]
    Issue(ExtractedIssue),

    #[serde(rename = "pull-request")]
    PullRequest(ExtractedPullRequest),
}
```

`ExtractedIssue` and `ExtractedPullRequest` mirror `src/forge/types.rs`
structs but are pipeline-facing (no `ForgeReader` dependency). They may
be the same types re-exported, or thin wrappers — decide during
implementation based on what `serde(tag = "kind")` requires.

**Backward compatibility:** Host deserializes as:
1. Try `serde_json::from_str::<ExtractedPayload>()` — if JSON has `kind`
2. If no `kind` field, try `serde_json::from_str::<ExtractedData>()` → wrap
   as `ExtractedPayload::Code`
3. If both fail, fall through to built-in processor (existing behavior)

**Zero changes to existing pipeline plugins.** They keep returning the
same JSON without a `kind` field.

**Code paths:**
- `src/commands/scrape/code/extract_v2.rs` — response deserialization
- New file or extend `src/commands/scrape/code/` — `ExtractedPayload` enum

### Phase B: Kind-Based Routing in Host

After deserializing `ExtractedPayload`, route to the correct insert
function:

```rust
match payload {
    ExtractedPayload::Code(data) => insert_code_facts(conn, &data),
    ExtractedPayload::Issue(issue) => insert_forge_issue(conn, &issue),
    ExtractedPayload::PullRequest(pr) => insert_forge_pr(conn, &pr),
}
```

The `insert_forge_issue` / `insert_forge_pr` functions write to the
**existing** `forge_issues` / `forge_prs` tables + eventlog with
`forge.issue` / `forge.pr` event_types. Same tables, same schema, same
FTS5 — just a different entry point.

**Code paths:**
- `src/commands/scrape/code/extract_v2.rs` — add match dispatch after
  deserialization
- `src/commands/scrape/forge/database.rs` — extract insert functions into
  reusable form (currently coupled to forge's bulk-fetch flow)

### Phase C: Forge Connector Separation

Split forge into two concerns per [[unix-philosophy]]:

1. **Connector** (fetching): `ForgeReader` trait implementations call
   `gh` CLI / Gitea API, fetch issues/PRs, write JSON files to a staging
   directory. This is the "messy" part — auth, rate limiting, incremental
   sync, background fork, PID files.

2. **Pipeline plugin** (extraction): A plugin claims the staged file
   format, parses it, returns `ExtractedPayload::Issue` or
   `ExtractedPayload::PullRequest`. This goes through the normal scrape
   pipeline.

**Staging format:** Forge connector writes one JSON file per issue/PR to
the canonical derived-data tree: `.patina/local/data/forge/{owner}-{repo}/issues/123.json`,
`.../prs/456.json` (via `paths::project::data_dir`). This aligns with the
CRDT spec's replica path and keeps forge artifacts under the same
rebuildable/replicable contract as the DB. The pipeline plugin reads
these files during `patina scrape`.

**Two-verb flow:** `patina scrape forge` = fetch to staging (connector).
`patina scrape` = process staging alongside source files (pipeline).
No extra flag needed — `extract_v2` learns to scan the staging tree
automatically, the same way it walks the source tree today.

**Incremental:** The connector already tracks `since` timestamps and
`forge_refs` backlog. This doesn't change — it still controls *what gets
fetched*. The pipeline controls *what gets indexed*.

**The `ForgeReader` trait and GitHub/Gitea implementations stay as-is.**
They just write to disk instead of directly to the DB.

**Code paths:**
- `src/commands/scrape/forge/mod.rs` — change `run()` to write JSON files
  to `paths::project::data_dir().join("forge")` instead of calling
  `insert_issues()` / `insert_prs()` directly
- New plugin: `grammar-forge` — claims staged JSON format (e.g., `.forge-issue`,
  `.forge-pr` extensions on staged files), returns `ExtractedPayload::Issue` /
  `ExtractedPayload::PullRequest`
- `src/commands/scrape/code/extract_v2.rs` — extend file discovery to scan
  the staging tree under `data_dir()` alongside the source tree

### Phase D: Verify End-to-End

Prove the contract works and forge migration is complete:

1. `patina scrape forge` fetches issues/PRs to staging directory
2. `patina scrape` processes staged files through pipeline → writes to
   `forge_issues` / `forge_prs` tables
3. `patina assay --include-issues "bug"` returns same results as before
4. FTS5 `code_fts` entries with `forge.issue` / `forge.pr` event_types
   are populated
5. Existing code plugins still work unchanged

## What Doesn't Change

- **`forge_issues` / `forge_prs` table schemas** — same tables, same columns
- **FTS5 integration** — same `code_fts` table, same event_types
- **`ForgeReader` trait** — GitHub/Gitea implementations unchanged
- **Existing pipeline plugins** — no `kind` field = `Code`, backward compat
- **`forge_refs` / sync engine** — PR ref discovery from commits unchanged
- **Eventlog** — still the raw event store for all data
- **Scry/Assay query interface** — no changes needed
- **Embedding offsets** — `FORGE_ID_OFFSET` stays at 5B

## Exit Criteria

1. Existing pipeline plugins continue working unchanged (no `kind` = `Code`)
2. A pipeline plugin returning `{"kind": "issue", ...}` writes to
   `forge_issues` table + eventlog
3. `patina scrape forge` writes JSON files to staging directory
4. `patina scrape` processes staged forge files through the pipeline
5. `patina assay --include-issues "bug"` returns same results as before
   migration
6. `cargo test --workspace` passes
7. `./resources/git/pre-push-checks.sh` passes

## Non-Goals

- Defining new data types beyond Code/Issue/PR — that's [[fact-schema-registry]]
- Changing the WIT interface (returns string — JSON schema is the
  extension point)
- Building external connectors (gh-sync, slack-export, etc.)
- Renaming `code_fts` to `fts` — follow-up if needed
- Embedding forge data in oxidize (enrichment code exists but corpus
  isn't built — separate concern)
- Schema-driven kind registration — deferred to [[fact-schema-registry]]
- CRDT storage or Mother sync — deferred to [[fact-crdt-substrate]]
- Real-time sync / push notifications

## Plugin Author Extension Pattern

**With this spec only (bridge path):** A plugin author extends capture by:

1. Add variant to `ExtractedPayload` enum (e.g., `Email(ExtractedEmail)`)
2. Define the struct with the fields their domain needs
3. Add a table schema for the materialized view
4. Add insert routing in the host match
5. Write a pipeline plugin that claims their file type and returns the
   new kind
6. Write a connector if their data source needs fetching (API, sync, etc.)

Steps 1-4 require binary changes. This is acceptable as a bridge — forge
proves the contract works end-to-end.

**With [[fact-schema-registry]] (target path):** Steps 1-4 are replaced
by a single schema declaration (`patina schema new <kind>`). The host
auto-generates structs, tables, and routing from the schema. Plugin
authors only write steps 5-6. No binary changes needed for new fact types.

## Resolved Decisions

**Staging directory:** `.patina/local/data/forge/{owner}-{repo}/` — under
`paths::project::data_dir`, aligned with the CRDT spec's replica path and
the canonical derived-data contract. Not a bespoke `.patina/local/forge/`.

**WASM vs built-in:** WASM pipeline plugin (`grammar-forge`). The host
handles all impure concerns (dedup, storage, FTS5, eventlog). The plugin
is pure compute: parse staged JSON → emit typed payload. This split is
what the staircase assumes — [[fact-crdt-substrate]] will own persistence,
[[fact-schema-registry]] will own validation, and the plugin stays in the
pipeline world with `log` as its only capability.

**Auto-include vs flag:** Auto-include, no flag. `patina scrape forge`
fills the staging directory. `patina scrape` scans it alongside source
files. Two verbs, one pipeline. `extract_v2` learns to walk the staging
tree under `data_dir()` — deterministic end-to-end proof without a second
toggle.

**Dynamic kind registration:** Deferred to [[fact-schema-registry]], which
will generate host code and routing metadata from WIT schema packages. The
tagged union here is a bridge; the enum is `#[non_exhaustive]` so
schema-defined kinds can land without breaking the host match.
