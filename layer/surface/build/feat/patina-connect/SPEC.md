---
type: feat
id: patina-connect
status: active
created: 2026-03-06
sessions:
  origin: 20260306-171859
related:
- pipe-architecture
beliefs:
- safety-boundaries
- mother-is-connection-and-continuity
- wasm-host-boundary-hides-credentials
- defense-in-depth-over-perfect-isolation
exit_criteria:
- id: connection-domain-model
  text: 'ConnectionRecord type with identity metadata (provider, account_id, auth_method, scopes, timestamps, is_default) and durable auth metadata (injection_strategy, secret_refs, allowed_domains, refresh_capability, expiry_state) — replaces bare ConnectionConfig'
  checked: true
  verify: 'ConnectionRecord round-trips through TOML with all fields. Unit tests construct records with OAuth and manual auth methods. broker/connection.rs ConnectionConfig replaced by import from connect module.'
- id: auth-plan-resolution
  text: 'resolve_auth(record) → AuthPlan is the single seam between durable metadata and runtime credential use. AuthPlan contains resolved credential value, injection strategy, and allowed domains. Broker consumes AuthPlan without knowing credential origin. Auth-required children fail closed — resolve_auth returns error, broker never spawns unauthenticated.'
  checked: true
  verify: 'Unit tests: resolve_auth with Bearer strategy produces AuthPlan with token + Bearer injection. resolve_auth with missing vault entry returns typed error (not a warning). Broker run_source calls connect::load() then connect::resolve_auth(), never calls get_global_secret directly. grep for "proceeding without auth" in broker/ returns zero matches.'
- id: auth-strategy-dispatch
  text: 'Broker HTTP handler dispatches on AuthPlan injection strategy (Bearer, Header, InProcess) — not on provider identity. Replaces hardcoded Bearer in broker/http.rs:80. Broker has no import of plugin auth types.'
  checked: true
  verify: 'broker/http.rs build_production_handler takes AuthPlan (not raw credential tuple). Test: construct AuthPlan with Bearer strategy, verify Authorization header injected. Test: construct AuthPlan with no credential, verify no header injected. grep for "plugin::.*Credential\|plugin::.*Injection\|host_support" in src/broker/ returns zero matches.'
- id: provider-interface
  text: 'Provider trait defines acquisition interface — acquire(), probe_account(), default_scopes(), default_child(). GitHub is first implementation. Adding a second provider requires only a new Provider impl, not changes to connection or broker logic.'
  checked: true
  verify: 'GitHub provider implements trait. Test: mock provider with different scopes and child name populates ConnectionRecord correctly. No GitHub-specific code outside src/connect/providers/github.rs.'
- id: connection-paths
  text: 'paths::connections module in src/paths.rs — connections_dir(), connection_path(name). Replaces hardcoded path in broker/connection.rs:42-48.'
  checked: true
  verify: 'All connection path references go through paths::connections. grep for hardcoded .patina/connections in src/ returns zero matches outside paths.rs.'
- id: connection-lifecycle
  text: 'Create (OAuth or manual), load, list, remove (with referential integrity check against sources.toml across registered projects), refresh. Store writes connection TOML + vault entry atomically.'
  checked: true
  verify: 'Unit tests: create writes both TOML and vault entry. Remove of connection referenced by a sources.toml returns error naming the project. List returns all connections with computed status. Refresh updates vault entry and timestamps without losing identity metadata.'
- id: cli-surface
  text: '`patina connect` subcommands: <provider> (acquire), list, show <name>, status, refresh <name>, remove <name> — integrated into clap hierarchy as top-level command'
  checked: true
  verify: '`patina connect --help` shows all subcommands. `patina connect list` with zero connections prints empty table. `patina connect show nonexistent` returns actionable error. `patina connect remove` on referenced connection warns before proceeding.'
- id: architectural-cleanup
  text: 'Broker has no plugin-era auth dependencies. broker/http.rs does not import from plugin module. Shared HTTP utilities (validate_http_url, build_http_client, leak_check) extracted to a shared module — not copied into broker. broker/connection.rs deleted (replaced by connect module). broker/mod.rs does not call get_global_secret or load_connection directly. Stale CLI text in commands/mother/mod.rs updated to reflect destination routing.'
  checked: true
  verify: 'grep for "use crate::plugin" in src/broker/ returns zero matches. Shared HTTP utility module exists and is imported by both broker/http.rs and plugin/internal/host_support.rs — no duplicated leak_check or validate_http_url implementations. ls src/broker/connection.rs fails (file deleted). grep for "get_global_secret\|load_connection" in src/broker/mod.rs returns zero matches. `patina mother run --help` text does not claim "write to events.db" as only behavior.'
- id: end-to-end
  text: '`patina connect github` → `patina mother run <source>` works: OAuth device flow acquires token, stores in vault, creates connection record, broker resolves auth plan, child fetches data successfully'
  checked: true
  verify: 'Full flow: connect github, verify connection in list, run a source that references it, confirm facts written to events.db. Then: remove connection, verify source run fails with actionable error.'
---
# feat: patina-connect — Connection Subsystem

> patina-connect owns connection lifecycle and metadata.
> Mother remains the only component that wields provider credentials at runtime.
> The subsystem has three layers: acquisition (per-provider), persistence
> (connection domain model), and consumption (auth plan resolution for broker).

## Problem

Connecting Patina to an external data source requires expert knowledge
of three separate systems:

1. **Credential production** — create a PAT on provider's web UI
2. **Credential storage** — `patina secrets add` into the vault
3. **Connection wiring** — hand-author `~/.patina/connections/{name}.toml`
   with correct provider, credential reference, and child binding
4. **Source binding** — hand-author `.patina/sources.toml` referencing
   the connection by name

Each step requires understanding vault mechanics, connection config
format, and child naming conventions. Failure at any step produces
opaque errors at runtime (`credential not found`, `child not found`).

Beyond the setup pain, the architecture has drifted. The runtime path
for external services is now native child + broker + connection config,
but the code still reflects transitional shapes:

- `ConnectionConfig` (`broker/connection.rs:12`) is a 4-field TOML
  reader with no lifecycle state, no health, no auth strategy metadata.
- The broker collapses storage, domain, and runtime concerns into one
  function (`broker/mod.rs:36-57`): loads TOML, decrypts vault, builds
  credential tuple.
- `broker/http.rs:12-13` imports `CredentialMapping` and
  `InjectionLocation` from the plugin module — plugin-era auth types
  that belong to the WASM security boundary, not native-child connections.
- `broker/mod.rs:44-48` logs a warning and proceeds without auth when
  a credential is missing, even when the child manifest declares
  `auth.required = true` — fail-open on a fail-closed contract.
- `broker/http.rs:80-86` hardcodes Bearer injection — GitHub's model
  baked into the runtime.
- `host_support.rs:276` tells users to run `patina plugin grant`,
  a command that does not exist.

If this drift continues, patina-connect will become a thin CLI wrapper
around broker internals instead of a real connection subsystem.

## Solution

patina-connect is a product subsystem with three layers:

### Acquisition Layer (per-provider)

Provider-specific logic for obtaining credentials. This is where
all provider variation lives.

```
$ patina connect github
  Opening browser for GitHub authorization...
  Enter code: ABCD-1234

  Waiting for approval... approved!
  Connection "github" created.
  Done. GitHub data flows on next mother run.
```

Each provider implements a trait that defines how to acquire
credentials, what scopes to request, what child binary to bind,
and how to probe account identity. OAuth device flow for GitHub;
different flows for future providers.

Manual fallback for CI/headless:
```
$ patina connect github --manual
  Vault secret name: github-token
  Connection "github" created (manual auth).
```

### Persistence Layer (connection domain model)

The center of the subsystem. A `ConnectionRecord` stores two
kinds of durable metadata:

**Connection identity** (human-facing, stable):
- provider, account_id, auth_method, scopes, created/updated
  timestamps, is_default

**Durable auth configuration** (machine-facing, stable):
- injection_strategy (Bearer, Header, InProcess), secret
  reference(s), allowed_domains, refresh_capability, expiry_state

Storage owns TOML serialization. The domain layer owns validation,
lifecycle transitions, and auth resolution. Storage knows *how*
auth works (e.g. `strategy = "bearer"`). Storage never holds
decrypted credential values.

### Consumption Layer (auth plan resolution)

`resolve_auth(record)` is the single seam between durable metadata
and runtime. It decrypts the vault entry referenced by the connection
record and produces an `AuthPlan` — an execution-ready value that
the broker consumes without knowing the credential's origin.

```
connect::load(name)          → ConnectionRecord  (durable state)
connect::resolve_auth(record) → AuthPlan          (resolved credential + strategy)
broker uses AuthPlan          → HTTP proxy injects auth per strategy
```

The broker dispatches on auth strategy (Bearer? Header? InProcess?),
never on provider identity (GitHub? Slack?).

## Connection Management

```
patina connect <provider>         # Acquire credentials, create connection
patina connect list               # Show all connections with status
patina connect show <name>        # Detail view of one connection
patina connect status             # Health summary (connected/expired/missing)
patina connect refresh <name>     # Re-acquire credentials, update vault
patina connect remove <name>      # Delete connection + vault entry (checks refs)
```

## Key Design Boundaries

1. **patina-connect owns lifecycle. Mother owns credential use.**
   The connect module creates, stores, lists, refreshes, and removes
   connections. Mother (via broker) resolves auth plans and injects
   credentials at runtime. These are separate concerns.

2. **Auth strategy dispatch, not provider dispatch.**
   `broker/http.rs` asks "Bearer? Header? InProcess?" — never
   "GitHub? Slack?" The connection record carries enough durable
   auth metadata to drive injection without provider-specific code
   in the broker.

3. **Storage / Domain / Runtime separation.**
   Storage owns TOML serialization (durable, no secrets).
   Domain owns validation, lifecycle, and `resolve_auth()` (the only
   place that decrypts vault material for connection use).
   Runtime owns actual use of the resolved AuthPlan.

4. **Referential integrity on mutation.**
   Connections are referenced by `sources.toml` entries across
   projects. Remove/rename must scan registered projects before
   mutating global state.

5. **Fail closed on auth-required children.**
   If a child declares `auth.required = true` and `resolve_auth()`
   cannot produce a credential, the broker returns an error. No
   "proceeding without auth" warnings. The contract declared in
   `child.toml` is enforced, not advisory.

6. **No plugin-era auth in the broker.**
   The broker's credential path uses `AuthPlan` and
   `InjectionStrategy` from the connect module. It does not import
   `CredentialMapping`, `InjectionLocation`, or `host_support` from
   the plugin module. Those types serve the WASM plugin security
   boundary, which is a different concern.

## Key Files

**Build on:**
- `src/secrets/mod.rs` — vault, identity, session caching (reuse)
- `src/broker/mod.rs` — run_source (refactor to consume AuthPlan)
- `src/broker/http.rs` — HTTP proxy (refactor to dispatch on strategy)
- `src/broker/sources.rs` — SourceEntry references connections by name
- `src/paths.rs` — add connections path API

**Replace:**
- `src/broker/connection.rs` — thin ConnectionConfig → import from connect

**New:**
- `src/connect/mod.rs` — public API
- `src/connect/internal/` — model, store, provider trait, auth resolution
- `src/connect/providers/github.rs` — GitHub acquisition (OAuth device flow)
- `src/commands/connect.rs` — CLI subcommands

## Scope Decisions

**Connections are global (user-level) in v1.** All connections live at
`~/.patina/connections/`. The `ConnectionRecord` has a `scope` field
defaulting to `Global`. Project-local or persona-scoped connections
are structurally anticipated but not implemented.

**Secret resolution is global-only in v1.** `resolve_auth()` calls
`get_global_secret()`. The resolution path can be extended to check
project vaults without changing the AuthPlan interface.

## Non-Goals

- **Multiple providers in v1.** GitHub only. The provider trait exists
  so the second provider doesn't force a rewrite, but only GitHub
  ships in this spec.
- **Multi-account UX.** Named connections (`github:work`) are
  structurally supported. The UX for choosing between accounts is
  deferred.
- **Token refresh automation.** Manual `patina connect refresh` first.
  Mother-automated refresh is future work.
- **Credential rotation alerts.** Future work.
- **Replacing `patina secrets`.** Secrets remains for non-connection
  credentials (API keys, CI tokens). Connect is for provider
  connections that have identity, scopes, and lifecycle.
