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
processes: github-connector is a normal Rust binary with pipe/http).

## The Migration: What Changes, What Stays

The domain logic — pagination, JSON parsing, data shapes, rate limit
handling — stays identical. Only the I/O boundary changes.

| Concern | Old (WASM via host functions) | New (native via pipe/http) |
|---------|------------------------------|---------------------------|
| HTTP calls | `host_http::get(url)?` | `io.get(url).send()?` (pipe/http, proxied through Mother) |
| Fact emission | `host_emit::emit_fact("forge", "issue", &json)` | `io.emit("github", "issue", &value)?` |
| Logging | `host_log::log(Level::Info, msg)` | `eprintln!("[github] {}", msg)` |
| Error types | `Result<_, String>` | `Result<_, PipeError>` |
| Rate limiting | `crate::rate_limit_sleep()` | `std::thread::sleep(Duration::from_millis(750))` |
| Schema namespace | `forge.*` | `github.*` |
| Auth delivery | Host-injected via capability grants | `pipe/initialize` params |
| Network access | Direct (via host proxy) | None — OS sandbox denies all sockets, Mother proxies HTTP |

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

### 3. pipe/http, Not reqwest

The connector does NOT bundle reqwest or open sockets directly. All
HTTP goes through Mother via `pipe/http` — the child calls
`io.get(url).send()` and Mother validates the domain against the
manifest allowlist, executes the call, and returns the response.

This is mandated by [[host-proxied-io-is-the-security-model]]: the OS
sandbox denies ALL outbound network. The child binary has zero TLS
dependencies. Mother already has `build_production_handler()` in
`src/broker/http.rs` that handles domain enforcement and credential
injection.

The Child trait is sync. Sequential paginated API calls. Each page
depends on the previous response (link headers). No async needed.

### 4. Cursor via Latest `updated_at`

The connector tracks the latest `updated_at` timestamp from fetched
items and returns it as the cursor in `FetchResult`. On the next fetch,
Mother passes this cursor back via `FetchParams.since`, and the
connector uses it as GitHub's `since` parameter (ISO 8601) to filter
by `updated_at`. This gives incremental fetching without a chrono
dependency.

Overlap between runs is handled by content-hash dedup in the broker
(`broker::routing::validate_fact()` + `broker::cursor::write_facts_with_cursor()`).

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

## Parity Verification Plan (AB Test)

Before deleting `src/forge/` (Step 6), run all three methods against
the same repo set. This is a **gating requirement** for deletion —
the historical rationale must be captured before the code disappears.

### Test Protocol

1. Pick 2-3 repos (document owner/repo, commit hash, timestamp)
2. Run all three methods:
   - Method 1: `patina scrape forge` (gh CLI → staging files)
   - Method 2: `patina mother run forge-wasm` (WASM plugin → direct events.db)
   - Method 3: `patina mother run github` (native child → broker → events.db)
3. Capture metrics: latency, request count, error rates, rate-limit hits
4. Normalize payloads before diffing (Method 2 uses `forge.*` namespace,
   translate to `github.*` schema for field-level comparison)

### Report Structure

- **Context/Goals:** sunsetting src/forge, last parity snapshot
- **Method Snapshots:** data flow, schema namespace, security model,
  maintenance state (to-be-deleted / legacy-frozen / future)
- **Metrics:** latency, request volume, error rates, rate-limit behavior
- **Data Drift Findings:** missing/mismatched fields with root causes.
  Note: Method 2 bypasses broker (no content-hash dedup, no cursors)
- **Recommendation:** delete src/forge once confidence is high

### Expected Differences

| Field | Method 1 (forge) | Method 2 (WASM) | Method 3 (native) |
|-------|------------------|-----------------|-------------------|
| event_type | forge.issue | forge.issue | github.issue |
| source_id | (staging files) | plugin:forge | child:github-connector |
| content_hash | (absent) | (absent) | blake3:... |
| Dedup | (none) | (none) | broker content-hash |
| Cursor | (none) | (none) | broker-managed |
| Data shape | Issue/PR structs | Same | Same |

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
children/github-connector/        # workspace member in Cargo.toml
  Cargo.toml              # patina-pipe, patina-pipe-types, serde, serde_json
  child.toml              # manifest for Mother
  src/
    main.rs               # Child trait impl + main()
    github.rs             # GitHub REST API client (migrated from forge)
```

No reqwest dependency — HTTP goes through pipe/http. No chrono — cursor
uses `updated_at` from fetched items. Minimal dependency footprint.

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
  binary. `cargo run`, `cargo test`, `dbg!()`. No WASM toolchain, no
  reqwest (pipe/http handles HTTP). Minimal dependency footprint.
- [[host-proxied-io-is-the-security-model]] — OS sandbox restricts
  the connector to `api.github.com` only. Credentials arrive via
  `pipe/initialize`, not environment variables.

## Resolved Questions

1. **~~chrono dependency~~** — Resolved: use latest `updated_at` from
   fetched items as cursor. No chrono needed. Dedup handles overlap.
   (Session 20260307-202447)

2. **~~reqwest vs pipe/http~~** — Resolved: connector uses `io.get(url).send()`
   via pipe/http. No reqwest, no direct sockets. Mother proxies all HTTP
   with domain enforcement. (Session 20260307-202447)

3. **~~Crate location~~** — Resolved: `children/github-connector/` as
   workspace member. Children isolated from core libraries.
   (Session 20260307-202447)

## Commits

1. `github-connector: create binary crate with Child trait impl` —
   children/github-connector/ with Cargo.toml, child.toml, main.rs.
   Add to workspace members.

2. `github-connector: migrate GitHub REST API client` — github.rs
   migrated from plugins/forge/src/github.rs. Replace host_http with
   io.get(url).send() (pipe/http), host_emit with io.emit(), errors
   with PipeError. No reqwest dependency.

3. `github-connector: add github.* schema definition` —
   .patina/schemas/github/schema.toml with fact types.

4. `github-connector: wire connection + source config` — Connection
   config, source entry. `patina mother run github` invokes connector.

5. `github-connector: AB parity verification` — Run all three methods
   against same repos, capture metrics and data shapes. Write report
   per AB Test Protocol above. Gate Step 6 on report.

6. `forge: delete src/forge/ (2,216 LOC) and scrape forge command
   (705 LOC)` — Remove old code. Keep plugins/forge/ (WASM).
   Only after AB report confirms parity.

## Key Files

- `children/github-connector/src/main.rs` — Child trait impl
- `children/github-connector/src/github.rs` — REST API client
- `children/github-connector/child.toml` — manifest for Mother
- `.patina/schemas/github/schema.toml` — schema definition
- `plugins/forge/src/github.rs` — migration source (450 LOC)
- `src/forge/` — deletion target (2,216 LOC)
- `src/commands/scrape/forge/mod.rs` — deletion target (705 LOC)
