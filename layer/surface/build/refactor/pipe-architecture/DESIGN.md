# Design: Pipe Architecture — Data Drivers, Not Plugins

## The Core Insight

Session 3 audit of forge-plugin-extraction revealed a category error.
We built a GitHub data driver and packaged it as a WASM plugin. But
plugins are for *opinions and behavior* — the spec lifecycle, grammars,
CLI extensions. Data drivers are *infrastructure* — they move data
from external sources into Patina's event system. Same relationship
as git (infrastructure, always present) vs the spec system (domain
logic, optionally installed).

The forge WASM plugin proved the plumbing works:
- host_emit → events.db with provenance and schema validation
- host_http → domain-allowlisted, credential-injected HTTP
- CQRS projection → events.db → patina.db materialized views
- FTS5 indexing from projection tables

What it got wrong:
- Packaged as per-project WASM plugin (should be user-level driver)
- Manual PAT creation (should be OAuth device flow)
- Per-plugin secret grants (should be shared credential store)
- No concept of multiple destinations sharing one pipe
- Plugin sandbox overhead on pure I/O (every HTTP call crosses boundary)

## Three Layers

### Layer 1: Secrets (`patina auth`)

**What exists today:**
- Age-encrypted vault (`~/.patina/vault.age`) with Keychain + Touch ID
- Global + project-scoped secrets
- Session caching via Mother child (10min TTL, avoids Touch ID spam)
- Secret scanning (pre-commit check for leaked secrets)
- CLI: `patina secrets add`, `remove`, `run`, `check`, `audit`
- Claude OAuth precedent: `patina secrets setup-claude` stores OAuth
  token, injects as env var on adapter launch

**What's missing:**
- `patina auth login <provider>` — OAuth device flow per source
- `patina auth status` — show all credentials, expiry, which pipes use them
- `patina auth refresh <provider>` — re-auth on expiry
- Credential metadata: scopes, created_at, expires_at, last_used_at
- Provider-specific OAuth registration (GitHub app, Slack app, etc.)

**Design approach:**
Build on existing secrets infrastructure. `patina auth` is a UX layer
on top of `patina secrets`. The vault stays the same. The addition is:
an OAuth device flow that acquires the token automatically (browser
popup, user approves, token stored) instead of manual PAT creation.

The `setup-claude` pattern is the precedent — it already stores an
OAuth token in the global vault and injects it on launch. Generalize
this to multiple providers:

```
patina auth login github
→ Opens browser to GitHub device authorization
→ User approves
→ Token stored: patina secrets add --global github:user
→ Metadata stored: scopes, expiry, provider type

patina auth login slack
→ Opens browser to Slack OAuth
→ Same flow, different provider
```

**Credential naming convention:**
`<provider>:<identity>` — e.g., `github:user`, `slack:workspace`,
`google:account@email.com`. Pipes reference these by name.

### Layer 2: Pipes (Mother-managed drivers)

**What a pipe is:**
A pipe is a data driver for one external source. It knows:
- How to authenticate (OAuth, API key, none for RSS)
- How to paginate (per_page+page, cursor, Link headers)
- How to rate limit (fixed delay, adaptive from headers)
- What data types it can provide (issues, PRs, channels, messages)
- How to filter and fetch on demand

A pipe does NOT know:
- Which project wants what data
- Where to store the data
- How to schedule itself
- Anything about Patina's internal data model

**Pipe interface (WIT-defined):**

```wit
interface pipe {
    /// What this pipe can provide.
    record capabilities {
        provider: string,           // "github", "slack", "rss"
        data-types: list<string>,   // ["issues", "prs", "comments"]
        supports-incremental: bool, // can fetch "since" timestamp?
        supports-streaming: bool,   // WebSocket/SSE available?
    }

    /// What a destination is asking for.
    record fetch-request {
        data-types: list<string>,   // which types to fetch
        filter: option<string>,     // provider-specific filter (JSON)
        since: option<string>,      // incremental: only after this timestamp
        limit: option<u32>,         // max items
    }

    /// A single fact emitted by the pipe.
    record fact {
        schema: string,             // "forge", "slack", etc.
        fact-type: string,          // "issue", "message", etc.
        data: string,               // JSON payload
    }

    /// Report what this pipe can do.
    get-capabilities: func() -> capabilities;

    /// Test connectivity with current credentials.
    check-health: func() -> result<string, string>;

    /// Fetch data matching the request. Returns facts to emit.
    fetch: func(request: fetch-request) -> result<list<fact>, string>;
}
```

The pipe returns facts. The host emits them (host_emit equivalent).
The pipe never touches events.db directly.

**WASM vs native:**
Pipes run in WASM for the same reason community plugins do — sandbox.
A community-published Slack pipe shouldn't be able to exfiltrate your
GitHub credentials. The WASM boundary ensures the pipe can only reach
its declared domains with its declared credentials.

The I/O overhead concern from the audit is real but minor:
- API latency: 100-500ms per call
- WASM boundary crossing: <1ms per call
- The boundary cost is noise compared to network latency

Core/first-party pipes (GitHub, built by Patina) could theoretically
run native for slightly less overhead, but the uniformity of "all
pipes are WASM" is worth more than the marginal performance gain.

**Pipe packaging:**
Same as today's plugins — a directory with `plugin.toml` (or
`pipe.toml`) and a `.wasm` binary. But installed at user level
(`~/.patina/pipes/`) not project level.

### Layer 3: Destinations (project, lake, block configuration)

**What a destination is:**
A destination is a consumer of pipe data. It specifies:
- Which pipe(s) to use
- Which credential to authenticate with
- What data to fetch (types, filters, repos/channels/feeds)
- When to fetch (schedule or trigger)

**Destination types:**

| Type | Scope | Example |
|---|---|---|
| Project | `.patina/` in a git repo | "issues + PRs from this repo" |
| Data lake | `~/.patina/lakes/<name>/` | "all repos from org X + org Y" |
| Data block | `~/.patina/blocks/<name>/` | "security-labeled PRs from repo Z" |

**Configuration format (project-level example):**

```toml
# .patina/sources.toml (or in patina.toml)

[sources.github]
pipe = "github"
auth = "github:user"
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]
schedule = "on-scrape"

[sources.slack]
pipe = "slack"
auth = "slack:myworkspace"
params = { channels = ["#dev", "#incidents"] }
types = ["messages"]
schedule = "hourly"
```

**Mother's role:**
Mother manages pipe scheduling. For each destination:
1. Read source configuration
2. Resolve credential from `patina auth`
3. Load pipe (WASM)
4. Call `fetch(request)` with destination's filter
5. Emit returned facts to destination's events.db
6. Track last-sync timestamp for incremental

This replaces the current `patina plugin run patina-forge -- sync`
manual invocation.

## Migration Path from forge-plugin-extraction

The forge WASM plugin code is the starting point for the GitHub pipe.
What changes:

| Aspect | Current (WASM plugin) | Target (pipe) |
|---|---|---|
| Interface | `handle("sync", payload)` | `fetch(request)` |
| Auth | Manual PAT + vault + grants TOML | `patina auth login github` |
| Scope | Per-project installation | User-level, shared |
| Config | JSON payload at runtime | TOML in destination config |
| Scheduling | Manual `plugin run` | Mother-managed per destination |
| Facts | Plugin calls host_emit directly | Pipe returns facts, host emits |
| Schema | Declared in plugin manifest | Ships with pipe, same mechanism |

The internal code (GitHub REST client, JSON parsing, rate limiting)
migrates almost unchanged. The wrapper changes from MotherChildPlugin
to a Pipe interface.

## Secrets Architecture Detail

**Credential lifecycle:**

```
patina auth login github
  → Register GitHub OAuth App (one-time, or use Patina's app ID)
  → Device authorization flow:
    1. POST https://github.com/login/device/code
    2. Display: "Go to github.com/login/device, enter code: XXXX-YYYY"
    3. Poll: POST https://github.com/login/oauth/access_token
    4. Receive token
  → Store: patina secrets add --global github:user <token>
  → Store metadata: provider=github, scopes=[repo], expires=never,
    created=2026-03-06, last_used=null
```

**Credential resolution for pipes:**

```
Mother loads source config → auth = "github:user"
  → Resolve: patina secrets get --global github:user
  → Decrypt via Keychain/Touch ID (or session cache)
  → Inject into pipe's WASM host state (same as today's host_http)
  → Pipe calls fetch → host makes HTTP with Bearer token
```

No secret-grants.toml. No manual PAT. The trust model shifts from
"user grants plugin access to a secret" to "user authenticates with
a provider, Mother injects credentials into configured pipes."

**Security model comparison:**

| | Current (plugin grants) | Target (pipe auth) |
|---|---|---|
| Who creates credential | User (manual PAT) | `patina auth` (OAuth) |
| Who stores it | User (vault add) | `patina auth` (vault add) |
| Who grants access | User (secret-grants.toml) | Source config (auth = "github:user") |
| Enforcement | host_http call-time check | Same — pipe runs in WASM, host injects |
| Revocation | Delete from vault | `patina auth revoke github` |

The WASM sandbox still prevents pipes from accessing credentials they
weren't configured with. The difference is UX: one command vs four steps.

## Relationship to Other Specs

- **forge-plugin-extraction** — proved the pattern. Pipe architecture
  supersedes the "plugin" framing but keeps the infrastructure (host_emit,
  projection, schema validation).
- **lake-registry** — becomes the "data lake destination" type. Lake
  metadata in graph.db, lake sources configured as pipe destinations.
- **core-extraction** — pipes are NOT core. They're user-level drivers
  managed by Mother. Core is protocol + stores.
- **continuous-operation** — Mother daemon manages pipe scheduling.
  Pipe health checks feed into Mother's health monitoring.
- **scrape-simplification** — scrape stays local (git). Pipe data
  arrives via Mother, not via scrape dispatch. `patina scrape` triggers
  projection of pipe-emitted events, not pipe execution.

## Key Files (current, to be refactored)

**Pipe code (from forge plugin):**
- `plugins/forge/src/lib.rs` — ForgeChild, becomes pipe interface impl
- `plugins/forge/src/github.rs` — GitHubClient, migrates to github-pipe
- `plugins/forge/plugin.toml` — becomes pipe.toml

**Host infrastructure (stays, shared by pipes and plugins):**
- `src/plugin/internal/host_support.rs` — emit_fact, http_get, leak_check
- `src/plugin/internal/mother_child.rs` — WASM runtime, may need pipe world
- `src/secrets/mod.rs` — vault, identity, session caching

**Projection (stays, source-agnostic):**
- `src/commands/scrape/forge/mod.rs` — project_from_events, FTS5 population

**New code needed:**
- `src/auth/` — OAuth device flow, provider registry, `patina auth` CLI
- `src/pipe/` — pipe interface, pipe loading, fetch→emit bridge
- WIT: `wit/pipe/pipe.wit` — pipe interface definition

## Open Questions

1. **Pipe discovery.** How do users find and install community pipes?
   Registry like crates.io? GitHub releases? Manual download?

2. **Schema ownership.** Today the forge schema ships with the plugin.
   With pipes, does the schema ship with the pipe or is it installed
   independently? Lean toward: ships with the pipe (same as today).

3. **Pipe versioning.** When a pipe's output format changes, how do
   projection tables migrate? Schema version in event metadata?

4. **Multi-provider pipes.** Is "github-pipe" one pipe for all GitHub
   instances, or is "github-enterprise" a separate pipe? Lean toward:
   one pipe, configured with base_url per destination.

5. **Pipe testing.** `patina pipe test github` — run a health check
   and small fetch to verify the pipe works with current credentials.

6. **The streaming question.** Poll-based pipes (GitHub, RSS) work today.
   Push-based pipes (Slack real-time, WebSocket feeds) need Mother to hold
   connections and buffer events. Design needed but not a blocker for
   initial pipe architecture.
