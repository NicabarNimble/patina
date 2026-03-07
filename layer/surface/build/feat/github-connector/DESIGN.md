# Design: GitHub Connector — Proving the Native Child Pattern

## Why This Work Exists

The github-connector is the first native child. It exists to prove
three things at once:

1. **The native child pattern works.** A real connector, talking to a
   real API, using the Child trait and `run()` from patina-pipe. If
   this works, every future connector (Slack, RSS, Jira) follows the
   same pattern.

2. **Domain code leaves core.** [[patina-is-domain-agnostic-knowledge-system]]
   says "plugins determine what domain Patina operates in." The forge
   code (`src/forge/`, 2,216 LOC + `src/commands/scrape/forge/`, 705
   LOC) is GitHub-specific knowledge compiled into every Patina binary.
   A law firm doesn't need it. This connector extracts it.

3. **Schema namespaces separate cleanly.** The connector emits
   `github.*` facts, not `forge.*`. The WASM forge plugin continues
   to emit `forge.*`. Both coexist under mother-broker, proving
   dual-runtime operation. After parity is verified, forge is deleted.

**Origin:** [[session-20260306-174214]] (audit: connectors use their
own schema namespaces — `github.*` not `forge.*`),
[[session-20260305-170212]] (forge extraction session: deep audit of
what exists before extraction), [[session-20260306-061745]] (pipes are
processes: github-connector is a normal Rust binary with reqwest).

## The Migration: What Changes, What Stays

The domain logic — pagination, JSON parsing, data shapes, rate limit
handling — stays identical. Only the I/O boundary changes.

| Concern | Old (WASM via host functions) | New (native via reqwest) |
|---------|------------------------------|-------------------------|
| HTTP calls | `host_http::get(url)?` | `self.get(url)?` (reqwest::blocking) |
| Fact emission | `host_emit::emit_fact("forge", "issue", &json)` | `emitter.emit("github", "issue", &value)?` |
| Logging | `host_log::log(Level::Info, msg)` | `eprintln!("[github] {}", msg)` |
| Error types | `Result<_, String>` | `Result<_, PipeError>` |
| Rate limiting | `crate::rate_limit_sleep()` | `std::thread::sleep(Duration::from_millis(750))` |
| Schema namespace | `forge.*` | `github.*` |
| Auth delivery | Host-injected via capability grants | `pipe/initialize` params |

The JSON shapes for issues and PRs are identical. The API response
structs (`GhApiIssue`, `GhApiPullRequest`, `GhApiComment`,
`GhApiReview`) migrate unchanged.

## Design Decisions

### 1. github.* Schema, Not forge.*

The Session 12 audit established this: connectors own their schema
namespace. "forge" was the WASM plugin name, not the data source. The
data source is GitHub. The schema is `github.*`.

This means `github.issue` and `github.pr` are distinct event types
from `forge.issue` and `forge.pr`. Both can coexist in events.db.
Parity verification compares the data shapes to confirm they produce
equivalent knowledge.

### 2. HTTP Error Mapping to PipeError

GitHub API errors map cleanly to PipeError variants:

| HTTP Status | PipeError Variant | Rationale |
|-------------|-------------------|-----------|
| 200 | Ok | Success |
| 401, 403 | Check body for rate limit | Could be auth failure OR rate limit |
| 401, 403 (not rate limit) | Fatal | Bad credentials, revoked token |
| 404 | Fatal | Repo not found, wrong owner/repo |
| 429 | RateLimited (retry 60s) | GitHub explicit throttle |
| 5xx | Transient (retry 5s) | Server error, usually recovers |
| Other | Transient | Unknown, retry with backoff |

The 401/403 ambiguity is GitHub-specific: rate limit exhaustion
returns 403 with a body containing "rate limit". The connector checks
the body before deciding Fatal vs RateLimited.

### 3. reqwest::blocking, Not async

Matches the codebase's sync-first position. The Child trait is sync.
The connector makes sequential paginated API calls. No benefit from
async here — each page depends on the previous response (link headers,
cursor). If concurrent page fetching becomes needed, a local tokio
runtime inside `fetch()` is the escape hatch.

### 4. Cursor via Wall Clock Time

The connector uses `chrono::Utc::now().to_rfc3339()` as the cursor.
GitHub's `/issues` endpoint accepts a `since` parameter (ISO 8601) to
filter by `updated_at`. This gives incremental fetching: each run
fetches only issues updated since the last run.

Alternative: use the latest `updated_at` from fetched items as the
cursor. More precise but requires tracking state across the fetch.
Wall clock is simpler and the overlap is handled by dedup.

## github.* Schema Definition

```toml
# .patina/schemas/github/schema.toml

[schema]
name = "github"
version = "1.0.0"
package = "patina:schema/github@1.0.0"
description = "GitHub issues and pull requests (native connector)"

[[facts]]
name = "issue"
event_type = "github.issue"
record = "issue"

[[facts]]
name = "pull-request"
event_type = "github.pr"
record = "pull-request"

[embedding]
offset_slot = 6           # next available after forge (slot 5)
corpus_query = """
SELECT seq,
       json_extract(data, '$.title') || ' ' ||
       COALESCE(json_extract(data, '$.body'), '')
       as content
FROM eventlog
WHERE event_type LIKE 'github.%'
"""

[[indexes]]
fact = "issue"
fts_fields = ["title", "body"]
table = "github_issues"

[[indexes]]
fact = "pull-request"
fts_fields = ["title", "body", "comments"]
table = "github_prs"
```

Schema ships with the connector. Manual installation during development
(copy to `.patina/schemas/github/`). Automatic installation from
child.toml is mother-broker scope.

## Parity Verification Plan

Before deleting `src/forge/`, prove the new connector produces
equivalent knowledge:

1. Run `patina scrape forge` against a test repo, capture events
   where `event_type LIKE 'forge.%'`
2. Run `patina mother run github` against same repo, capture events
   where `event_type LIKE 'github.%'`
3. Compare data shapes:

```sql
-- Should produce identical rows (modulo event_type prefix)
SELECT json_extract(data, '$.number'),
       json_extract(data, '$.title'),
       json_extract(data, '$.state')
FROM eventlog WHERE event_type = 'forge.issue'
ORDER BY json_extract(data, '$.number');
```

### Expected Differences

| Field | forge.* | github.* | Why |
|-------|---------|----------|-----|
| event_type | forge.issue | github.issue | Schema namespace change |
| source_id | plugin:patina-forge | child:github-connector | Source type change |
| content_hash | (absent) | blake3:... | New capability |
| Data shape | Identical | Identical | Same API, same parsing |

## src/forge/ Deletion Checklist

After parity is verified. Total: 2,921 LOC removed from core.

| File | LOC | Safe to delete? |
|------|-----|-----------------|
| `src/forge/mod.rs` | 181 | Yes, after scrape forge removed |
| `src/forge/types.rs` | 80 | Yes |
| `src/forge/writer.rs` | 231 | Yes |
| `src/forge/github/mod.rs` | 66 | Yes |
| `src/forge/github/internal.rs` | 376 | Yes |
| `src/forge/sync/mod.rs` | 93 | Yes |
| `src/forge/sync/internal.rs` | 615 | Yes |
| `src/forge/none.rs` | 41 | Yes |
| `src/commands/scrape/forge/mod.rs` | 705 | Yes, remove subcommand |

**Pre-deletion checks:**
1. `grep -r "forge::" src/ --include="*.rs"` — find all imports
2. Remove `mod forge;` from declaration
3. Remove forge subcommand from `src/commands/scrape/mod.rs`
4. `cargo build --release` — verify clean compile
5. Keep `plugins/forge/` — WASM plugin stays (proves dual runtime)

## Crate Structure

```
children/github-connector/
  Cargo.toml              # patina-pipe, patina-pipe-types, reqwest, serde
  child.toml              # manifest for Mother
  src/
    main.rs               # Child trait impl + main()
    github.rs             # GitHub REST API client (migrated from forge)
```

## What's NOT In Scope

- **WASM forge plugin changes** — plugins/forge/ continues to work
  with `forge.*` schema. Both runtimes coexist under mother-broker.
- **Automatic schema installation** — mother-broker scope. Manual copy
  during development.
- **Multi-instance support** — one github-connector binary handles
  any GitHub repo via `params.owner` and `params.repo`. No need for
  separate binaries per repo.
- **GitHub API v4 (GraphQL)** — the connector uses v3 REST, matching
  the existing forge code. GraphQL migration is future optimization.

## Belief Anchors

- [[patina-is-domain-agnostic-knowledge-system]] — GitHub knowledge
  doesn't belong in core. A law firm installing Patina shouldn't
  compile GitHub API types.
- [[pipes-are-processes-not-wasm]] — the connector is a normal Rust
  binary. `cargo run`, `cargo test`, `dbg!()`, reqwest, the full
  ecosystem. No WASM toolchain needed.
- [[host-proxied-io-is-the-security-model]] — OS sandbox restricts
  the connector to `api.github.com` only. Credentials arrive via
  `pipe/initialize`, not environment variables.

## Open Questions

1. **chrono dependency.** The cursor uses `chrono::Utc::now()`. chrono
   is in the main binary's dep tree but not the connector's. Add it,
   or use a simpler approach (pass back the latest `updated_at` from
   fetched items as the cursor)?

## Commits

1. `github-connector: create binary crate with Child trait impl` —
   children/github-connector/ with Cargo.toml, child.toml, main.rs.

2. `github-connector: migrate GitHub REST API client` — github.rs
   migrated from plugins/forge/src/github.rs. Replace host_http with
   reqwest, host_emit with emitter.emit(), errors with PipeError.

3. `github-connector: add github.* schema definition` —
   .patina/schemas/github/schema.toml with fact types.

4. `github-connector: wire patina mother run github` — Mother-side
   spawn, pipe/initialize with credentials, pipe/fetch, pipe/shutdown.

5. `github-connector: parity verification` — Run both connectors,
   compare data shapes. Document results.

6. `forge: delete src/forge/ (2,216 LOC) and scrape forge command
   (705 LOC)` — Remove old code. Keep plugins/forge/ (WASM).

## Key Files

- `children/github-connector/src/main.rs` — Child trait impl
- `children/github-connector/src/github.rs` — REST API client
- `children/github-connector/child.toml` — manifest for Mother
- `.patina/schemas/github/schema.toml` — schema definition
- `plugins/forge/src/github.rs` — migration source (450 LOC)
- `src/forge/` — deletion target (2,216 LOC)
- `src/commands/scrape/forge/mod.rs` — deletion target (705 LOC)
