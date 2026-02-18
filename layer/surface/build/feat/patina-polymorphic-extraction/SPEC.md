---
type: feature
id: patina-polymorphic-extraction
status: draft
created: 2026-02-17
sessions:
  origin: 20260217-081150
  review-1: 20260217-070309
related:
  - belief-graph
beliefs:
  - unix-philosophy
  - dependable-rust
  - adapter-pattern
  - patina-identity
---

# feature: Polymorphic Extraction — Extensible Pipeline Plugin Contract

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

## What To Build

### Phase A: Polymorphic `ExtractedPayload`

Define the tagged union. Start with `Code` + `Issue` + `PullRequest` —
enough to prove the contract, not more.

```rust
/// Pipeline plugins return JSON matching one of these variants.
/// If no `kind` field is present, defaults to Code (backward compat).
#[derive(Debug, Deserialize)]
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
a known directory (e.g., `.patina/local/forge/{owner}-{repo}/issues/123.json`,
`.../prs/456.json`). The pipeline plugin reads these files during
`patina scrape`.

**Incremental:** The connector already tracks `since` timestamps and
`forge_refs` backlog. This doesn't change — it still controls *what gets
fetched*. The pipeline controls *what gets indexed*.

**The `ForgeReader` trait and GitHub/Gitea implementations stay as-is.**
They just write to disk instead of directly to the DB.

**Code paths:**
- `src/commands/scrape/forge/mod.rs` — change `run()` to write JSON files
  instead of calling `insert_issues()` / `insert_prs()` directly
- New plugin: `grammar-forge` — claims staged JSON format, returns
  `ExtractedPayload::Issue` / `ExtractedPayload::PullRequest`
- `src/commands/scrape/code/extract_v2.rs` — ensure staged directory is
  included in file discovery

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

- Defining new data types (email, calendar, etc.) — plugin authors do
  that by adding variants to `ExtractedPayload` and writing plugins
- Changing the WIT interface (returns string — JSON schema is the
  extension point)
- Building external connectors (gh-sync, slack-export, etc.)
- Renaming `code_fts` to `fts` — follow-up if needed
- Embedding forge data in oxidize (enrichment code exists but corpus
  isn't built — separate concern)
- Real-time sync / push notifications

## Plugin Author Extension Pattern

Once this spec is complete, a plugin author extends capture to their
domain by:

1. Add variant to `ExtractedPayload` enum (e.g., `Email(ExtractedEmail)`)
2. Define the struct with the fields their domain needs
3. Add a table schema for the materialized view
4. Add insert routing in the host match
5. Write a pipeline plugin that claims their file type and returns the
   new kind
6. Write a connector if their data source needs fetching (API, sync, etc.)

Each step is small, isolated, and follows the pattern established by
forge. The plugin author owns the domain knowledge. Patina owns the
pipeline contract.

## Open Questions

- Should the staging directory be `.patina/local/forge/` or somewhere
  else? It needs to survive between `scrape forge` (fetch) and
  `scrape` (index) runs.
- Should the forge plugin be a WASM plugin (`grammar-forge`) or a
  built-in Rust handler? WASM is consistent with the architecture but
  adds compilation overhead for JSON parsing. Built-in is simpler for
  the proof.
- Should `patina scrape` auto-include the staging directory, or require
  explicit `patina scrape --include-forge`? Auto-include is simpler.
- Steps 1-4 of the extension pattern require changes to the Patina
  binary. Should the host eventually support dynamic kind registration
  (plugin declares its schema at load time) to eliminate binary changes?
  That's a future spec if needed.
